use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::SceneScaleTables;

pub fn load_scene_scale_tables(executable: &[u8]) -> Result<SceneScaleTables, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Scene scale requires canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let offset = (0x8008_699c - header.load_address) as usize + 0x800;
    let bytes = executable
        .get(offset..offset + 60)
        .ok_or("Scene scale packets outside executable")?;
    Ok(SceneScaleTables {
        packets: std::array::from_fn(|row| {
            std::array::from_fn(|axis| {
                let start = row * 12 + axis * 4;
                u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap())
            })
        }),
    })
}
