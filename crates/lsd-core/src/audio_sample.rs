//! SPU sample decoding and pitch arithmetic. Device scheduling is a separate layer.

/// Decoded values from one 16-byte SPU ADPCM block, before pitch interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuAdpcmBlock {
    pub samples: [i16; 28],
    pub flags: u8,
}

impl SpuAdpcmBlock {
    pub fn loop_start(&self) -> bool {
        self.flags & 4 != 0
    }

    pub fn loop_end(&self) -> bool {
        self.flags & 1 != 0
    }

    pub fn loop_repeat(&self) -> bool {
        self.flags & 2 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpuAdpcmFault {
    /// The five documented predictors are implemented. Reserved encodings need
    /// separate hardware evidence rather than an arbitrary modulo operation.
    ReservedFilter(u8),
    ReservedShift(u8),
    PartialBlock(usize),
}

impl std::fmt::Display for SpuAdpcmFault {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReservedFilter(filter) => write!(formatter, "reserved SPU ADPCM filter {filter}"),
            Self::ReservedShift(shift) => {
                write!(formatter, "unverified reserved SPU ADPCM shift {shift}")
            }
            Self::PartialBlock(bytes) => {
                write!(formatter, "partial SPU ADPCM block: {bytes} bytes")
            }
        }
    }
}

impl std::error::Error for SpuAdpcmFault {}

/// Predictor history belongs to the voice, and continues across block boundaries.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpuAdpcmDecoder {
    /// Most recently decoded sample, followed by its predecessor.
    pub history: [i16; 2],
}

impl SpuAdpcmDecoder {
    pub fn decode_block(&mut self, bytes: &[u8; 16]) -> Result<SpuAdpcmBlock, SpuAdpcmFault> {
        const COEFFICIENTS: [[i32; 2]; 5] = [[0, 0], [60, 0], [115, -52], [98, -55], [122, -60]];
        let filter = bytes[0] >> 4;
        let Some(&[recent_factor, older_factor]) = COEFFICIENTS.get(usize::from(filter)) else {
            return Err(SpuAdpcmFault::ReservedFilter(filter));
        };
        let shift = bytes[0] & 15;
        if shift > 12 {
            return Err(SpuAdpcmFault::ReservedShift(shift));
        }
        let mut samples = [0; 28];
        for (index, sample) in samples.iter_mut().enumerate() {
            let nibble = (bytes[2 + index / 2] >> ((index & 1) * 4)) & 15;
            let residual = i32::from((nibble << 4) as i8 >> 4) << 12;
            // Arithmetic shifting is significant for negative predictions.
            let prediction = ((i32::from(self.history[0]) * recent_factor) >> 6)
                + ((i32::from(self.history[1]) * older_factor) >> 6);
            *sample = ((residual >> shift) + prediction)
                .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            self.history = [*sample, self.history[0]];
        }
        Ok(SpuAdpcmBlock {
            samples,
            flags: bytes[1],
        })
    }

    /// Decode the stored blocks in order. This deliberately retains block flags:
    /// a voice's loop/release behavior is not equivalent to concatenating PCM.
    pub fn decode_blocks(&mut self, bytes: &[u8]) -> Result<Vec<SpuAdpcmBlock>, SpuAdpcmFault> {
        if !bytes.len().is_multiple_of(16) {
            return Err(SpuAdpcmFault::PartialBlock(bytes.len() % 16));
        }
        // Validate before changing predictor state on a bounded format error.
        for block in bytes.chunks_exact(16) {
            if block[0] >> 4 > 4 {
                return Err(SpuAdpcmFault::ReservedFilter(block[0] >> 4));
            }
            if block[0] & 15 > 12 {
                return Err(SpuAdpcmFault::ReservedShift(block[0] & 15));
            }
        }
        bytes
            .chunks_exact(16)
            .map(|block| self.decode_block(block.try_into().unwrap()))
            .collect()
    }
}

/// Pitch-register processing before addition to the 12-bit fractional counter.
/// `previous_voice` is the preceding voice's signed output after its envelope.
pub fn spu_pitch_step(pitch: u16, previous_voice: Option<i16>) -> u16 {
    let step = if let Some(previous) = previous_voice {
        let factor = i32::from(previous) + 0x8000;
        ((i32::from(pitch as i16) * factor) >> 15) as u16
    } else {
        pitch
    };
    step.min(0x4000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_nibble_precedes_high_and_prediction_clamps_before_history() {
        let mut decoder = SpuAdpcmDecoder::default();
        let mut bytes = [0; 16];
        bytes[2] = 0x8f;
        let result = decoder.decode_block(&bytes).unwrap();
        assert_eq!(&result.samples[..4], &[-4096, -32768, 0, 0]);
        bytes[0] = 0x20;
        bytes[2..].fill(0x77);
        let result = decoder.decode_block(&bytes).unwrap();
        assert_eq!(&result.samples[..4], &[28672, 32767, 32767, 32767]);
        assert_eq!(decoder.history, [32767; 2]);
    }

    #[test]
    fn negative_prediction_uses_arithmetic_shift_and_retains_history() {
        let mut decoder = SpuAdpcmDecoder { history: [-1, 0] };
        let result = decoder.decode_block(&[0x1c; 16]).unwrap();
        assert_eq!(result.samples[0], -5);
        assert_eq!(decoder.history, [result.samples[27], result.samples[26]]);
    }

    #[test]
    fn reserved_encodings_and_partial_blocks_are_explicit() {
        for shift in 13..16 {
            let mut first = [0x78; 16];
            first[0] = shift;
            assert_eq!(
                SpuAdpcmDecoder::default().decode_block(&first),
                Err(SpuAdpcmFault::ReservedShift(shift))
            );
        }
        let mut decoder = SpuAdpcmDecoder {
            history: [123, -234],
        };
        let before = decoder;
        assert_eq!(
            decoder.decode_block(&[0x50; 16]),
            Err(SpuAdpcmFault::ReservedFilter(5))
        );
        assert_eq!(decoder, before);
        assert_eq!(
            decoder.decode_blocks(&[0; 17]),
            Err(SpuAdpcmFault::PartialBlock(1))
        );
        assert_eq!(decoder, before);
        let mut invalid_tail = [0; 32];
        invalid_tail[16] = 0xf0;
        assert_eq!(
            decoder.decode_blocks(&invalid_tail),
            Err(SpuAdpcmFault::ReservedFilter(15))
        );
        assert_eq!(decoder, before);
    }

    #[test]
    fn pitch_modulation_preserves_signed_register_glitch_before_clipping() {
        assert_eq!(spu_pitch_step(0xffff, None), 0x4000);
        assert_eq!(spu_pitch_step(0x1000, Some(-32768)), 0);
        assert_eq!(spu_pitch_step(0x1000, Some(0)), 0x1000);
        assert_eq!(spu_pitch_step(0x1000, Some(32767)), 0x1fff);
        assert_eq!(spu_pitch_step(0xffff, Some(0)), 0x4000);
        assert_eq!(spu_pitch_step(0xffff, Some(32767)), 0x4000);
        assert_eq!(spu_pitch_step(0x8000, Some(32767)), 1);
    }
}
