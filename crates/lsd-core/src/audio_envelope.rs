//! Per-sample SPU envelope and volume state. Register/key timing is handled by
//! the device layer; these components advance only when explicitly clocked.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpuEnvelopePhase {
    #[default]
    Attack = 0,
    Decay = 1,
    Sustain = 2,
    Release = 3,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpuEnvelope {
    pub control: [u16; 2],
    /// Raw current-level register; manual writes can set its sign bit.
    pub level: u16,
    pub divider: u32,
    pub phase: SpuEnvelopePhase,
}

impl SpuEnvelope {
    pub fn new(control: [u16; 2]) -> Self {
        Self {
            control,
            ..Self::default()
        }
    }

    /// Internal action when the device accepts a key-on, after its timing gate.
    pub fn restart_attack(&mut self) {
        self.level = 0;
        self.divider = 0;
        self.phase = SpuEnvelopePhase::Attack;
    }

    /// Internal action when the device accepts a release. Repeated key-off
    /// gating belongs to the device rather than this unconditional action.
    pub fn begin_release(&mut self) {
        self.divider = 0;
        self.phase = SpuEnvelopePhase::Release;
    }

    pub fn clock(&mut self) {
        use SpuEnvelopePhase::*;
        if self.phase == Attack && self.level == 0x7fff {
            self.phase = Decay;
        }
        let [first, second] = self.control;
        let (rate, exponential, decreasing, stopped_rate) = match self.phase {
            Attack => (((first >> 8) & 127) as u8, first & 0x8000 != 0, false, 127),
            Decay => (((first >> 4) & 15) as u8 * 4, true, true, 124),
            Sustain => (
                ((second >> 6) & 127) as u8,
                second & 0x8000 != 0,
                second & 0x4000 != 0,
                127,
            ),
            Release => ((second & 31) as u8 * 4, second & 32 != 0, true, 124),
        };
        let (step, counter_increment) = volume_delta(
            rate,
            stopped_rate,
            exponential,
            decreasing,
            decreasing,
            self.level as i16,
        );
        self.divider = self.divider.wrapping_add(counter_increment);
        if self.divider & 0x8000 == 0 {
            return;
        }
        self.divider = 0;
        let previous = self.level;
        self.level = self.level.wrapping_add(step as u16);
        let overflow = if self.phase == Attack {
            previous & 0x8000 == 0 && self.level & 0x8000 != 0
        } else {
            self.level & 0x8000 != 0
        };
        if overflow {
            self.level = if decreasing { 0 } else { 0x7fff };
        }
        let sustain_target = ((first & 15) + 1) << 11;
        if self.phase == Decay && self.level < sustain_target {
            self.phase = Sustain;
        }
    }
}

/// Current-volume state shared by the main output and individual voice gains.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpuVolume {
    pub control: u16,
    pub level: u16,
    pub divider: u32,
}

impl SpuVolume {
    /// A control write does not immediately change current volume or reset the
    /// counter. Direct mode becomes visible on the subsequent sample clock.
    pub fn write_control(&mut self, control: u16) {
        self.control = control;
    }

    pub fn current(&self) -> i16 {
        self.level as i16
    }

    pub fn clock(&mut self) {
        if self.control & 0x8000 == 0 {
            self.level = self.control << 1;
            return;
        }
        let exponential = self.control & 0x4000 != 0;
        let decreasing = self.control & 0x2000 != 0;
        let negative = self.control & 0x1000 != 0;
        let invert_step = (decreasing ^ negative) || (decreasing && exponential);
        let invert_level = negative && !(decreasing && exponential);
        let working_level = if invert_level {
            !self.level
        } else {
            self.level
        } as i16;
        let (step, counter_increment) = volume_delta(
            (self.control & 127) as u8,
            127,
            exponential,
            decreasing,
            invert_step,
            working_level,
        );
        let wrong_sign = (self.level & 0x8000 != 0) != negative;
        if decreasing && !(negative && exponential) && (wrong_sign || self.level == 0) {
            self.level = 0;
            return;
        }
        self.divider = self.divider.wrapping_add(counter_increment);
        if self.divider & 0x8000 == 0 {
            return;
        }
        self.divider = 0;
        let target = if negative { 0x8000 } else { 0x7fff };
        if !decreasing && self.level == target {
            return;
        }
        let previous = self.level;
        self.level = self.level.wrapping_add(step as u16);
        if !decreasing
            && (self.level ^ previous) & 0x8000 != 0
            && (self.level & 0x8000 != 0) != negative
        {
            self.level = target;
        }
    }
}

fn volume_delta(
    rate: u8,
    stopped_rate: u8,
    exponential: bool,
    decreasing: bool,
    invert: bool,
    level: i16,
) -> (i32, u32) {
    let mut step = 7 - i32::from(rate & 3);
    if invert {
        step = !step;
    }
    step <<= 11_u8.saturating_sub(rate >> 2);
    let mut increment = 0x8000_u32 >> (rate >> 2).saturating_sub(11);
    if exponential {
        if decreasing {
            step = (step * i32::from(level)) >> 15;
        } else if (level as u16 & 0x7fff) >= 0x6000 {
            if rate < 40 {
                step >>= 2;
            } else if rate >= 44 {
                increment >>= 2;
            } else {
                step >>= 1;
                increment >>= 1;
            }
        }
    }
    if increment == 0 && rate < stopped_rate {
        increment = 1;
    }
    (step, increment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_release_actions_reset_only_the_required_state() {
        let mut envelope = SpuEnvelope {
            control: [0x1234, 0x5678],
            level: 0x6000,
            divider: 1234,
            phase: SpuEnvelopePhase::Sustain,
        };
        envelope.begin_release();
        assert_eq!(
            (envelope.level, envelope.divider, envelope.phase),
            (0x6000, 0, SpuEnvelopePhase::Release)
        );
        envelope.restart_attack();
        assert_eq!(
            (
                envelope.control,
                envelope.level,
                envelope.divider,
                envelope.phase
            ),
            ([0x1234, 0x5678], 0, 0, SpuEnvelopePhase::Attack)
        );
    }

    #[test]
    fn direct_gain_is_delayed_and_retains_counter() {
        let mut volume = SpuVolume {
            control: 0,
            level: 1234,
            divider: 5678,
        };
        volume.write_control(0x4000);
        assert_eq!(volume.level, 1234);
        volume.clock();
        assert_eq!(volume.current(), -32768);
        assert_eq!(volume.divider, 5678);
    }

    #[test]
    fn all_rate_bits_set_hold_attack_and_release() {
        let mut envelope = SpuEnvelope {
            control: [0x7f00, 31],
            level: 12345,
            divider: 0,
            phase: SpuEnvelopePhase::Attack,
        };
        for _ in 0..65536 {
            envelope.clock();
        }
        assert_eq!(envelope.level, 12345);
        envelope.begin_release();
        for _ in 0..65536 {
            envelope.clock();
        }
        assert_eq!(envelope.level, 12345);
    }
}
