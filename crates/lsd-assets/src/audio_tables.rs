use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::SpuPitchTable;

pub fn load_spu_pitch_table(executable: &[u8]) -> Result<SpuPitchTable, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("SPU pitch table requires canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let offset = (0x8006_dad8 - header.load_address) as usize + 0x800;
    let bytes = executable
        .get(offset..offset + 384)
        .ok_or("SPU pitch table outside executable")?;
    Ok(SpuPitchTable {
        values: Box::new(std::array::from_fn(|i| {
            u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]])
        })),
    })
}
