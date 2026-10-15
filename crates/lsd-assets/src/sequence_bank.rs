use crate::formats::{DecodeError, Vab};
use lsd_core::{SequenceProgramDefinition, SequenceVolumeDefinition};

/// SDK sequence programs retain all 128 MIDI IDs, including empty programs.
/// This differs from the sound object's logical program-count table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceBankBinding {
    programs: Vec<SequenceProgramDefinition>,
    volume: SequenceVolumeDefinition,
}

impl SequenceBankBinding {
    pub fn from_vab(vab: &Vab<'_>, bank: i16, spu_base: u32) -> Result<Self, DecodeError> {
        if !(0..16).contains(&bank) {
            return Err(DecodeError::new(0, "invalid SDK bank index"));
        }
        let mut programs = Vec::with_capacity(128);
        let mut packed = 0_u8;
        for id in 0..128_u8 {
            let attributes = vab.program_attributes[usize::from(id)];
            let mut tones = Vec::new();
            if let Some(program) = vab.program(id) {
                for tone in 0..program.tones.len() {
                    tones.push(vab.prepare_voice_note(bank, id, tone, spu_base)?);
                }
            }
            programs.push(SequenceProgramDefinition {
                bank,
                program: i16::from(id),
                packed_program: packed,
                packed_program_count: vab.header.program_count,
                bank_volume: vab.header.volume,
                program_volume: attributes[1],
                program_pan: attributes[4],
                tones,
            });
            if attributes[0] != 0 {
                packed = packed.wrapping_add(1);
            }
        }
        let volume = SequenceVolumeDefinition {
            bank,
            bank_volume: vab.header.volume,
            programs: vab.program_attributes.clone(),
            first_program_tones: vab
                .programs
                .first()
                .map(|p| p.tone_attributes.clone())
                .unwrap_or_else(|| Box::new([[0; 32]; 16])),
        };
        Ok(Self { programs, volume })
    }

    pub fn program(&self, id: u8) -> Option<&SequenceProgramDefinition> {
        self.programs.get(usize::from(id))
    }

    pub fn volume_definition(&self) -> &SequenceVolumeDefinition {
        &self.volume
    }

    /// Expression is a shared bank program-volume write, not a separate
    /// per-channel gain. All sequence tracks using that program observe it.
    pub fn set_program_volume(&mut self, id: u8, volume: u8) -> Result<(), DecodeError> {
        let program = self
            .programs
            .get_mut(usize::from(id))
            .ok_or_else(|| DecodeError::new(0, "invalid SDK program"))?;
        program.program_volume = volume;
        for tone in &mut program.tones {
            tone.program_volume = volume;
        }
        self.volume.programs[usize::from(id)][1] = volume;
        Ok(())
    }
}
