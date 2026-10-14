use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::{GenerationActorEntry, GenerationEntry, GenerationTables};

/// Read generation content from the user's canonical executable. No game tables
/// are embedded in this crate or distributed with the port.
pub fn load_generation_tables(executable: &[u8]) -> Result<GenerationTables, &'static str> {
    let reader = Reader::new(executable)?;
    let mut entries = std::array::from_fn(|_| Vec::new());
    for (i, table) in entries.iter_mut().enumerate() {
        let count = reader.bytes(0x8008_73D8 + i as u32, 1)?[0] as usize;
        let pointer = reader.word(0x8008_73C8 + i as u32 * 4)?;
        *table = reader
            .bytes(pointer, count * 4)?
            .chunks_exact(4)
            .map(|v| GenerationEntry::from(v.map_array()))
            .collect();
    }
    let fixed = (0..14)
        .map(|i| {
            let pointer = reader.word(0x8008_73EC + i * 4)?;
            if pointer == 0 {
                Ok(None)
            } else {
                Ok(Some(GenerationEntry::from(
                    reader.bytes(pointer, 4)?.map_array(),
                )))
            }
        })
        .collect::<Result<Vec<_>, &'static str>>()?
        .try_into()
        .unwrap();
    let mut actor_entries = std::array::from_fn(|_| Vec::new());
    for (i, table) in actor_entries.iter_mut().enumerate() {
        let count = reader.bytes(0x8008_76EC + i as u32, 1)?[0] as usize;
        if count == 0 {
            continue;
        }
        let pointer = reader.word(0x8008_76B4 + i as u32 * 4)?;
        *table = reader
            .bytes(pointer, count * 8)?
            .chunks_exact(8)
            .map(|v| GenerationActorEntry {
                key: v[..4].map_array(),
                style: v[4],
                effect: v[6] as i8,
                reserved: [v[5], v[7]],
            })
            .collect();
    }
    let style_count = actor_entries
        .iter()
        .flatten()
        .map(|v| usize::from(v.style))
        .max()
        .unwrap_or(0)
        + 1;
    // find_entry deliberately addresses styles from effects + 0x3c, rather
    // than from the adjacent entry data. Preserve this original alias.
    let styles = reader
        .bytes(0x8008_74EC, style_count * 6)?
        .chunks_exact(6)
        .map(|v| {
            std::array::from_fn(|i| i16::from_le_bytes(v[i * 2..i * 2 + 2].try_into().unwrap()))
        })
        .collect();
    Ok(GenerationTables {
        entries,
        cycle: reader.bytes(0x8008_73DC, 16)?.map_array(),
        fixed,
        palette: reader.colors(0x8008_72C4, 24)?,
        heights: reader.words(0x8008_730C)?,
        // Random structure placement uses % 5 on a four-word table. Its fifth
        // result is the first word of the adjacent entry table, as on PS1.
        sample_offsets: reader.words(0x8008_7328)?,
        rotations: reader.packets(0x8008_7174)?,
        scales: reader.packets(0x8008_71C8)?,
        fixed_scale: reader.words(0x8008_7204)?,
        colors_a: reader.colors(0x8008_721C, 3)?.try_into().unwrap(),
        colors_b: reader.colors(0x8008_7228, 3)?.try_into().unwrap(),
        effect_palettes: [
            reader.colors(0x8008_7234, 18)?.try_into().unwrap(),
            reader.colors(0x8008_726C, 18)?.try_into().unwrap(),
        ],
        entity_ranges: reader.words(0x8008_7474)?,
        actor_entries,
        styles,
        alternate_parameter: reader.word(0x8008_7430)? as i32,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationPaths {
    /// Paths relative to CDI, with normalized separators.
    pub textures: [[String; 4]; 14],
    pub music: [[String; 5]; 14],
}

pub fn load_generation_paths(executable: &[u8]) -> Result<GenerationPaths, &'static str> {
    let reader = Reader::new(executable)?;
    let mut textures = std::array::from_fn(|_| std::array::from_fn(|_| String::new()));
    let mut music = std::array::from_fn(|_| std::array::from_fn(|_| String::new()));
    for stage in 0..14 {
        let record = reader.bytes(0x8008_19E8 + stage as u32 * 2, 2)?;
        let start = u16::from_le_bytes(record.try_into().unwrap()) as u32;
        for (index, value) in textures[stage].iter_mut().enumerate() {
            *value = reader.path(0x8008_1A04 + (start + index as u32) * 28)?;
        }
        for (index, value) in music[stage].iter_mut().enumerate() {
            *value = reader.path(0x8008_1A04 + (start + 4 + index as u32) * 28)?;
        }
    }
    Ok(GenerationPaths { textures, music })
}

struct Reader<'a> {
    executable: &'a [u8],
    load: u32,
}
impl<'a> Reader<'a> {
    fn new(executable: &'a [u8]) -> Result<Self, &'static str> {
        if !is_canonical_reference(executable) {
            return Err("Generation content requires canonical SLPS_015.56");
        }
        Ok(Self {
            executable,
            load: PsxExeHeader::parse(executable)
                .map_err(|_| "Invalid executable extent")?
                .load_address,
        })
    }
    fn bytes(&self, address: u32, length: usize) -> Result<&'a [u8], &'static str> {
        let start = address
            .checked_sub(self.load)
            .ok_or("Generation pointer below executable")? as usize;
        let start = start
            .checked_add(0x800)
            .ok_or("Generation offset overflow")?;
        self.executable
            .get(
                start
                    ..start
                        .checked_add(length)
                        .ok_or("Generation extent overflow")?,
            )
            .ok_or("Generation pointer outside executable")
    }
    fn word(&self, address: u32) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            self.bytes(address, 4)?.try_into().unwrap(),
        ))
    }
    fn words<T: FromWord, const N: usize>(&self, address: u32) -> Result<[T; N], &'static str> {
        let bytes = self.bytes(address, N * 4)?;
        Ok(std::array::from_fn(|i| {
            T::from_word(u32::from_le_bytes(
                bytes[i * 4..i * 4 + 4].try_into().unwrap(),
            ))
        }))
    }
    fn packets<const N: usize>(&self, address: u32) -> Result<[[u32; 3]; N], &'static str> {
        (0..N)
            .map(|i| self.words(address + i as u32 * 12))
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.try_into().unwrap())
    }
    fn colors(&self, address: u32, count: usize) -> Result<Vec<[u8; 3]>, &'static str> {
        Ok(self
            .bytes(address, count * 3)?
            .chunks_exact(3)
            .map(|v| v.try_into().unwrap())
            .collect())
    }
    fn path(&self, address: u32) -> Result<String, &'static str> {
        let bytes = self.bytes(address, 28)?;
        let end = bytes
            .iter()
            .position(|&v| v == 0)
            .ok_or("Generation path has no terminator")?;
        let path = std::str::from_utf8(&bytes[..end])
            .map_err(|_| "Generation path is not UTF-8")?
            .replace('\\', "/");
        if path.is_empty()
            || path.starts_with('/')
            || path.contains(':')
            || path.split('/').any(|v| v == "..")
        {
            return Err("Invalid relative generation path");
        }
        Ok(path)
    }
}
trait FromWord {
    fn from_word(word: u32) -> Self;
}
impl FromWord for i32 {
    fn from_word(word: u32) -> Self {
        word as i32
    }
}
impl FromWord for u32 {
    fn from_word(word: u32) -> Self {
        word
    }
}
trait ByteArray {
    fn map_array<const N: usize>(&self) -> [i8; N];
}
impl ByteArray for [u8] {
    fn map_array<const N: usize>(&self) -> [i8; N] {
        assert_eq!(self.len(), N);
        std::array::from_fn(|i| self[i] as i8)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn foreign_executables_cannot_supply_original_generation_content() {
        assert!(super::load_generation_tables(&[]).is_err());
        assert!(super::load_generation_paths(&[]).is_err());
    }
}
