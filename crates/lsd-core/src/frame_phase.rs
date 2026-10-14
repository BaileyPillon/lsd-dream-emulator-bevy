/// Original frame clock flags and signed phase counter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FramePhase {
    pub phase: i32,
    pub waiting: i32,
    pub finished: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSignal {
    Tick,
    Waiting,
    Finished,
}
impl FrameSignal {
    pub const fn code(self) -> i32 {
        match self {
            Self::Tick => 2,
            Self::Waiting => 3,
            Self::Finished => 4,
        }
    }
}
impl FramePhase {
    pub fn reset(&mut self, phase: i32) {
        *self = Self {
            phase,
            ..Default::default()
        };
    }
    /// The notification is selected once before its parent traversal.
    pub fn advance(&mut self) -> FrameSignal {
        if self.finished != 0 {
            FrameSignal::Finished
        } else if self.waiting != 0 {
            FrameSignal::Waiting
        } else {
            self.phase = self.phase.wrapping_add(1);
            FrameSignal::Tick
        }
    }
}
