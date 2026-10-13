use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::PsxMathTables;

pub fn load_math_tables(executable: &[u8]) -> Result<PsxMathTables, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Math tables require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let offset = |address: u32| (address - header.load_address) as usize + 0x800;
    let sin_cos = Box::new(std::array::from_fn(|index| {
        let at = offset(0x8006_6BE0) + index * 4;
        u32::from_le_bytes(executable[at..at + 4].try_into().unwrap())
    }));
    let sqrt_window = Box::new(std::array::from_fn(|index| {
        let at = offset(0x8006_BEF0 - 640) + index * 2;
        i16::from_le_bytes(executable[at..at + 2].try_into().unwrap())
    }));
    Ok(PsxMathTables {
        sin_cos,
        sqrt_window,
    })
}
