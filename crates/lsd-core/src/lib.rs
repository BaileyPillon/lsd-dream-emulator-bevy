//! Original-behavior primitives. This crate never accesses a renderer or AI service.

mod animation;
mod animation_player;
mod chunk_collision;
mod chunk_player;
mod collision;
mod collision_cloud;
mod fixed_math;
mod pad;
mod player_controls;
mod rng;
mod streaming;

pub use animation::{AnimationClip, AnimationFrame, AnimationRecord, AnimationWord};
pub use animation_player::{AnimationError, AnimationParent, AnimationPart, AnimationPlayer};
pub use chunk_collision::{
    CollisionChunk, CollisionModel, CollisionPolicy, CollisionTile, WorldContact,
};
pub use chunk_player::{ChunkPlayer, ChunkPlayerStep};
pub use collision::{CollisionFace, CollisionFault, SegmentHit, intersect_model};
pub use collision_cloud::{CollisionBounds, CollisionCloud};
pub use fixed_math::{FixedMatrix, PsxMathTables, transform_number};
pub use pad::{PAD_MASKS, PadState};
pub use player_controls::{
    FloorEffectStep, MotionAxis, MotionRequest, MovementStep, PlayerControls,
};
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
