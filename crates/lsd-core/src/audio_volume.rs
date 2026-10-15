use crate::AudioNoteDriver;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioVolumeContext {
    pub bank_volume: u8,
    pub program_volume: u8,
    pub tone_volume: u8,
    pub tone_pan: u8,
    pub program_pan: u8,
    pub request_pan: u8,
    pub request_volume: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioVolumeFault {
    DivisionByZero,
    InvalidVoice,
}

impl AudioNoteDriver {
    /// Literal SsUtAutoVol initialization. This preserves its signed halfword
    /// arithmetic rather than replacing it with a generic interpolation curve.
    pub fn auto_volume(
        &mut self,
        handle: i32,
        first: i32,
        target: i32,
        duration: i32,
    ) -> Result<i32, AudioVolumeFault> {
        let voice = handle as u16;
        if voice >= 24 {
            return Ok(-1);
        }
        let first = i32::from(first as i16);
        let target = i32::from(target as i16);
        if first == target {
            return Ok(0);
        }
        let slot = &mut self.pool.slots[usize::from(voice)];
        slot.set_word(28, 1);
        slot.set_word(36, first as u16);
        slot.set_word(38, target as u16);
        let duration = i32::from(duration as i16);
        let difference = first - target;
        if difference.abs() < duration {
            let interval = duration / difference;
            slot.set_word(30, 1);
            slot.set_word(32, interval as u16);
            slot.set_word(34, interval as u16);
        } else {
            if duration == 0 {
                return Err(AudioVolumeFault::DivisionByZero);
            }
            slot.set_word(32, 0);
            slot.set_word(30, (difference / duration) as u16);
        }
        Ok(0)
    }

    /// Original per-voice helper. The SDK invokes this only when record+28 is
    /// nonzero; its shared bank/tone context is retained across voices.
    pub fn advance_volume(&mut self, voice: usize) -> Result<(), AudioVolumeFault> {
        let slot = self
            .pool
            .slots
            .get_mut(voice)
            .ok_or(AudioVolumeFault::InvalidVoice)?;
        let interval = slot.word(32);
        if interval != 0 {
            let old = slot.word(34);
            slot.set_word(34, old.wrapping_sub(1));
            if (old as i16) > 0 {
                return Ok(());
            }
            slot.set_word(34, interval);
        }
        let step = slot.word(30) as i16;
        let mut volume = slot.word(36).wrapping_add(step as u16) as i16;
        let target = slot.word(38) as i16;
        if (step > 0 && volume >= target) || (step < 0 && volume <= target) {
            volume = target;
            slot.set_word(28, 0);
        }
        slot.set_word(36, volume as u16);
        self.volume_context.request_volume = volume as u8;
        let context = self.volume_context;
        let first = i32::from(volume).wrapping_mul(i32::from(context.bank_volume) * 16383) / 16129;
        let amplitude = (first as u32)
            .wrapping_mul(u32::from(context.program_volume))
            .wrapping_mul(u32::from(context.tone_volume))
            / 16129;
        let mut levels = [amplitude; 2];
        if context.tone_pan < 64 {
            levels[1] = levels[1].wrapping_mul(u32::from(context.tone_pan)) >> 6;
        } else {
            levels[0] =
                levels[0].wrapping_mul(127_u32.wrapping_sub(u32::from(context.tone_pan))) >> 6;
        }
        for pan in [context.program_pan, context.request_pan] {
            let index = if pan < 64 { 1 } else { 0 };
            let factor = if pan < 64 {
                u32::from(pan)
            } else {
                127_u32.wrapping_sub(u32::from(pan))
            };
            let product = (levels[index] as u16 as u32).wrapping_mul(factor);
            let rounded = if (product as i32) < 0 {
                product.wrapping_add(63)
            } else {
                product
            };
            levels[index] = rounded >> 6;
        }
        if self.mono == 1 {
            if (levels[0] as u16) < (levels[1] as u16) {
                levels[0] = levels[1];
            } else {
                levels[1] = levels[0];
            }
        }
        self.registers[voice][0] = levels[0] as u16;
        self.registers[voice][1] = levels[1] as u16;
        self.dirty[voice] |= 3;
        Ok(())
    }
}
