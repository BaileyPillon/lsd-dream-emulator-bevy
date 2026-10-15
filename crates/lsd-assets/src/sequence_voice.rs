use crate::SequenceBankBinding;
use lsd_core::{
    AudioNoteDriver, SequenceChannelRequest, SequenceNoteRequest, SequenceNoteStart,
    SequencePitchTable, SequenceVoiceFault,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeqVoiceDispatch {
    pub sequence: i16,
    pub current_channel: u8,
    pub pans: [u8; 16],
    pub programs: [u8; 16],
    pub channel_volumes: [i16; 16],
    pub sequence_volume: [u16; 2],
    pub last_velocity: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqVoiceFault {
    Voice(SequenceVoiceFault),
    MissingProgram,
    InvalidMidiData,
    UnsupportedController(u8),
    UnsupportedStatus(u8),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeqVoiceEffect {
    pub note: Option<SequenceNoteStart>,
    pub voices_changed: u8,
}

impl SeqVoiceDispatch {
    pub fn new(sequence: i16) -> Self {
        Self {
            sequence,
            current_channel: 0,
            pans: [64; 16],
            programs: std::array::from_fn(|i| i as u8),
            channel_volumes: [127; 16],
            sequence_volume: [127; 2],
            last_velocity: 127,
        }
    }

    /// Recovered channel forms used by the local original SEQ corpus. E0 uses
    /// its second data byte; the SDK does not combine the two into a 14-bit bend.
    pub fn channel_event(
        &mut self,
        status: u8,
        data: [u8; 2],
        bank: &mut SequenceBankBinding,
        driver: &mut AudioNoteDriver,
        pitch: &SequencePitchTable,
    ) -> Result<SeqVoiceEffect, SeqVoiceFault> {
        if data.iter().any(|v| *v >= 128) {
            return Err(SeqVoiceFault::InvalidMidiData);
        }
        self.current_channel = status & 15;
        let channel = usize::from(self.current_channel);
        let id = self.programs[channel];
        let mut effect = SeqVoiceEffect::default();
        match status >> 4 {
            9 => {
                if self.sequence_volume[0] == 0 {
                    return Ok(effect);
                }
                let program = bank.program(id).ok_or(SeqVoiceFault::MissingProgram)?;
                if data[1] == 0 {
                    effect.voices_changed = driver
                        .sequence_key_off(
                            self.sequence,
                            program.bank,
                            program.program,
                            u16::from(data[0]),
                        )
                        .map_err(SeqVoiceFault::Voice)?;
                } else {
                    let velocity = (i32::from(data[1]) * i32::from(self.channel_volumes[channel])
                        / 127) as u16;
                    effect.note = Some(
                        driver
                            .sequence_key_on(
                                program,
                                pitch,
                                SequenceNoteRequest {
                                    sequence: self.sequence,
                                    key: u16::from(data[0]),
                                    velocity,
                                    pan: u16::from(self.pans[channel]),
                                    channel_volume: self.channel_volumes[channel],
                                    sequence_volume: self.sequence_volume,
                                },
                            )
                            .map_err(SeqVoiceFault::Voice)?,
                    );
                    self.last_velocity = u16::from(data[1]);
                }
            }
            11 => {
                let (volume, pan) = match data[0] {
                    7 => (u16::from(data[1]), u16::from(self.pans[channel])),
                    10 => (self.channel_volumes[channel] as u16, u16::from(data[1])),
                    11 => {
                        bank.set_program_volume(id, data[1])
                            .map_err(|_| SeqVoiceFault::MissingProgram)?;
                        (
                            self.channel_volumes[channel] as u16,
                            u16::from(self.pans[channel]),
                        )
                    }
                    controller => return Err(SeqVoiceFault::UnsupportedController(controller)),
                };
                effect.voices_changed = driver
                    .sequence_channel_volume(
                        bank.volume_definition(),
                        SequenceChannelRequest {
                            sequence: self.sequence,
                            program: i16::from(id),
                            volume,
                            pan,
                            sequence_volume: self.sequence_volume,
                        },
                    )
                    .map_err(SeqVoiceFault::Voice)?;
                if data[0] == 7 {
                    self.channel_volumes[channel] = i16::from(data[1]);
                }
                if data[0] == 10 {
                    self.pans[channel] = data[1];
                }
            }
            12 => {
                self.programs[channel] = data[0];
            }
            14 => {
                let program = bank.program(id).ok_or(SeqVoiceFault::MissingProgram)?;
                effect.voices_changed = driver
                    .sequence_pitch_bend(program, pitch, self.sequence, u16::from(data[1]))
                    .map_err(SeqVoiceFault::Voice)?;
            }
            _ => return Err(SeqVoiceFault::UnsupportedStatus(status)),
        }
        Ok(effect)
    }
}
