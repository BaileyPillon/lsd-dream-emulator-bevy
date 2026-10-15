//! Clocked voice, memory and register path. The returned accumulators precede
//! the reverb and main-output stage; this is not an output-device adapter.

use crate::audio_sample::spu_pitch_step_wide;
use crate::{
    SpuAdpcmDecoder, SpuAdpcmFault, SpuEnvelope, SpuEnvelopePhase, SpuVolume, spu_gaussian,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioEngineTickFault {
    Driver(crate::AudioTickFault),
    EngineRegister(u16),
}

impl crate::AudioNoteDriver {
    /// Feed the SDK update from current device envelope/noise registers, then
    /// execute its ordered writes. The caller still selects callback timing.
    pub fn tick_engine(
        &mut self,
        engine: &mut SpuVoiceEngine,
    ) -> Result<Vec<crate::AudioRegisterWrite>, AudioEngineTickFault> {
        let levels = std::array::from_fn(|index| engine.voices[index].envelope.level);
        let noise = [engine.registers[0x194 / 2], engine.registers[0x196 / 2]];
        let writes = self
            .tick(levels, noise)
            .map_err(AudioEngineTickFault::Driver)?;
        for write in &writes {
            engine
                .write_register(write.offset, write.value)
                .map_err(|_| AudioEngineTickFault::EngineRegister(write.offset))?;
        }
        Ok(writes)
    }
}

const RAM_WORDS: usize = 0x40000;
const RAM_MASK: u32 = 0x3ffff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpuEngineFault {
    Register(u16),
    UploadRange,
    Adpcm { voice: u8, fault: SpuAdpcmFault },
}

impl std::fmt::Display for SpuEngineFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SPU engine error: {self:?}")
    }
}
impl std::error::Error for SpuEngineFault {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuSampleMemory {
    words: Box<[u16; RAM_WORDS]>,
}

impl Default for SpuSampleMemory {
    fn default() -> Self {
        Self {
            words: vec![0; RAM_WORDS].into_boxed_slice().try_into().unwrap(),
        }
    }
}

impl SpuSampleMemory {
    pub fn read_word(&self, address: u32) -> u16 {
        self.words[(address & RAM_MASK) as usize]
    }
    pub fn write_word(&mut self, address: u32, value: u16) {
        self.words[(address & RAM_MASK) as usize] = value;
    }
    /// Install identified sample bytes without modelling a DMA duration. Live
    /// transfers use the engine's transfer-address/data operations instead.
    pub fn upload(&mut self, byte_address: u32, bytes: &[u8]) -> Result<(), SpuEngineFault> {
        let start = byte_address as usize;
        if !start.is_multiple_of(2)
            || !bytes.len().is_multiple_of(2)
            || start
                .checked_add(bytes.len())
                .is_none_or(|end| end > RAM_WORDS * 2)
        {
            return Err(SpuEngineFault::UploadRange);
        }
        for (word, pair) in self.words[start / 2..(start + bytes.len()) / 2]
            .iter_mut()
            .zip(bytes.chunks_exact(2))
        {
            *word = u16::from_le_bytes([pair[0], pair[1]]);
        }
        Ok(())
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        self.words
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpuNoise {
    pub divider: u32,
    pub counter: u32,
    pub level: u16,
}

impl SpuNoise {
    pub fn clock(&mut self, control: u16) {
        let rate = (control >> 8) & 63;
        let (divider_increment, counter_increment) = if rate >= 60 {
            (0x8000, 8)
        } else {
            (2_u32 << (rate >> 2), 4 + u32::from(rate & 3))
        };
        self.divider = self.divider.wrapping_add(divider_increment);
        if self.divider & 0x8000 != 0 {
            self.divider = 0;
            self.counter = self.counter.wrapping_add(counter_increment);
            if self.counter & 8 != 0 {
                self.counter &= 7;
                let bit = ((self.level >> 15)
                    ^ (self.level >> 12)
                    ^ (self.level >> 11)
                    ^ (self.level >> 10)
                    ^ 1)
                    & 1;
                self.level = (self.level << 1) | bit;
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SpuVoice {
    pub decoder: SpuAdpcmDecoder,
    pub decoded: [i16; 32],
    pub play_delay: u32,
    pub write_position: u32,
    pub read_position: u32,
    pub available: u32,
    pub header: u8,
    pub flags: u8,
    pub ignore_loop_start: bool,
    pub volume: [SpuVolume; 2],
    pub pitch: u16,
    pub phase: u32,
    /// Internal addresses are halfword indices in the 512 KiB memory.
    pub start_address: u32,
    pub current_address: u32,
    pub loop_address: u32,
    pub envelope: SpuEnvelope,
    /// The reference power state has a zero cached target until an ADSR write.
    pub sustain_target: u16,
    pub output: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpuVoiceFrame {
    /// Voice/CD mix before reverb and main volume; deliberately not saturated.
    pub dry: [i32; 2],
    pub reverb_input: [i16; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuVoiceEngine {
    pub memory: SpuSampleMemory,
    pub voices: [SpuVoice; 24],
    pub registers: [u16; 256],
    pub auxiliary: [u16; 16],
    pub noise: SpuNoise,
    pub pitch_modulation: u32,
    pub noise_mode: u32,
    pub reverb_mode: u32,
    pub key_on: u32,
    pub key_off: u32,
    pub block_end: u32,
    pub capture_address: u32,
    pub irq_address: u32,
    pub irq_asserted: bool,
    pub transfer_address: u32,
    pub control: u16,
    pub main_volume: [SpuVolume; 2],
    pub reverb_volume: [i16; 2],
    pub cd_volume: [i16; 2],
    pub external_volume: [i16; 2],
    pub reverb_base: u32,
    pub reverb_current: u32,
}

impl Default for SpuVoiceEngine {
    fn default() -> Self {
        Self {
            memory: SpuSampleMemory::default(),
            voices: std::array::from_fn(|_| SpuVoice::default()),
            registers: [0; 256],
            auxiliary: [0; 16],
            noise: SpuNoise::default(),
            pitch_modulation: 0,
            noise_mode: 0,
            reverb_mode: 0,
            key_on: 0,
            key_off: 0,
            block_end: 0,
            capture_address: 0,
            irq_address: 0,
            irq_asserted: false,
            transfer_address: 0,
            control: 0,
            main_volume: [SpuVolume::default(); 2],
            reverb_volume: [0; 2],
            cd_volume: [0; 2],
            external_volume: [0; 2],
            reverb_base: 0,
            reverb_current: 0,
        }
    }
}

impl SpuVoiceEngine {
    fn irq(&mut self, address: u32) {
        if self.control & 0x40 != 0 && self.irq_address == address {
            self.irq_asserted = true;
        }
    }
    fn decoder_irq(&mut self, address: u32) {
        self.irq(address);
        self.irq(address & !7);
    }
    fn write_memory(&mut self, address: u32, value: u16) {
        self.irq(address);
        self.memory.write_word(address, value);
    }
    fn read_memory(&mut self, address: u32) -> u16 {
        self.irq(address);
        self.memory.read_word(address)
    }
    pub fn dma_write(&mut self, value: u32) {
        self.write_memory(self.transfer_address, value as u16);
        self.transfer_address = (self.transfer_address + 1) & RAM_MASK;
        self.write_memory(self.transfer_address, (value >> 16) as u16);
        self.transfer_address = (self.transfer_address + 1) & RAM_MASK;
        self.irq(self.transfer_address);
    }
    pub fn dma_read(&mut self) -> u32 {
        let low = u32::from(self.read_memory(self.transfer_address));
        self.transfer_address = (self.transfer_address + 1) & RAM_MASK;
        let high = u32::from(self.read_memory(self.transfer_address));
        self.transfer_address = (self.transfer_address + 1) & RAM_MASK;
        self.irq(self.transfer_address);
        low | high << 16
    }
    fn validate_register(offset: u16) -> Result<(), SpuEngineFault> {
        if offset < 0x400 && offset.is_multiple_of(2) {
            Ok(())
        } else {
            Err(SpuEngineFault::Register(offset))
        }
    }
    pub fn write_register(&mut self, offset: u16, value: u16) -> Result<(), SpuEngineFault> {
        Self::validate_register(offset)?;
        if offset >= 0x200 {
            if offset < 0x260 {
                self.voices[usize::from((offset - 0x200) / 4)].volume
                    [usize::from((offset & 2) / 2)]
                .level = value;
            } else if offset < 0x280 {
                self.auxiliary[usize::from((offset & 31) / 2)] = value;
            }
            return Ok(());
        }
        if offset < 0x180 {
            let voice = &mut self.voices[usize::from(offset / 16)];
            match offset & 15 {
                0 | 2 => voice.volume[usize::from((offset & 2) / 2)].write_control(value),
                4 => voice.pitch = value,
                6 => voice.start_address = u32::from(value) << 2,
                8 => {
                    voice.envelope.control[0] = value;
                    voice.sustain_target = ((voice.envelope.control[0] & 15) + 1) << 11;
                }
                10 => {
                    voice.envelope.control[1] = value;
                    voice.sustain_target = ((voice.envelope.control[0] & 15) + 1) << 11;
                }
                12 => voice.envelope.level = value,
                14 => {
                    voice.loop_address = u32::from(value) << 2;
                    voice.ignore_loop_start = true;
                }
                _ => unreachable!(),
            }
        } else {
            let replace_mask = |mask: &mut u32, high: bool, limit_high: bool| {
                if high {
                    *mask = (*mask & 0xffff)
                        | (u32::from(value) & if limit_high { 255 } else { 65535 }) << 16;
                } else {
                    *mask = (*mask & 0xffff0000) | u32::from(value);
                }
            };
            match offset {
                0x180 | 0x182 => {
                    self.main_volume[usize::from((offset & 2) / 2)].write_control(value)
                }
                0x184 | 0x186 => self.reverb_volume[usize::from((offset & 2) / 2)] = value as i16,
                0x188 | 0x18a => replace_mask(&mut self.key_on, offset & 2 != 0, true),
                0x18c | 0x18e => replace_mask(&mut self.key_off, offset & 2 != 0, true),
                0x190 | 0x192 => replace_mask(&mut self.pitch_modulation, offset & 2 != 0, true),
                0x194 | 0x196 => replace_mask(&mut self.noise_mode, offset & 2 != 0, true),
                0x198 | 0x19a => replace_mask(&mut self.reverb_mode, offset & 2 != 0, true),
                0x19c | 0x19e => replace_mask(&mut self.block_end, offset & 2 != 0, false),
                0x1a2 => {
                    self.reverb_base = u32::from(value) << 2;
                    self.reverb_current = self.reverb_base;
                }
                0x1a4 => {
                    self.irq_address = u32::from(value) << 2;
                    self.irq(self.transfer_address);
                }
                0x1a6 => {
                    self.transfer_address = u32::from(value) << 2;
                    self.irq(self.transfer_address);
                }
                0x1a8 => {
                    self.write_memory(self.transfer_address, value);
                    self.transfer_address = (self.transfer_address + 1) & RAM_MASK;
                    self.irq(self.transfer_address);
                }
                0x1aa => {
                    self.control = value;
                    if value & 0x40 == 0 {
                        self.irq_asserted = false;
                    }
                    self.irq(self.transfer_address);
                }
                0x1b0 | 0x1b2 => self.cd_volume[usize::from((offset & 2) / 2)] = value as i16,
                0x1b4 | 0x1b6 => self.external_volume[usize::from((offset & 2) / 2)] = value as i16,
                0x1b8 | 0x1ba => self.main_volume[usize::from((offset & 2) / 2)].level = value,
                _ => {}
            }
        }
        self.registers[usize::from(offset / 2)] = value;
        Ok(())
    }
    pub fn read_register(&mut self, offset: u16) -> Result<u16, SpuEngineFault> {
        Self::validate_register(offset)?;
        if offset >= 0x200 {
            return Ok(if offset < 0x260 {
                self.voices[usize::from((offset - 0x200) / 4)].volume[usize::from((offset & 2) / 2)]
                    .level
            } else if offset < 0x280 {
                self.auxiliary[usize::from((offset & 31) / 2)]
            } else {
                0xffff
            });
        }
        if offset < 0x180 {
            let voice = &self.voices[usize::from(offset / 16)];
            if offset & 15 == 12 {
                return Ok(voice.envelope.level);
            }
            if offset & 15 == 14 {
                return Ok((voice.loop_address >> 2) as u16);
            }
        }
        Ok(match offset {
            0x19c => self.block_end as u16,
            0x19e => (self.block_end >> 16) as u16,
            0x1a8 => {
                let value = self.read_memory(self.transfer_address);
                self.transfer_address = (self.transfer_address + 1) & RAM_MASK;
                self.irq(self.transfer_address);
                value
            }
            0x1aa => self.control,
            0x1b8 | 0x1ba => self.main_volume[usize::from((offset & 2) / 2)].level,
            0x1bc => 0,
            _ => self.registers[usize::from(offset / 2)],
        })
    }
    fn decode_voice(&mut self, index: usize) -> Result<(), SpuEngineFault> {
        if self.voices[index].available >= 11 {
            self.decoder_irq(self.voices[index].current_address.wrapping_sub(1) & RAM_MASK);
            return Ok(());
        }
        if self.voices[index].current_address & 7 == 0 && self.voices[index].flags & 1 != 0 {
            self.voices[index].current_address = self.voices[index].loop_address & !7;
            self.block_end |= 1 << index;
            if self.voices[index].flags & 2 == 0 && self.noise_mode & (1 << index) == 0 {
                self.voices[index].envelope.phase = SpuEnvelopePhase::Release;
                self.voices[index].envelope.level = 0;
            }
        }
        self.decoder_irq(self.voices[index].current_address);
        if self.voices[index].current_address & 7 == 0 {
            let address = self.voices[index].current_address;
            let header = self.memory.read_word(address);
            let voice = &mut self.voices[index];
            voice.header = header as u8;
            voice.flags = (header >> 8) as u8;
            if voice.flags & 4 != 0 && !voice.ignore_loop_start {
                voice.loop_address = address;
            }
            voice.current_address = (voice.current_address + 1) & RAM_MASK;
        }
        let packed = self.memory.read_word(self.voices[index].current_address);
        let voice = &mut self.voices[index];
        let samples = voice
            .decoder
            .decode_word(voice.header, packed)
            .map_err(|fault| SpuEngineFault::Adpcm {
                voice: index as u8,
                fault,
            })?;
        for (offset, sample) in samples.into_iter().enumerate() {
            voice.decoded[(voice.write_position as usize + offset) & 31] = sample;
        }
        voice.write_position = (voice.write_position + 4) & 31;
        voice.available += 4;
        voice.current_address = (voice.current_address + 1) & RAM_MASK;
        Ok(())
    }
    /// One explicit 44.1 kHz voice/capture/noise clock. Reverb and main gain are
    /// subsequent stages and are not silently approximated in this method.
    pub fn clock_voices(&mut self, cd_input: [i16; 2]) -> Result<SpuVoiceFrame, SpuEngineFault> {
        self.registers[0x1ae / 2] = (self.control & 63) | if self.irq_asserted { 64 } else { 0 };
        if self.registers[0x1ac / 2] == 4 && self.capture_address & 0x100 != 0 {
            self.registers[0x1ae / 2] |= 0x800;
        }
        let mut dry = [0_i32; 2];
        let mut wet = [0_i32; 2];
        let mut previous_output = 0;
        for index in 0..24 {
            if self.voices[index].play_delay != 0 {
                self.voices[index].ignore_loop_start = false;
            }
            self.decode_voice(index)?;
            let voice = &mut self.voices[index];
            let sample = if self.noise_mode & (1 << index) != 0 {
                self.noise.level as i16
            } else {
                spu_gaussian(
                    std::array::from_fn(|n| {
                        voice.decoded[((voice.read_position as usize) + n) & 31]
                    }),
                    ((voice.phase & 0xfff) >> 4) as u8,
                )
            };
            voice.output = (i32::from(sample) * i32::from(voice.envelope.level as i16)) >> 15;
            let output = voice.output;
            for side in 0..2 {
                let value = (output * i32::from(voice.volume[side].current())) >> 15;
                dry[side] += value;
                if self.reverb_mode & (1 << index) != 0 {
                    wet[side] += value;
                }
                voice.volume[side].clock();
            }
            if voice.play_delay == 0 {
                voice
                    .envelope
                    .clock_with_sustain_target(voice.sustain_target);
                let modulator = (index != 0 && self.pitch_modulation & (1 << index) != 0)
                    .then_some(previous_output);
                let phase = voice.phase + u32::from(spu_pitch_step_wide(voice.pitch, modulator));
                let consumed = phase >> 12;
                voice.phase = phase & 0xfff;
                voice.available = voice.available.wrapping_sub(consumed);
                voice.read_position = (voice.read_position + consumed) & 31;
            } else {
                voice.play_delay -= 1;
            }
            if self.key_off & (1 << index) != 0
                && voice.envelope.phase != SpuEnvelopePhase::Release
                && voice.play_delay < 3
            {
                voice.envelope.begin_release();
            }
            if self.key_on & (1 << index) != 0 {
                voice.envelope.restart_attack();
                voice.flags = 0;
                voice.write_position = 0;
                voice.read_position = 0;
                voice.available = 0;
                voice.play_delay = 4;
                voice.decoder.history = [0; 2];
                voice.phase = 0;
                voice.current_address = voice.start_address & !7;
                voice.ignore_loop_start = false;
                self.block_end &= !(1 << index);
            }
            if self.control & 0x8000 == 0 {
                voice.envelope.phase = SpuEnvelopePhase::Release;
                voice.envelope.level = 0;
            }
            previous_output = output;
            if index == 1 || index == 3 {
                self.write_memory(
                    0x400 | ((index as u32 >> 1) * 0x200) | self.capture_address,
                    output as u16,
                );
            }
        }
        self.key_on = 0;
        self.key_off = 0;
        if self.control & 0x4000 == 0 {
            dry = [0; 2];
            wet = [0; 2];
        }
        for side in 0..2 {
            self.write_memory(
                (side as u32 * 0x200) | self.capture_address,
                cd_input[side] as u16,
            );
            if self.control & 1 != 0 {
                let value = (i32::from(cd_input[side]) * i32::from(self.cd_volume[side])) >> 15;
                dry[side] += value;
                if self.control & 4 != 0 {
                    wet[side] += value;
                }
            }
        }
        self.capture_address = (self.capture_address + 1) & 0x1ff;
        self.noise.clock(self.control);
        Ok(SpuVoiceFrame {
            dry,
            reverb_input: wet.map(|value| value.clamp(-32768, 32767) as i16),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uploads_reject_invalid_ranges_before_writing() {
        let mut memory = SpuSampleMemory::default();
        memory.upload(0x7fffe, &[0x34, 0x12]).unwrap();
        assert_eq!(memory.read_word(0x3ffff), 0x1234);
        for (address, bytes) in [
            (1, &[1, 2][..]),
            (0, &[1][..]),
            (0x7fffe, &[1, 2, 3, 4][..]),
        ] {
            assert_eq!(
                memory.upload(address, bytes),
                Err(SpuEngineFault::UploadRange)
            );
        }
        assert_eq!(memory.read_word(0x3ffff), 0x1234);
        assert_eq!(memory.read_word(0), 0);
    }

    #[test]
    fn sdk_flush_reads_device_levels_and_executes_ordered_writes() {
        let mut driver = crate::AudioNoteDriver::new(24);
        let mut engine = SpuVoiceEngine::default();
        engine.voices[0].envelope.level = 1234;
        driver.pool.key_on = 1;
        driver.registers[0][0] = 0x1234;
        driver.registers[0][1] = 0x2345;
        driver.registers[0][2] = 0x1000;
        driver.dirty[0] = 7;
        let writes = driver.tick_engine(&mut engine).unwrap();
        assert_eq!(driver.pool.slots[0].word(6), 1234);
        assert_eq!(engine.voices[0].volume[0].control, 0x1234);
        assert_eq!(engine.voices[0].volume[1].control, 0x2345);
        assert_eq!(engine.voices[0].pitch, 0x1000);
        assert_eq!(engine.key_on, 1);
        assert_eq!(driver.pool.key_on, 0);
        assert_eq!(writes[0].offset, 0);
    }

    #[test]
    fn invalid_registers_do_not_change_engine_state() {
        let mut engine = SpuVoiceEngine::default();
        assert_eq!(
            engine.write_register(1, 0xffff),
            Err(SpuEngineFault::Register(1))
        );
        assert_eq!(
            engine.read_register(0x400),
            Err(SpuEngineFault::Register(0x400))
        );
        assert_eq!(engine.registers, [0; 256]);
    }
}
