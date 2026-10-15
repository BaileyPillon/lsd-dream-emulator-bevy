//! Original SDK sequence timing in tenths of MIDI ticks, driven by callbacks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceClockFault {
    ZeroTempo,
    ZeroResolutionProduct,
    ZeroCallbackRate,
    EventLimit,
}

/// Software fields at offsets 0x6e, 0x70, 0x72 and 0x88 of an SDK score.
/// Signed halfwords and wrapping word operations are observable behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceClock {
    pub countdown: i16,
    pub step: i16,
    pub initial_step: i16,
    pub remaining: i32,
}

impl SequenceClock {
    /// `_SsInitSoundSeq` rounds BPM to nearest, with ties down. The supplied
    /// callback rate is the SDK's VBLANK_MINUS value, not host frame rate.
    pub fn initialize(
        resolution: i16,
        tempo_microseconds: u32,
        callback_rate: u32,
        initial_delta: u32,
    ) -> Result<(Self, u32), SequenceClockFault> {
        if tempo_microseconds == 0 {
            return Err(SequenceClockFault::ZeroTempo);
        }
        let bpm = 60_000_000 / tempo_microseconds
            + u32::from(60_000_000 % tempo_microseconds > tempo_microseconds / 2);
        let (countdown, step) = timing(resolution, bpm, callback_rate)?;
        Ok((
            Self {
                countdown,
                step,
                initial_step: step,
                remaining: initial_delta.wrapping_mul(10) as i32,
            },
            bpm,
        ))
    }

    /// The embedded 0xff51 handler truncates BPM; it does not use the header's
    /// rounding rule and does not rewrite the saved initial step.
    pub fn set_tempo(
        &mut self,
        resolution: i16,
        tempo_microseconds: u32,
        callback_rate: u32,
    ) -> Result<u32, SequenceClockFault> {
        if tempo_microseconds == 0 {
            return Err(SequenceClockFault::ZeroTempo);
        }
        let bpm = 60_000_000 / tempo_microseconds;
        let (countdown, step) = timing(resolution, bpm, callback_rate)?;
        self.countdown = countdown;
        self.step = step;
        Ok(bpm)
    }

    /// Original func_80034138. The event decoder mutates `remaining` (the next
    /// delta), and may change `step` via a tempo event. Effects run immediately.
    /// The explicit limit bounds malformed zero-delta streams; valid inputs
    /// must provide enough budget to reach the original return boundary.
    pub fn advance<E>(
        &mut self,
        event_limit: usize,
        mut dispatch: impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Result<i32, SequenceAdvanceFault<E>> {
        let difference = self.remaining.wrapping_sub(i32::from(self.step));
        if difference > 0 {
            if self.countdown > 0 {
                let result = i32::from(self.countdown) - 1;
                self.countdown = result as i16;
                return Ok(result);
            }
            if self.countdown == 0 {
                self.countdown = self.step;
                self.remaining = self.remaining.wrapping_sub(1);
                return Ok(self.remaining);
            }
            self.remaining = difference;
            return Ok(i32::from(self.countdown));
        }
        if i32::from(self.step) < self.remaining {
            return Ok(1);
        }
        let mut accumulated = self.remaining;
        for _ in 0..event_limit {
            dispatch(self).map_err(SequenceAdvanceFault::Event)?;
            if self.remaining == 0 {
                continue;
            }
            accumulated = accumulated.wrapping_add(self.remaining);
            if accumulated < i32::from(self.step) {
                continue;
            }
            self.remaining = accumulated.wrapping_sub(i32::from(self.step));
            return Ok(self.remaining);
        }
        Err(SequenceAdvanceFault::Clock(SequenceClockFault::EventLimit))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceAdvanceFault<E> {
    Clock(SequenceClockFault),
    Event(E),
}

fn timing(resolution: i16, bpm: u32, rate: u32) -> Result<(i16, i16), SequenceClockFault> {
    let product = (i32::from(resolution) as u32).wrapping_mul(bpm);
    let denominator = rate.wrapping_mul(60);
    let scaled = product.wrapping_mul(10);
    if scaled < denominator {
        if product == 0 {
            return Err(SequenceClockFault::ZeroResolutionProduct);
        }
        let slow = rate.wrapping_mul(600) / product;
        Ok((slow as i16, slow as i16))
    } else {
        if denominator == 0 {
            return Err(SequenceClockFault::ZeroCallbackRate);
        }
        let quotient = scaled / denominator;
        let remainder = scaled % denominator;
        let step = quotient.wrapping_add(u32::from(remainder > rate.wrapping_mul(30))) as i16;
        Ok((-1, step))
    }
}
