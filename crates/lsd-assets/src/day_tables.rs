use crate::{PsxExeHeader, is_canonical_reference, load_scene_grid};
use lsd_core::{DayTables, Mood};

/// Original day/mood content is read from the canonical local executable only.
pub fn load_day_tables(executable: &[u8]) -> Result<DayTables, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Day tables require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let bytes = |address: u32, size: usize| -> Result<&[u8], &'static str> {
        let start = address
            .checked_sub(header.load_address)
            .ok_or("Day pointer below executable")? as usize
            + 0x800;
        executable
            .get(start..start.checked_add(size).ok_or("Day extent overflow")?)
            .ok_or("Day pointer outside executable")
    };
    let word = |address| -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(bytes(address, 4)?.try_into().unwrap()))
    };
    let grids: Vec<_> = (0..14)
        .map(|stage| load_scene_grid(executable, stage))
        .collect::<Result<_, _>>()?;
    let grids: [_; 14] = grids.try_into().unwrap();
    let mut moods = std::array::from_fn(|_| Vec::new());
    for stage in 0..14 {
        let address = word(0x8008_6590 + stage as u32 * 4)?;
        let count = (i32::from(grids[stage].columns) * i32::from(grids[stage].rows)) as usize;
        moods[stage] = bytes(address, count * 2)?
            .chunks_exact(2)
            .map(|value| Mood([value[0] as i8, value[1] as i8]))
            .collect();
    }
    let shorts = |address: u32, count: usize| -> Result<Vec<i16>, &'static str> {
        Ok(bytes(address, count * 2)?
            .chunks_exact(2)
            .map(|value| i16::from_le_bytes(value.try_into().unwrap()))
            .collect())
    };
    let marker = bytes(0x8008_ABF4, 2)?;
    Ok(DayTables {
        grids,
        moods,
        special_days: shorts(0x8008_8CCC, 42)?.try_into().unwrap(),
        special_marker: Mood([marker[0] as i8, marker[1] as i8]),
        colors: bytes(0x8008_7E14, 9)?
            .iter()
            .map(|&value| value as i8)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        time_limits: shorts(0x8008_7F14, 14)?.try_into().unwrap(),
        save_magic: word(0x8008_ABE0)?,
        start_rotation: [word(0x8008_7E08)?, word(0x8008_7E0C)?, word(0x8008_7E10)?],
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn unrecognized_executable_does_not_supply_original_day_content() {
        assert!(super::load_day_tables(&[]).is_err());
    }
}
