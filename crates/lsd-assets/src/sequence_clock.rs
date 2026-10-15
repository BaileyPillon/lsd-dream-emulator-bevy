//! Original event clock for the SEQ forms imported from the local game.
use crate::formats::{Seq, SeqEvent, SeqEventKind};
use lsd_core::{SequenceAdvanceFault, SequenceClock, SequenceClockFault};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqEventClockFault {
    Clock(SequenceClockFault),
    EmptySequence,
    MissingNextDelta,
    UnsupportedStatus(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeqEventDispatchFault<E> {
    Clock(SeqEventClockFault),
    Handler(E),
}

/// The visitor observes original decoder entry, before that event is handled.
/// Channel synthesis/controller effects belong to the SDK handler; this type
/// advances event positions and repeat timing, and does not itself play music.
#[derive(Debug, Clone)]
pub struct SeqEventClock {
    sequence: Seq,
    pub clock: SequenceClock,
    pub next_event: usize,
    pub loop_count: u16,
    pub loop_limit: i16,
    pub stopped: bool,
    cursor: usize,
}

impl SeqEventClock {
    pub fn new(
        sequence: Seq,
        callback_rate: u32,
        loop_limit: i16,
    ) -> Result<Self, SeqEventClockFault> {
        let first = sequence
            .events
            .first()
            .ok_or(SeqEventClockFault::EmptySequence)?;
        let clock = sequence
            .header
            .initial_clock(callback_rate, first.delta)
            .map_err(SeqEventClockFault::Clock)?;
        let cursor = first.after_delta;
        Ok(Self {
            sequence,
            clock,
            next_event: 0,
            loop_count: 0,
            loop_limit,
            stopped: false,
            cursor,
        })
    }

    pub fn byte_cursor(&self) -> usize {
        self.cursor
    }

    /// SDK loop count zero means unlimited repeats. At a repeat the original
    /// cursor resumes AFTER the first delta, so that initial wait is not replayed.
    pub fn tick(
        &mut self,
        event_limit: usize,
        visitor: impl FnMut(&SeqEvent),
    ) -> Result<i32, SeqEventClockFault> {
        self.tick_inner(event_limit, visitor, |_| {
            Ok::<(), std::convert::Infallible>(())
        })
        .map_err(|error| match error {
            SeqEventDispatchFault::Clock(error) => error,
            SeqEventDispatchFault::Handler(error) => match error {},
        })
    }

    /// Dispatch at the original handler boundary. Note-on reads the following
    /// delta first; program, controller and bend handlers read it afterwards.
    /// A fault preserves the cursor and partial handler state at that boundary.
    pub fn tick_with_handler<E>(
        &mut self,
        event_limit: usize,
        handler: impl FnMut(&SeqEvent) -> Result<(), E>,
    ) -> Result<i32, SeqEventDispatchFault<E>> {
        self.tick_inner(event_limit, |_| {}, handler)
    }

    fn tick_inner<E>(
        &mut self,
        event_limit: usize,
        mut visitor: impl FnMut(&SeqEvent),
        mut handler: impl FnMut(&SeqEvent) -> Result<(), E>,
    ) -> Result<i32, SeqEventDispatchFault<E>> {
        if self.stopped {
            return Ok(0);
        }
        let Self {
            sequence,
            clock,
            next_event,
            loop_count,
            loop_limit,
            stopped,
            cursor,
        } = self;
        clock
            .advance(event_limit, |clock| {
                let event =
                    sequence
                        .events
                        .get(*next_event)
                        .ok_or(SeqEventDispatchFault::Clock(
                            SeqEventClockFault::MissingNextDelta,
                        ))?;
                visitor(event);
                match event.kind {
                    SeqEventKind::Channel { status, .. } => {
                        if !matches!(status >> 4, 9 | 11 | 12 | 14) {
                            return Err(SeqEventDispatchFault::Clock(
                                SeqEventClockFault::UnsupportedStatus(status),
                            ));
                        }
                        if status >> 4 != 9 {
                            handler(event).map_err(SeqEventDispatchFault::Handler)?;
                        }
                        *next_event += 1;
                        let following = sequence.events.get(*next_event).ok_or(
                            SeqEventDispatchFault::Clock(SeqEventClockFault::MissingNextDelta),
                        )?;
                        clock.remaining = following.sdk_delta() as i32;
                        *cursor = following.after_delta;
                        if status >> 4 == 9 {
                            handler(event).map_err(SeqEventDispatchFault::Handler)?;
                        }
                    }
                    SeqEventKind::End => {
                        handler(event).map_err(SeqEventDispatchFault::Handler)?;
                        *loop_count = loop_count.wrapping_add(1);
                        if *loop_limit == 0 || (*loop_count as i16) < *loop_limit {
                            *next_event = 0;
                            *cursor = sequence.events[0].after_delta;
                            clock.remaining = 0;
                        } else {
                            *stopped = true;
                            // The decoder has consumed ff and 2f, but leaves the
                            // final zero byte at the cursor on this original path.
                            *cursor = event.after_delta + 2;
                            clock.remaining = i32::from(clock.step);
                        }
                    }
                }
                Ok(())
            })
            .map_err(|error| match error {
                SequenceAdvanceFault::Clock(error) => {
                    SeqEventDispatchFault::Clock(SeqEventClockFault::Clock(error))
                }
                SequenceAdvanceFault::Event(error) => error,
            })
    }
}
