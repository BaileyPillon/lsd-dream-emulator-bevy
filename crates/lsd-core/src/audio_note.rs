use crate::{AudioVoicePool, SpuPitchFault, SpuPitchTable};

/// A tone bound to an assigned SDK bank and a region of SPU RAM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceNoteDefinition {
    pub bank: i16,
    pub program: i16,
    pub packed_program: u8,
    pub tone: i16,
    pub bank_volume: u8,
    pub program_volume: u8,
    pub program_pan: u8,
    pub attributes: [u8; 32],
    pub sample_address: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteSetupFault {
    InvalidBinding,
    StereoDivisionByZero,
    Pitch(SpuPitchFault),
    SpecialSample255,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteStart {
    pub voice: Option<u8>,
    pub immediate_key_off: Option<u32>,
}

impl NoteStart {
    pub fn handle(self) -> i32 {
        self.voice.map(i32::from).unwrap_or(-1)
    }
}

/// Original SDK sound-effect note setup. The register cache and command masks
/// are software state; starting a note here does not synthesize or play audio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioNoteDriver {
    pub pool: AudioVoicePool,
    pub registers: [[u16; 8]; 24],
    pub dirty: [u8; 24],
    pub automation: [u32; 16],
    pub reverb: u32,
    pub mono: i16,
    pub adsr_release_offset: u16,
}

impl AudioNoteDriver {
    pub fn new(voices: i32) -> Self {
        Self {
            pool: AudioVoicePool::new(voices),
            registers: [[0; 8]; 24],
            dirty: [0; 24],
            automation: [0; 16],
            reverb: 0,
            mono: 0,
            adsr_release_offset: 0,
        }
    }

    pub fn key_on(
        &mut self,
        definition: &VoiceNoteDefinition,
        pitch_table: &SpuPitchTable,
        note: i32,
        fine: i32,
        left: i32,
        right: i32,
    ) -> Result<NoteStart, NoteSetupFault> {
        let failed = NoteStart {
            voice: None,
            immediate_key_off: None,
        };
        if self.pool.lock == 1 {
            return Ok(failed);
        }
        if !(0..16).contains(&definition.bank)
            || !(0..128).contains(&definition.program)
            || !(0..16).contains(&definition.tone)
        {
            return Err(NoteSetupFault::InvalidBinding);
        }
        let sample = u16::from_le_bytes([definition.attributes[22], definition.attributes[23]]);
        if sample == 0 {
            self.pool.lock = 0;
            return Ok(failed);
        }
        if sample == 255 {
            return Err(NoteSetupFault::SpecialSample255);
        }
        let (volume, pan) = stereo_request(left, right)?;
        let pitch = pitch_table
            .pitch(
                i32::from(note as u16),
                i32::from(fine as u16),
                definition.attributes[4],
                definition.attributes[5],
            )
            .map_err(NoteSetupFault::Pitch)?;
        self.pool.lock = 1;
        let allocation = self.pool.allocate(definition.attributes[0]);
        if allocation.index >= self.pool.capacity() {
            self.pool.lock = 0;
            return Ok(failed);
        }
        let voice = usize::from(allocation.index);
        self.pool.current = u16::from(allocation.index);
        let slot = &mut self.pool.slots[voice];
        slot.set_word(14, 0x21);
        slot.set_word(22, definition.bank as u16);
        slot.set_word(18, definition.program as u16);
        slot.set_word(16, u16::from(definition.packed_program));
        slot.set_word(0, sample);
        slot.set_word(12, note as u16);
        slot.record[27] = 1;
        slot.set_word(2, 0);
        slot.set_word(20, definition.tone as u16);
        slot.set_word(6, 0x7fff);
        let bit = 1_u32 << allocation.index;
        for mask in &mut self.automation {
            *mask &= !bit;
        }
        let registers = &mut self.registers[voice];
        registers[3] = definition.sample_address;
        registers[4] = u16::from_le_bytes([definition.attributes[16], definition.attributes[17]]);
        registers[5] = u16::from_le_bytes([definition.attributes[18], definition.attributes[19]])
            .wrapping_add(self.adsr_release_offset);
        let mut amplitude = u32::from(definition.bank_volume) * 0x3fff * u32::from(volume) / 16129;
        amplitude = amplitude
            .wrapping_mul(u32::from(definition.program_volume))
            .wrapping_mul(u32::from(definition.attributes[2]))
            / 16129;
        let mut levels = [amplitude; 2];
        for pan in [definition.attributes[3], definition.program_pan, pan] {
            if pan < 64 {
                levels[1] = levels[1].wrapping_mul(u32::from(pan)) / 63;
            } else {
                levels[0] = levels[0].wrapping_mul(127_u32.wrapping_sub(u32::from(pan))) / 63;
            }
        }
        if self.mono == 1 {
            levels = [levels[0].max(levels[1]); 2];
        }
        registers[0] = (levels[0].wrapping_mul(levels[0]) / 16383) as u16;
        registers[1] = (levels[1].wrapping_mul(levels[1]) / 16383) as u16;
        registers[2] = pitch;
        self.dirty[voice] |= 0x3f;
        slot.set_word(4, pitch);
        if definition.attributes[1] & 4 != 0 {
            self.reverb |= bit;
        } else {
            self.reverb &= !bit;
        }
        self.pool.key_on |= bit;
        self.pool.key_off &= !self.pool.key_on;
        self.pool.lock = 0;
        Ok(NoteStart {
            voice: Some(allocation.index),
            immediate_key_off: allocation.immediate_key_off,
        })
    }
}

fn stereo_request(left: i32, right: i32) -> Result<(u8, u8), NoteSetupFault> {
    let left = i32::from(left as i16);
    let right = i32::from(right as i16);
    if left == right {
        Ok((left as u8, 64))
    } else if right < left {
        if left == 0 {
            return Err(NoteSetupFault::StereoDivisionByZero);
        }
        Ok((left as u8, ((right << 6) / left) as u8))
    } else {
        if right == 0 {
            return Err(NoteSetupFault::StereoDivisionByZero);
        }
        Ok((right as u8, (127 - (left << 6) / right) as u8))
    }
}
