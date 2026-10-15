//! Original SDK music voices, separate from the sound-effect key-on entry point.
use crate::{AudioNoteDriver, AudioVolumeContext, SpuPitchFault, VoiceNoteDefinition};

/// Literal lookup window for signed semitone remainders: -176 through 207.
/// The negative entries are local reference data preceding the named SDK table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequencePitchTable {
    pub values: Box<[u16; 384]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceVolumeDefinition {
    pub bank: i16,
    pub bank_volume: u8,
    pub programs: Box<[[u8; 16]; 128]>,
    /// The SDK controller indexes tone number without its packed program base.
    pub first_program_tones: Box<[[u8; 32]; 16]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceChannelRequest {
    pub sequence: i16,
    pub program: i16,
    pub volume: u16,
    pub pan: u16,
    pub sequence_volume: [u16; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceProgramDefinition {
    pub bank: i16,
    pub program: i16,
    pub packed_program: u8,
    pub packed_program_count: u16,
    pub bank_volume: u8,
    pub program_volume: u8,
    pub program_pan: u8,
    pub tones: Vec<VoiceNoteDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceNoteRequest {
    /// Low byte: score slot. High byte: track within that score.
    pub sequence: i16,
    pub key: u16,
    pub velocity: u16,
    pub pan: u16,
    pub channel_volume: i16,
    pub sequence_volume: [u16; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceNoteStart {
    /// Literal SDK return: allocated voice indices folded with shift-four/OR.
    /// Zero also occurs when no tone starts; it is not a voice handle.
    pub result: i32,
    pub voices: Vec<u8>,
    pub disable_noise: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceVoiceFault {
    InvalidBinding,
    ChannelVolumeDivisionByZero,
    Pitch(SpuPitchFault),
    SpecialSample255,
}

impl AudioNoteDriver {
    /// Original func_80030980. Matching does not test voice mode, so released
    /// records retain controller behavior. Shared volume context is preserved.
    pub fn sequence_channel_volume(
        &mut self,
        bank: &SequenceVolumeDefinition,
        request: SequenceChannelRequest,
    ) -> Result<u8, SequenceVoiceFault> {
        let program = usize::try_from(request.program)
            .ok()
            .and_then(|i| bank.programs.get(i))
            .ok_or(SequenceVoiceFault::InvalidBinding)?;
        self.volume_context.bank_volume = bank.bank_volume;
        let mut changed = 0;
        for voice in 0..usize::from(self.pool.capacity()) {
            let slot = &self.pool.slots[voice];
            if slot.word(14) as i16 != request.sequence
                || slot.word(18) as i16 != request.program
                || slot.word(22) as i16 != bank.bank
            {
                continue;
            }
            let tone = bank
                .first_program_tones
                .get(usize::from(slot.word(20)))
                .ok_or(SequenceVoiceFault::InvalidBinding)?;
            let packed = bank
                .programs
                .get(usize::from(slot.word(16)))
                .ok_or(SequenceVoiceFault::InvalidBinding)?;
            let velocity = i32::from(slot.word(8) as i16) * i32::from(request.volume) / 127;
            let amplitude = velocity
                .wrapping_mul(0x3fff)
                .wrapping_mul(i32::from(bank.bank_volume))
                / 16129;
            let amplitude = (amplitude as u32)
                .wrapping_mul(u32::from(program[1]))
                .wrapping_mul(u32::from(tone[2]))
                / 16129;
            self.registers[voice][..2].copy_from_slice(&pan_sequence_levels(
                scale_sequence_levels(amplitude, request.sequence_volume),
                [tone[3], packed[4], request.pan as u8],
                self.mono,
            ));
            self.dirty[voice] |= 3;
            changed += 1;
        }
        Ok(changed)
    }

    /// Original func_8002F610/func_8002F3E8, including its asymmetric positive
    /// divisor 63, negative divisor 64 and retained unbent note record.
    pub fn sequence_pitch_bend(
        &mut self,
        program: &SequenceProgramDefinition,
        pitch: &SequencePitchTable,
        sequence: i16,
        bend: u16,
    ) -> Result<u8, SequenceVoiceFault> {
        self.volume_context.bank_volume = program.bank_volume;
        let mut changed = 0;
        let delta = i32::from(bend.wrapping_sub(64) as i16);
        for voice in 0..usize::from(self.pool.capacity()) {
            let slot = &self.pool.slots[voice];
            if slot.word(14) as i16 != sequence
                || slot.word(18) as i16 != program.program
                || slot.word(22) as i16 != program.bank
            {
                continue;
            }
            let tone = program
                .tones
                .get(usize::from(slot.word(20)))
                .ok_or(SequenceVoiceFault::InvalidBinding)?
                .attributes;
            let key = i32::from(slot.word(12));
            let (key, fine) = if delta > 0 {
                let product = delta * i32::from(tone[13]);
                let steps = product / 63;
                (key + steps, (product - steps * 63) * 2)
            } else if delta < 0 {
                let product = delta * i32::from(tone[12]);
                let steps = product / 64;
                (key + steps - 1, (product - steps * 64) * 2 + 127)
            } else {
                (key, 0)
            };
            self.pool.current = voice as u16;
            self.registers[voice][2] = pitch
                .fine_pitch(key as u16, fine as u16, tone[4], tone[5])
                .map_err(SequenceVoiceFault::Pitch)?;
            self.dirty[voice] |= 4;
            changed += 1;
        }
        Ok(changed)
    }

    /// Original func_8002FAC4: range-select every overlapping tone and allocate
    /// each through the shared pool. This entry neither checks nor changes the
    /// SDK callback lock; music dispatch normally executes while it is held.
    pub fn sequence_key_on(
        &mut self,
        program: &SequenceProgramDefinition,
        pitch: &SequencePitchTable,
        request: SequenceNoteRequest,
    ) -> Result<SequenceNoteStart, SequenceVoiceFault> {
        if !(0..16).contains(&program.bank)
            || !(0..128).contains(&program.program)
            || program.tones.len() > 16
        {
            return Err(SequenceVoiceFault::InvalidBinding);
        }
        self.volume_context.bank_volume = program.bank_volume;
        self.volume_context.program_volume = program.program_volume;
        self.volume_context.program_pan = program.program_pan;
        self.volume_context.request_volume = request.velocity as u8;
        self.volume_context.request_pan = request.pan as u8;
        let mut result = SequenceNoteStart {
            result: 0,
            voices: Vec::new(),
            disable_noise: Vec::new(),
        };
        if u16::from(program.packed_program) >= program.packed_program_count {
            result.result = -1;
            return Ok(result);
        }
        if request.velocity == 0 {
            self.sequence_key_off(request.sequence, program.bank, program.program, request.key)?;
            return Ok(result);
        }
        for (index, definition) in program.tones.iter().enumerate() {
            let tone = definition.attributes;
            let key = request.key as u8;
            if key < tone[6] || key > tone[7] {
                continue;
            }
            self.volume_context.tone_volume = tone[2];
            self.volume_context.tone_pan = tone[3];
            let allocation = self.pool.allocate(tone[0]);
            self.pool.current = u16::from(allocation.index);
            if allocation.index >= self.pool.capacity() {
                continue;
            }
            if let Some(mask) = allocation.disable_noise {
                result.disable_noise.push(mask);
            }
            let voice = usize::from(allocation.index);
            let slot = &mut self.pool.slots[voice];
            slot.record[27] = 1;
            slot.set_word(2, 0);
            slot.set_word(14, request.sequence as u16);
            slot.set_word(22, program.bank as u8 as u16);
            slot.set_word(16, u16::from(program.packed_program));
            slot.set_word(18, program.program as u16);
            if request.sequence != 0x21 {
                if request.channel_volume == 0 {
                    return Err(SequenceVoiceFault::ChannelVolumeDivisionByZero);
                }
                slot.set_word(
                    8,
                    (i32::from(request.velocity) * 127 / i32::from(request.channel_volume)) as u16,
                );
            }
            slot.record[10] = request.pan as u8;
            slot.set_word(20, index as u16);
            slot.set_word(12, request.key);
            slot.set_word(24, u16::from(tone[0]));
            let sample = u16::from(tone[22]);
            slot.set_word(0, sample);
            slot.set_word(6, 0x7fff);
            let bit = 1_u32 << allocation.index;
            for mask in &mut self.automation {
                *mask &= !bit;
            }
            let registers = &mut self.registers[voice];
            registers[3] = definition.sample_address;
            registers[4] = u16::from_le_bytes([tone[16], tone[17]]);
            registers[5] =
                u16::from_le_bytes([tone[18], tone[19]]).wrapping_add(self.adsr_release_offset);
            self.dirty[voice] |= 0x38;
            if sample == 255 {
                return Err(SequenceVoiceFault::SpecialSample255);
            }
            let value = pitch
                .pitch(key, tone[4], tone[5])
                .map_err(SequenceVoiceFault::Pitch)?;
            let levels = sequence_levels(
                self.volume_context,
                request.sequence,
                request.sequence_volume,
                self.mono,
            );
            registers[0] = levels[0];
            registers[1] = levels[1];
            registers[2] = value;
            self.dirty[voice] |= 7;
            self.pool.slots[voice].set_word(4, value);
            if tone[1] & 4 != 0 {
                self.reverb |= bit;
            } else {
                self.reverb &= !bit;
            }
            self.pool.key_on |= bit;
            self.pool.key_off &= !self.pool.key_on;
            result.result = result.result.wrapping_shl(4) | i32::from(allocation.index);
            result.voices.push(allocation.index);
        }
        Ok(result)
    }

    /// Original func_800300D0 matches sequence, bank, program and key, including
    /// matching records that are already released. It does not acquire the lock.
    pub fn sequence_key_off(
        &mut self,
        sequence: i16,
        bank: i16,
        program: i16,
        key: u16,
    ) -> Result<u8, SequenceVoiceFault> {
        let mut released = 0_u8;
        for index in 0..self.pool.capacity() {
            let slot = &mut self.pool.slots[usize::from(index)];
            if i32::from(slot.word(12) as i16) != i32::from(key)
                || slot.word(18) as i16 != program
                || slot.word(14) as i16 != sequence
                || slot.word(22) as i16 != bank
            {
                continue;
            }
            if slot.word(0) == 255 {
                return Err(SequenceVoiceFault::SpecialSample255);
            }
            self.pool.current = u16::from(index);
            slot.record[27] = 0;
            slot.set_word(4, 0);
            slot.set_word(0, 0);
            self.pool.key_off |= 1_u32 << index;
            self.pool.key_on &= !self.pool.key_off;
            released = released.wrapping_add(1);
        }
        Ok(released)
    }
}

impl SequencePitchTable {
    fn lookup(&self, index: i32, octave: i32) -> Result<u16, SpuPitchFault> {
        let value = usize::try_from(index + 176)
            .ok()
            .and_then(|index| self.values.get(index))
            .ok_or(SpuPitchFault { table_index: index })?;
        Ok(if octave > 0 {
            u32::from(*value).wrapping_shl(octave as u32)
        } else {
            u32::from(*value).wrapping_shr((-octave) as u32)
        } as u16)
    }

    /// Music's func_8002DF7C clamps the correction fraction to fifteen, unlike
    /// the sound-effect entry's explicit fine input and semitone carry.
    pub fn pitch(&self, key: u8, center: u8, correction: u8) -> Result<u16, SpuPitchFault> {
        let adjusted = i32::from(key) + 60 - i32::from(center);
        let octave = adjusted / 12 - 5;
        let index = adjusted % 12 * 16 + i32::from((correction >> 3).min(15));
        self.lookup(index, octave)
    }

    pub fn fine_pitch(
        &self,
        key: u16,
        fine: u16,
        center: u8,
        correction: u8,
    ) -> Result<u16, SpuPitchFault> {
        let steps = (i32::from(fine) + i32::from(correction)) / 8;
        let carry = i32::from(steps >= 16);
        let adjusted = (i32::from(key) + 60 - i32::from(center) + carry) as i16;
        let index = i32::from(adjusted % 12) * 16 + i32::from((steps - carry * 16) as i16);
        self.lookup(index, i32::from(adjusted / 12 - 5))
    }
}

fn sequence_levels(
    context: AudioVolumeContext,
    sequence: i16,
    sequence_volume: [u16; 2],
    mono: i16,
) -> [u16; 2] {
    let amplitude =
        u32::from(context.bank_volume) * 0x3fff * u32::from(context.request_volume) / 16129;
    let amplitude = amplitude
        .wrapping_mul(u32::from(context.program_volume))
        .wrapping_mul(u32::from(context.tone_volume))
        / 16129;
    let mut levels = [amplitude; 2];
    if sequence != 0x21 {
        levels = scale_sequence_levels(amplitude, sequence_volume);
    }
    pan_sequence_levels(
        levels,
        [context.tone_pan, context.program_pan, context.request_pan],
        mono,
    )
}

fn scale_sequence_levels(amplitude: u32, sequence_volume: [u16; 2]) -> [u32; 2] {
    sequence_volume.map(|volume| amplitude.wrapping_mul(u32::from(volume)) / 127)
}

fn pan_sequence_levels(mut levels: [u32; 2], pans: [u8; 3], mono: i16) -> [u16; 2] {
    for pan in pans {
        if pan < 64 {
            levels[1] = levels[1].wrapping_mul(u32::from(pan)) / 63;
        } else {
            levels[0] = levels[0].wrapping_mul(127_u32.wrapping_sub(u32::from(pan))) / 63;
        }
    }
    if mono == 1 {
        levels = [levels[0].max(levels[1]); 2];
    }
    levels.map(|level| (level.wrapping_mul(level) / 16383) as u16)
}
