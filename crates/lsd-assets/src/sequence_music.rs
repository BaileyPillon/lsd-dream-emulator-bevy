use crate::{
    AudioImmediateCommand, SeqEventClock, SeqEventClockFault, SeqEventDispatchFault,
    SeqVoiceDispatch, SeqVoiceFault, SequenceBankBinding,
    formats::{Seq, SeqEventKind},
};
use lsd_core::{AudioNoteDriver, SequencePitchTable};

/// Infinite-repeat music track, as requested by the original game's BGM play.
/// Score/driver ordering, shared SPU placement and callback cadence belong to
/// the caller. This object does not independently flush or own physical voices.
#[derive(Debug, Clone)]
pub struct SeqMusicTrack {
    pub events: SeqEventClock,
    pub channels: SeqVoiceDispatch,
    pub events_dispatched: u64,
    pub notes_allocated: u64,
}

impl SeqMusicTrack {
    pub fn new(seq: Seq, rate: u32, sequence: i16) -> Result<Self, SeqEventClockFault> {
        Ok(Self {
            events: SeqEventClock::new(seq, rate, 0)?,
            channels: SeqVoiceDispatch::new(sequence),
            events_dispatched: 0,
            notes_allocated: 0,
        })
    }

    pub fn tick(
        &mut self,
        event_limit: usize,
        bank: &mut SequenceBankBinding,
        driver: &mut AudioNoteDriver,
        pitch: &SequencePitchTable,
        commands: &mut Vec<AudioImmediateCommand>,
    ) -> Result<i32, SeqEventDispatchFault<SeqVoiceFault>> {
        let Self {
            events,
            channels,
            events_dispatched,
            notes_allocated,
        } = self;
        events.tick_with_handler(event_limit, |event| {
            if let SeqEventKind::Channel { status, data, .. } = event.kind {
                let effect = channels.channel_event(status, data, bank, driver, pitch)?;
                if let Some(note) = effect.note {
                    *notes_allocated += note.voices.len() as u64;
                    commands.extend(
                        note.disable_noise
                            .into_iter()
                            .map(AudioImmediateCommand::DisableNoise),
                    );
                }
            }
            *events_dispatched += 1;
            Ok(())
        })
    }
}
