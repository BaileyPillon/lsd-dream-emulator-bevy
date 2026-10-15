use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::SpuPitchTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuReverbPreset {
    pub start_address: u16,
    pub registers: [u16; 32],
}

impl SpuReverbPreset {
    /// Register values for a mode-only selection with zero depth. These do not
    /// implement SDK allocation checks, work-area clearing or callback timing.
    pub fn register_writes(&self) -> Vec<lsd_core::AudioRegisterWrite> {
        let mut writes = vec![
            lsd_core::AudioRegisterWrite {
                offset: 0x184,
                value: 0,
            },
            lsd_core::AudioRegisterWrite {
                offset: 0x186,
                value: 0,
            },
        ];
        writes.extend(
            self.registers
                .into_iter()
                .enumerate()
                .map(|(index, value)| lsd_core::AudioRegisterWrite {
                    offset: 0x1c0 + index as u16 * 2,
                    value,
                }),
        );
        writes.push(lsd_core::AudioRegisterWrite {
            offset: 0x1a2,
            value: self.start_address,
        });
        writes
    }
}

pub fn load_spu_reverb_presets(executable: &[u8]) -> Result<[SpuReverbPreset; 10], &'static str> {
    if !is_canonical_reference(executable) {
        return Err("SPU reverb presets require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let offset = |address: u32| (address - header.load_address) as usize + 0x800;
    let starts = executable
        .get(offset(0x8006_e194)..offset(0x8006_e194) + 40)
        .ok_or("SPU reverb starts outside executable")?;
    let presets = executable
        .get(offset(0x8006_e1e4)..offset(0x8006_e1e4) + 680)
        .ok_or("SPU reverb presets outside executable")?;
    if starts
        .chunks_exact(4)
        .any(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()) > 65535)
    {
        return Err("SPU reverb start exceeds register width");
    }
    Ok(std::array::from_fn(|mode| SpuReverbPreset {
        start_address: u32::from_le_bytes(starts[mode * 4..mode * 4 + 4].try_into().unwrap())
            as u16,
        registers: std::array::from_fn(|index| {
            let at = mode * 68 + 4 + index * 2;
            u16::from_le_bytes([presets[at], presets[at + 1]])
        }),
    }))
}

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
