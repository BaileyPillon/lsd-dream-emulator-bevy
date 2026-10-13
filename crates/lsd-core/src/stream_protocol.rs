use crate::{MapRequest, SceneGridError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StreamSlotState {
    pub label: u16,
    pub flag: u16,
    pub busy: u16,
    pub ready: i16,
    pub completed: u16,
    pub map_index: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamEvent {
    Notify {
        slot: usize,
        code: u32,
        index: usize,
    },
    Release {
        slot: usize,
    },
    Load {
        slot: usize,
        map: i32,
    },
    Cancel {
        slot: usize,
    },
    Bind {
        slot: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProtocol {
    pub activity: i32,
    pub pending: u16,
    pub finalized: i32,
    pub slots: [StreamSlotState; 7],
}

impl StreamProtocol {
    /// Emits the original virtual calls; the backend supplies their I/O/binding effects.
    pub fn request(&mut self, requests: &[MapRequest]) -> Result<Vec<StreamEvent>, SceneGridError> {
        let mut events = Vec::new();
        for (request_index, request) in requests.iter().enumerate() {
            let physical = self
                .slots
                .iter()
                .position(|slot| slot.label == u16::from(request.label))
                .ok_or(SceneGridError::MissingSlot(i16::from(request.label)))?;
            events.push(StreamEvent::Notify {
                slot: physical,
                code: 6,
                index: request_index,
            });
            let slot = &mut self.slots[physical];
            if slot.ready != 0 {
                events.push(StreamEvent::Release { slot: physical });
            }
            if let Some(map) = request.map_index {
                slot.map_index = map as u16;
                events.push(StreamEvent::Load {
                    slot: physical,
                    map,
                });
                slot.flag = 1;
                self.activity = 1;
            } else if slot.busy != 0 {
                events.push(StreamEvent::Cancel { slot: physical });
                slot.flag = 0;
            }
        }
        self.pending = self.slots.iter().filter(|slot| slot.flag != 0).count() as u16;
        Ok(events)
    }

    pub fn tick(&mut self, code: i32) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        if code != 2 {
            return events;
        }
        for (physical, slot) in self.slots.iter_mut().enumerate() {
            if slot.completed != 0 {
                slot.completed = 0;
                events.push(StreamEvent::Notify {
                    slot: physical,
                    code: 7,
                    index: physical,
                });
            }
            let state = self.activity;
            if state == 1 && slot.flag != 0 {
                if slot.ready != 0 {
                    events.push(StreamEvent::Bind { slot: physical });
                    slot.ready = 2;
                    slot.flag = 0;
                    self.pending = self.pending.wrapping_sub(1);
                    if self.pending == 0 {
                        self.activity = 0;
                        self.finalized = state;
                    }
                } else if slot.busy == 0 {
                    slot.flag = 0;
                }
            }
        }
        events
    }
}
