//! Original-behavior primitives. This crate never accesses a renderer or AI service.

mod animation;
mod rng;
mod streaming;

pub use animation::{AnimationClip, AnimationFrame, AnimationRecord, AnimationWord};
pub use rng::PsxRng;
pub use streaming::{CellBounds, InvalidLayer, StreamSelection, select_stream_layers};

/// A deterministic development probe, not the game's dream clock or RNG call history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityProbe {
    pub steps: u64,
    pub last_random: u16,
    rng: PsxRng,
}

impl CompatibilityProbe {
    pub const fn new(seed: u32) -> Self {
        Self {
            steps: 0,
            last_random: 0,
            rng: PsxRng::new(seed),
        }
    }

    pub fn step(&mut self) {
        self.last_random = self.rng.next_u15();
        self.steps += 1;
    }

    pub const fn rng_state(&self) -> u32 {
        self.rng.state()
    }
}

impl Default for CompatibilityProbe {
    fn default() -> Self {
        Self::new(1)
    }
}
