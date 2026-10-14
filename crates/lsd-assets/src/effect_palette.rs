use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::EffectPalette;

/// Read original effect colors only from the identified local executable.
pub fn load_effect_palette(executable: &[u8]) -> Result<EffectPalette, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Effect palettes require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let read = |address: u32| {
        let base = (address - header.load_address) as usize + 0x800;
        std::array::from_fn(|index| {
            executable[base + index * 3..base + index * 3 + 3]
                .try_into()
                .unwrap()
        })
    };
    Ok(EffectPalette {
        start: read(0x8006_EA90),
        attached: read(0x8006_EAA8),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_synthetic_executable_is_not_accepted_as_an_original_palette() {
        assert_eq!(
            super::load_effect_palette(&[]),
            Err("Effect palettes require canonical SLPS_015.56")
        );
    }
}
