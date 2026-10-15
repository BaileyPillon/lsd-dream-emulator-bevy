//! SPU reverb and final main-gain processing. Host playback/resampling is a
//! separate layer and does not change these 44.1 kHz PCM samples.

use crate::{SpuEngineFault, SpuVoiceEngine, SpuVoiceFrame};

const RESAMPLE: [i32; 20] = [
    -1, 2, -10, 35, -103, 266, -616, 1332, -2960, 10246, 10246, -2960, 1332, -616, 266, -103, 35,
    -10, 2, -1,
];

fn saturate(value: i32) -> i16 {
    value.clamp(-32768, 32767) as i16
}

fn inverted_coefficient(value: i16) -> i32 {
    if value == i16::MIN {
        32767
    } else {
        -i32::from(value)
    }
}

fn complement_product(coefficient: i16, sample: i16) -> i32 {
    if coefficient == i16::MIN {
        if sample == i16::MIN {
            0
        } else {
            i32::from(sample) * -65536
        }
    } else {
        i32::from(sample) * (32768 - i32::from(coefficient))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuOutput {
    /// Mirrored rings retain the reference's contiguous FIR windows.
    pub downsample: [[i16; 128]; 2],
    pub upsample: [[i16; 64]; 2],
    pub position: u32,
}

impl Default for SpuOutput {
    fn default() -> Self {
        Self {
            downsample: [[0; 128]; 2],
            upsample: [[0; 64]; 2],
            position: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpuPcmFrame {
    pub stereo: [i16; 2],
    pub reverb: [i16; 2],
}

impl SpuOutput {
    fn address(engine: &SpuVoiceEngine, offset: u32) -> u32 {
        let mut address = engine.reverb_current + (offset & 0x3ffff);
        if address & 0x40000 != 0 {
            address += engine.reverb_base;
        }
        address & 0x3ffff
    }
    fn read(engine: &mut SpuVoiceEngine, raw: u16, extra: i32) -> i16 {
        let offset = (u32::from(raw) << 2).wrapping_add(extra as u32);
        let address = Self::address(engine, offset);
        engine.read_memory(address) as i16
    }
    fn write(engine: &mut SpuVoiceEngine, raw: u16, value: i16) {
        if engine.control & 0x80 != 0 {
            let address = Self::address(engine, u32::from(raw) << 2);
            engine.write_memory(address, value as u16);
        }
    }
    fn channel(&mut self, engine: &mut SpuVoiceEngine, side: usize, input: i16) {
        let registers: [u16; 32] = std::array::from_fn(|index| engine.registers[0xe0 + index]);
        let alpha = registers[2] as i16;
        let reflection = i32::from(registers[7] as i16);
        let input_coefficient = i32::from(registers[30 + side] as i16);
        let destinations = [registers[10 + side], registers[18 + side]];
        let inputs = [registers[16 + side], registers[24 + (side ^ 1)]].map(|address| {
            let delayed = i32::from(Self::read(engine, address, 0));
            saturate(
                (((delayed * reflection) >> 14) + ((i32::from(input) * input_coefficient) >> 14))
                    >> 1,
            )
        });
        let filtered: [i16; 2] = std::array::from_fn(|index| {
            let previous = Self::read(engine, destinations[index], -1);
            saturate(
                (((i32::from(inputs[index]) * i32::from(alpha)) >> 14)
                    + (complement_product(alpha, previous) >> 14))
                    >> 1,
            )
        });
        for index in 0..2 {
            Self::write(engine, destinations[index], filtered[index]);
        }
        let mut combined = 0_i32;
        for index in 0..4 {
            let source = [12, 14, 20, 22][index] + side;
            combined += (i32::from(Self::read(engine, registers[source], 0))
                * i32::from(registers[3 + index] as i16))
                >> 14;
        }
        let delayed_a = Self::read(engine, registers[26 + side].wrapping_sub(registers[0]), 0);
        let delayed_b = Self::read(engine, registers[28 + side].wrapping_sub(registers[1]), 0);
        let feedback = registers[8] as i16;
        let allpass = registers[9] as i16;
        let mix_a = saturate(
            (combined + ((i32::from(delayed_a) * inverted_coefficient(feedback)) >> 14)) >> 1,
        );
        let mix_b = saturate(
            i32::from(delayed_a)
                + ((((i32::from(mix_a) * i32::from(feedback)) >> 14)
                    + ((i32::from(delayed_b) * inverted_coefficient(allpass)) >> 14))
                    >> 1),
        );
        let wet = saturate(i32::from(delayed_b) + ((i32::from(mix_b) * i32::from(allpass)) >> 15));
        Self::write(engine, registers[26 + side], mix_a);
        Self::write(engine, registers[28 + side], mix_b);
        let position = (self.position >> 1) as usize;
        self.upsample[side][position] = wet;
        self.upsample[side][position + 32] = wet;
    }
    /// Reverb continues reading/filtering when its write-enable bit is clear.
    /// Channels share a RAM cursor but are computed on alternating clocks.
    pub fn clock_reverb(&mut self, engine: &mut SpuVoiceEngine, input: [i16; 2]) -> [i16; 2] {
        let position = self.position as usize;
        for (side, value) in input.into_iter().enumerate() {
            self.downsample[side][position] = value;
            self.downsample[side][position + 64] = value;
        }
        let side = if position & 1 != 0 { 0 } else { 1 };
        let start = (position + 64 - 38) & 63;
        let mut sum = 0;
        for (index, coefficient) in RESAMPLE.into_iter().enumerate() {
            sum += coefficient * i32::from(self.downsample[side][start + index * 2]);
        }
        sum += 0x4000 * i32::from(self.downsample[side][start + 19]);
        self.channel(engine, side, saturate(sum >> 15));
        if side == 1 {
            engine.reverb_current = (engine.reverb_current + 1) & 0x3ffff;
            if engine.reverb_current == 0 {
                engine.reverb_current = engine.reverb_base;
            }
        }
        let start = (((position >> 1) + 32) - 19) & 31;
        let result = std::array::from_fn(|channel| {
            if channel == side {
                let sum: i32 = RESAMPLE
                    .into_iter()
                    .enumerate()
                    .map(|(index, coefficient)| {
                        coefficient * i32::from(self.upsample[channel][start + index])
                    })
                    .sum();
                saturate(sum >> 14)
            } else {
                self.upsample[channel][start + 9]
            }
        });
        self.position = (self.position + 1) & 63;
        result
    }
    /// Complete the output stage once for the corresponding voice clock.
    /// Host front-end gain is deliberately outside the SPU PCM calculation.
    pub fn finish_frame(
        &mut self,
        engine: &mut SpuVoiceEngine,
        frame: SpuVoiceFrame,
    ) -> SpuPcmFrame {
        let reverb = self.clock_reverb(engine, frame.reverb_input);
        let stereo = std::array::from_fn(|side| {
            let wet = (i32::from(reverb[side]) * i32::from(engine.reverb_volume[side])) >> 15;
            let mixed = saturate(frame.dry[side] + wet);
            saturate((i32::from(mixed) * i32::from(engine.main_volume[side].current())) >> 15)
        });
        for volume in &mut engine.main_volume {
            volume.clock();
        }
        SpuPcmFrame { stereo, reverb }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SpuDevice {
    pub engine: SpuVoiceEngine,
    pub output: SpuOutput,
}

impl SpuDevice {
    pub fn clock_sample(&mut self, cd_input: [i16; 2]) -> Result<SpuPcmFrame, SpuEngineFault> {
        let frame = self.engine.clock_voices(cd_input)?;
        Ok(self.output.finish_frame(&mut self.engine, frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_gain_clocks_after_output_and_saturates_before_signed_gain() {
        let mut engine = SpuVoiceEngine::default();
        engine.main_volume[0].control = 0x3fff;
        engine.main_volume[1].control = 0x4000;
        let mut output = SpuOutput::default();
        let frame = SpuVoiceFrame {
            dry: [90000, -90000],
            reverb_input: [0; 2],
        };
        assert_eq!(output.finish_frame(&mut engine, frame).stereo, [0; 2]);
        assert_eq!(
            output.finish_frame(&mut engine, frame).stereo,
            [32765, 32767]
        );
    }
}
