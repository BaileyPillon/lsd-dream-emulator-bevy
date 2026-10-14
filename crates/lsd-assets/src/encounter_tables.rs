use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::{EncounterChunk, EncounterObject, EncounterTables, EncounterVariant};

pub fn load_encounter_tables(executable: &[u8]) -> Result<EncounterTables, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Encounter tables require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let bytes = |address: u32, count: usize| -> Result<&[u8], &'static str> {
        let offset = address
            .checked_sub(header.load_address)
            .ok_or("Encounter pointer below executable")? as usize
            + 0x800;
        executable
            .get(
                offset
                    ..offset
                        .checked_add(count)
                        .ok_or("Encounter extent overflow")?,
            )
            .ok_or("Encounter pointer outside executable")
    };
    let word = |address: u32| -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(bytes(address, 4)?.try_into().unwrap()))
    };
    let mut chunks = std::array::from_fn(|_| Vec::new());
    let mut objects = std::array::from_fn(|_| Vec::new());
    let mut variants_needed = 0;
    for stage in 0..14 {
        let count = bytes(0x8008_9ac4 + stage as u32, 1)?[0] as i8;
        if count < 0 {
            return Err("Negative encounter chunk count");
        }
        let pointer = word(0x8008_9a8c + stage as u32 * 4)?;
        if count > 0 {
            for record in bytes(pointer, count as usize * 6)?.chunks_exact(6) {
                chunks[stage].push(EncounterChunk {
                    chunk: i16::from_le_bytes(record[..2].try_into().unwrap()),
                    day_gate: record[2] as i8,
                    objects: record[3..6]
                        .iter()
                        .map(|&v| v as i8)
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                });
            }
        }
        let count = bytes(0x8008_9a7c + stage as u32, 1)?[0] as i8;
        if count < 0 {
            return Err("Negative encounter object count");
        }
        let pointer = word(0x8008_9a44 + stage as u32 * 4)?;
        if count > 0 {
            for record in bytes(pointer, count as usize * 8)?.chunks_exact(8) {
                let variants: [i8; 4] = record[4..8]
                    .iter()
                    .map(|&v| v as i8)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                for v in variants.into_iter().take_while(|&v| v != -1) {
                    variants_needed = variants_needed.max(usize::from(v as u8) + 1);
                }
                objects[stage].push(EncounterObject {
                    used: record[0] as i8,
                    gate: record[1] as i8,
                    model: record[2] as i8,
                    entity: record[3],
                    variants,
                });
            }
        }
    }
    let variants = bytes(0x8008_8f48, variants_needed * 4)?
        .chunks_exact(4)
        .map(|v| EncounterVariant {
            tile: i16::from_le_bytes(v[..2].try_into().unwrap()),
            rotation: v[2] as i8,
            position: v[3] as i8,
        })
        .collect::<Vec<_>>();
    let positions_needed = variants
        .iter()
        .map(|v| {
            usize::try_from(v.position)
                .map(|v| v + 1)
                .map_err(|_| "Negative encounter position index")
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let rotations_needed = variants
        .iter()
        .map(|v| {
            usize::try_from(v.rotation)
                .map(|v| v + 1)
                .map_err(|_| "Negative encounter rotation index")
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let positions = bytes(0x8008_8d3c, positions_needed * 6)?
        .chunks_exact(6)
        .map(|v| {
            std::array::from_fn(|i| i16::from_le_bytes(v[i * 2..i * 2 + 2].try_into().unwrap()))
        })
        .collect();
    let rotations = bytes(0x8008_8f18, rotations_needed * 12)?
        .chunks_exact(12)
        .map(|v| {
            std::array::from_fn(|i| u32::from_le_bytes(v[i * 4..i * 4 + 4].try_into().unwrap()))
        })
        .collect();
    let colors = bytes(0x8008_8ccc + 0x4a, 129)?
        .iter()
        .map(|&v| v as i8)
        .collect();
    Ok(EncounterTables {
        chunks,
        objects,
        variants,
        positions,
        rotations,
        color_lookup: colors,
    })
}
