//! Original-behavior primitives. This crate never accesses a renderer or AI service.

mod actor_journal;
mod audio_engine;
mod audio_envelope;
pub use audio_engine::{
    AudioEngineTickFault, SpuEngineFault, SpuNoise, SpuSampleMemory, SpuVoice, SpuVoiceEngine,
    SpuVoiceFrame,
};
mod audio_gaussian;
pub use audio_envelope::{SpuEnvelope, SpuEnvelopePhase, SpuVolume};
mod audio_note;
mod audio_output;
pub use audio_gaussian::spu_gaussian;
pub use audio_output::{SpuDevice, SpuOutput, SpuPcmFrame};
mod audio_sample;
pub use audio_sample::{SpuAdpcmBlock, SpuAdpcmDecoder, SpuAdpcmFault, spu_pitch_step};
mod audio_tick;
mod audio_volume;
pub use audio_tick::{AudioRegisterWrite, AudioTickFault, SpuNoiseMode};
pub use audio_volume::{AudioVolumeContext, AudioVolumeFault};
mod audio_voice;
pub use audio_note::{AudioNoteDriver, NoteSetupFault, NoteStart, VoiceNoteDefinition};
pub use audio_voice::{
    AudioVoicePool, AudioVoiceSlot, SpuPitchFault, SpuPitchTable, VoiceAllocation,
};
mod animation;
mod animation_player;
pub use actor_journal::{ActorJournalEvent, ActorJournalHost, actor_journal, actor_link_stage};
mod body_dispatch;
mod body_transform;
mod chunk_collision;
mod chunk_player;
mod collision;
mod collision_cloud;
mod contact_links;
mod dream_day;
mod dream_session;
mod effect;
mod encounters;
pub use encounters::{
    EncounterChunk, EncounterFault, EncounterFrame, EncounterHost, EncounterObject, EncounterSpawn,
    EncounterTables, EncounterVariant, encounter_object_gate,
};
mod fixed_math;
mod frame_phase;
mod frame_subscribers;
mod generation;
mod generation_effects;
mod generation_layers;
mod generation_objects;
mod link_runtime;
mod linked_player;
mod links;
mod map_effect;
mod map_links;
mod model_actor;
pub use model_actor::{ActorAxis, ActorContact, ModelActor};
mod actor_floor;
pub use actor_floor::{ActorFloorQuery, move_actor_on_floor};
mod object_aim;
pub use object_aim::{AimFault, PsxAimTables};
mod entity_behavior;
mod entity_effect;
pub use entity_effect::{EntityEffect, EntityEffectDefinition};
mod entity_body;
pub use entity_behavior::{
    BehaviorContext, BehaviorFault, BehaviorTick, EntityBehavior, EntityInteractionHost,
    EntityScriptHost, EntityTarget, NpcScript, entity_distance, entity_proximity_args,
};
pub use entity_body::{
    BodyPeer, EntityBodyEvent, EntityBodyHost, EntityBodyPlan, entity_body_plan,
};
mod pad;
mod player_controls;
mod rng;
mod scene_collision;
mod scene_grid;
mod scene_keys;
mod scene_motion;
mod stream_grid;
mod stream_protocol;
mod streaming;

pub use animation::{AnimationClip, AnimationFrame, AnimationRecord, AnimationWord};
pub use animation_player::{AnimationError, AnimationParent, AnimationPart, AnimationPlayer};
pub use body_dispatch::{BodyContactState, BodyNotification};
pub use body_transform::BodyGeometryQuery;
pub use chunk_collision::{
    BodyQueryState, BodyWorldFrame, CollisionChunk, CollisionModel, CollisionPolicy, CollisionTile,
    PreparedActorCloud, WorldContact,
};
pub use chunk_player::{ChunkPlayer, ChunkPlayerStep};
pub use collision::{CollisionFace, CollisionFault, SegmentHit, intersect_model};
pub use collision_cloud::{CollisionBounds, CollisionCloud};
pub use contact_links::{
    DreamContactEffects, FloorLinkRequest, FloorLinkResponse, LinkTransformState, WallLinkRequest,
};
pub use dream_day::{
    DayFault, DayStartInputs, DayTables, DreamDay, DreamDayEffects, FlashbackRecord, Mood,
    MoodContributors,
};
pub use dream_session::{DreamSession, DreamSessionHost};
pub use effect::{EffectEvent, EffectFault, EffectPalette, EffectState};
pub use fixed_math::{FixedMatrix, PsxMathTables, transform_number};
mod scene_scale;
pub use frame_phase::{FramePhase, FrameSignal};
pub use frame_subscribers::FrameSubscribers;
pub use generation::GenerationFrame;
pub use scene_scale::{SceneScale, SceneScaleTables};
mod gpu_boxes;
pub use generation::{
    DreamGeneration, GeneratedActor, GenerationActorEntry, GenerationEntry, GenerationFault,
    GenerationSound, GenerationSpawn, GenerationStyle, GenerationTables, choose_music,
    choose_texture, shift_generation_color,
};
pub use generation_effects::generation_effect;
pub use generation_layers::{
    GenerationEffectTables, GenerationLayer, GenerationLayers, VramMove, VramRectangle,
    generation_border_moves,
};
pub use generation_objects::{
    GenerationChild, GenerationObjectTables, GenerationPose, GenerationSprite, GenerationStructure,
};
pub use gpu_boxes::{BoxFramebuffer, GpuBoxPacket, blend_rgb555, rgb555, rgb555_rgba};
pub use link_runtime::{AutoMoveTables, DreamLinkEffects, DreamLinks, rotation_degrees};
pub use linked_player::{LinkedPlayer, LinkedPlayerTick, SceneLinkTables};
pub use links::{
    LinkActionHost, LinkCatalog, LinkContext, LinkError, LinkExecution, LinkKind, LinkMotionRules,
    LinkSet, LinkTransforms, LinkTrigger, SpawnEntry, SpawnLocation, is_facing_link_angle,
    trigger_link_action,
};
pub use map_effect::MapEffect;
pub use map_links::{LinkPalette, MapCompletionCommand, MapLinkCommand, MapLinkPolicy};
pub use pad::{PAD_MASKS, PadState};
pub use player_controls::{
    FloorEffectStep, MotionAxis, MotionRequest, MovementIntent, MovementStep, PlayerControls,
};
pub use rng::PsxRng;
pub use scene_collision::{
    BodyCandidate, BodyVisit, SceneCollision, SceneCollisionError, SceneContact,
};
pub use scene_grid::{
    CellRectangle, SceneGrid, SceneGridError, SceneLocation, ScenePlacement, SceneQuery, SceneSlot,
    SceneSlots,
};
pub use scene_keys::SceneActorKeys;
pub use scene_motion::{SceneMotionReset, reset_scene_motion};
pub use stream_grid::{MapRequest, SlotChoice, SlotSelection, StreamGridTables, StreamRebind};
pub use stream_protocol::{StreamEvent, StreamProtocol, StreamSlotState};
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
