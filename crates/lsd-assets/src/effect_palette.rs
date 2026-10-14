use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::EffectPalette;
pub fn load_entity_effect_definition(
    executable: &[u8],
) -> Result<lsd_core::EntityEffectDefinition, &'static str> {
    let palette = load_effect_palette(executable)?;
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let words = |address: u32| {
        let base = (address - header.load_address) as usize + 0x800;
        std::array::from_fn::<_, 2, _>(|i| {
            i32::from_le_bytes(
                executable[base + i * 4..base + i * 4 + 4]
                    .try_into()
                    .unwrap(),
            )
        })
    };
    Ok(lsd_core::EntityEffectDefinition {
        palette,
        size: words(0x8008_ac14).map(|v| v as u16),
        offset: words(0x8008_ac0c),
    })
}

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
