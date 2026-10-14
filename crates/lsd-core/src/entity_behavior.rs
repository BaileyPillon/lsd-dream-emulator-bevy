//! Original entity lifecycle and three-slot behavior/sound contexts.
use crate::{
    ActorAxis, AnimationClip, AnimationError, FixedMatrix, GenerationSound, ModelActor, PsxRng,
};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorFault {
    Division,
    Animation(AnimationError),
    PendingCallback,
    MissingMotionServices,
    Collision(crate::CollisionFault),
    Journal(crate::DayFault),
    Effect(crate::EffectFault),
    MissingEffectServices,
    Aim(crate::AimFault),
    MissingAimServices,
    MissingPlayerServices,
    Scene(crate::SceneCollisionError),
}
impl fmt::Display for BehaviorFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported entity behavior: {self:?}")
    }
}
impl std::error::Error for BehaviorFault {}
impl From<AnimationError> for BehaviorFault {
    fn from(value: AnimationError) -> Self {
        Self::Animation(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BehaviorContext {
    pub words: [i32; 21],
}
impl BehaviorContext {
    /// Owner and callback words are typed identities, replacing native pointers.
    pub fn initialize(&mut self, state: i32, owner: i32, callback: i32) -> bool {
        if self.words[0] != 0 {
            return false;
        }
        self.words[0] = state;
        self.words[2] = owner;
        self.words[3] = callback;
        for index in [6, 11, 16] {
            self.words[index] = -1;
        }
        self.words[1] = 0;
        self.words[5] = 10;
        true
    }
    pub fn stop(&mut self, sound: &mut impl GenerationSound) {
        for index in [6, 11, 16] {
            if self.words[index] >= 0 {
                self.words[index] = sound.stop_note(self.words[index]);
            }
        }
        self.words[0] = 0;
    }
    pub fn update(
        &mut self,
        sound: &mut impl GenerationSound,
        callback: impl FnOnce(&mut Self) -> Result<(), BehaviorFault>,
    ) -> Result<(), BehaviorFault> {
        self.update_with_sound(sound, |context, _sound| callback(context))
    }
    pub fn update_with_sound<S: GenerationSound>(
        &mut self,
        sound: &mut S,
        callback: impl FnOnce(&mut Self, &mut S) -> Result<(), BehaviorFault>,
    ) -> Result<(), BehaviorFault> {
        if self.words[0] <= 0 {
            return Ok(());
        }
        for index in [6, 11, 16] {
            self.words[index + 1] = -1;
            self.words[index + 2] = 0;
            self.words[index + 3] = 127;
            self.words[index + 4] = 64;
        }
        self.words[4] = 0;
        if self.words[3] != 0 {
            callback(self, sound)?;
        }
        let motion = self.words[4];
        if motion >= 0 {
            for index in [6, 11, 16] {
                let id = self.words[index + 1];
                if id >= 0 {
                    if self.words[index] >= 0 {
                        sound.stop_note(self.words[index]);
                    }
                    sound.volume_offset(self.words[index + 2]);
                    let period = self.words[index + 3];
                    let counter = self.words[index + 4];
                    let divisor = self.words[5];
                    self.words[index] = sound.play_note(
                        id.wrapping_shl(4),
                        period.wrapping_sub(
                            period
                                .checked_div(divisor)
                                .ok_or(BehaviorFault::Division)?
                                .wrapping_mul(motion),
                        ),
                        counter.wrapping_sub(
                            counter
                                .checked_div(divisor)
                                .ok_or(BehaviorFault::Division)?
                                .wrapping_mul(motion),
                        ),
                    );
                } else if id == -2 && self.words[index] >= 0 {
                    sound.stop_note(self.words[index]);
                }
            }
        }
        self.words[1] = self.words[1].wrapping_add(1);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcScript {
    None,
    Futon,
    Television,
    Drumstick,
    Yokai,
    Sailboat,
    Balloon,
    FishGiant,
    Halo,
    SiblingsFlock,
    BedBoneDead,
    KickingMan,
    PrincePrincess,
    GrayMan,
    Bear,
    Rabbit,
    Peacock,
    Dog,
    Gargoyle,
    DemonKing,
    FerrisWheel,
    BellStriker,
    Elephant,
    Minotaur,
    GiantMinotaur,
    Boatman,
    TeaDoll,
    Buddha,
    OxCart,
    OperaSinger,
    MarchingBand,
    Gears,
    Horse,
    Soldier,
    GiantTurtle,
    GiantMaiko,
    Fox,
    StandingPenguin,
    SlidingPenguin,
    Astronaut,
    Rainbow,
    BoatBird,
    Tennyo,
    Bird,
    Rocket,
    Ufo,
    Shark,
    Fish,
    Mirage,
    GargoylePit,
    Mushroom,
    HumanFaceFlower,
    GiantLips,
    Whale,
    HopscotchGirl,
    Corpse,
    Pending(u32),
}
impl NpcScript {
    pub fn from_reference_address(address: Option<u32>) -> Self {
        match address {
            None => Self::None,
            Some(0x8005_e480) => Self::Futon,
            Some(0x8006_3e68) => Self::Television,
            Some(0x8005_ed10) => Self::Drumstick,
            Some(0x8006_41c0) => Self::Yokai,
            Some(0x8005_fa64) => Self::Sailboat,
            Some(0x8005_fec8) => Self::Balloon,
            Some(0x8006_51d0) => Self::FishGiant,
            Some(0x8005_ef20) => Self::Halo,
            Some(0x8005_f6d4) => Self::SiblingsFlock,
            Some(0x8006_4b14) => Self::BedBoneDead,
            Some(0x8006_1c04) => Self::KickingMan,
            Some(0x8006_07f8) => Self::PrincePrincess,
            Some(0x8006_4618) => Self::GrayMan,
            Some(0x8006_04dc) => Self::Bear,
            Some(0x8006_05d0) => Self::Rabbit,
            Some(0x8005_f1a8) => Self::Peacock,
            Some(0x8006_21a8) => Self::Dog,
            Some(0x8005_e3c4) => Self::Gargoyle,
            Some(0x8005_f970) => Self::DemonKing,
            Some(0x8006_090c) => Self::FerrisWheel,
            Some(0x8005_ebb4) => Self::BellStriker,
            Some(0x8005_e694) => Self::Elephant,
            Some(0x8005_e6f0) => Self::Minotaur,
            Some(0x8005_e7a8) => Self::GiantMinotaur,
            Some(0x8005_ec98) => Self::Boatman,
            Some(0x8005_ef54) => Self::TeaDoll,
            Some(0x8005_eff4) => Self::Buddha,
            Some(0x8005_f0d8) => Self::OxCart,
            Some(0x8005_fef8) => Self::OperaSinger,
            Some(0x8006_03c4) => Self::MarchingBand,
            Some(0x8006_23e8) => Self::Gears,
            Some(0x8005_f544) => Self::Horse,
            Some(0x8006_0148) => Self::Soldier,
            Some(0x8005_fa94) => Self::GiantTurtle,
            Some(0x8005_e7f8) => Self::GiantMaiko,
            Some(0x8005_ed30) => Self::Fox,
            Some(0x8006_0d80) => Self::StandingPenguin,
            Some(0x8006_0f38) => Self::SlidingPenguin,
            Some(0x8005_e160) => Self::Astronaut,
            Some(0x8005_f708) => Self::Rainbow,
            Some(0x8005_f800) => Self::BoatBird,
            Some(0x8005_f1d4) => Self::Tennyo,
            Some(0x8005_f368) => Self::Bird,
            Some(0x8005_f454) => Self::Rocket,
            Some(0x8005_f608) => Self::Ufo,
            Some(0x8005_fb6c) => Self::Shark,
            Some(0x8005_fc58) => Self::Fish,
            Some(0x8005_fdfc) => Self::Mirage,
            Some(0x8005_e4d0) => Self::GargoylePit,
            Some(0x8005_ff7c) => Self::Mushroom,
            Some(0x8006_0800) => Self::HumanFaceFlower,
            Some(0x8006_0a4c) => Self::GiantLips,
            Some(0x8006_0cf0) => Self::Whale,
            Some(0x8006_1070) => Self::HopscotchGirl,
            Some(0x8006_1158) => Self::Corpse,
            Some(at) => Self::Pending(at),
        }
    }
    pub fn pending(self) -> bool {
        matches!(self, Self::Pending(_))
    }
}
fn animation_duration(actor: &ModelActor, clips: &[AnimationClip]) -> Result<i32, AnimationError> {
    let index = actor
        .animation
        .selected_clip
        .ok_or(AnimationError::InvalidClip)?;
    Ok(clips
        .get(index)
        .ok_or(AnimationError::InvalidClip)?
        .frames
        .len() as i32)
}
pub fn entity_distance(position: [i32; 3], target: [i32; 3]) -> i32 {
    position[0]
        .wrapping_sub(target[0])
        .wrapping_abs()
        .wrapping_add(position[2].wrapping_sub(target[2]).wrapping_abs())
}

/// The scene/transform backend resolves these requests. This policy interface
/// alone does not establish native target geometry or body dispatch.
pub trait EntityInteractionHost {
    fn proximity(&mut self, position: [i32; 3], forward: i32, radius: i32) -> bool;
    fn facing_target(&mut self, position: [i32; 3], distance: i32) -> bool;
    fn aim_at_target(&mut self, rng: &mut PsxRng);
    fn aim_at_target_actor(&mut self, _actor: &mut ModelActor, rng: &mut PsxRng) {
        self.aim_at_target(rng);
    }
    fn notify(&mut self, rng: &mut PsxRng, code: i32);
    fn notify_actor(&mut self, _actor: &mut ModelActor, rng: &mut PsxRng, code: i32) {
        self.notify(rng, code);
    }
}
/// Script services use the original rotation math and synchronous notification order.
pub trait EntityScriptHost {
    fn day_number(&mut self) -> Result<i32, BehaviorFault> {
        Err(BehaviorFault::MissingPlayerServices)
    }
    fn actor_proximity(
        &mut self,
        _actor: &ModelActor,
        _forward: i32,
        _radius: i32,
    ) -> Result<bool, BehaviorFault> {
        Err(BehaviorFault::MissingPlayerServices)
    }
    fn dream_color(&mut self) -> Result<i32, BehaviorFault> {
        Err(BehaviorFault::MissingPlayerServices)
    }
    fn player_local_position(&mut self) -> Result<[i32; 3], BehaviorFault> {
        Err(BehaviorFault::MissingPlayerServices)
    }
    fn actor_floor_query(
        &mut self,
        _position: [i32; 3],
    ) -> Result<crate::ActorFloorQuery, BehaviorFault> {
        Err(BehaviorFault::MissingMotionServices)
    }
    /// Negative floor signals are local virtual calls, not parent journal events.
    fn actor_floor_signal(
        &mut self,
        _actor: &mut ModelActor,
        _code: i32,
    ) -> Result<(), BehaviorFault> {
        Ok(())
    }
    fn move_player_local(&mut self, _axis: ActorAxis, _amount: i32) -> Result<(), BehaviorFault> {
        Err(BehaviorFault::MissingPlayerServices)
    }
    fn player_action_pressed(&mut self) -> Result<bool, BehaviorFault> {
        Err(BehaviorFault::MissingPlayerServices)
    }
    fn clear_player_callbacks(&mut self, _with_view: bool) -> Result<(), BehaviorFault> {
        Err(BehaviorFault::MissingPlayerServices)
    }
    fn face_player(&mut self, _actor: &mut ModelActor) -> Result<(), BehaviorFault> {
        Err(BehaviorFault::MissingAimServices)
    }
    fn movement_matrix(&mut self, angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault>;
    fn notify(&mut self, rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault>;
    fn notify_actor(
        &mut self,
        _actor: &mut ModelActor,
        rng: &mut PsxRng,
        code: i32,
    ) -> Result<(), BehaviorFault> {
        self.notify(rng, code)
    }
    fn start_entity_effect(
        &mut self,
        _effect: &mut Option<crate::EntityEffect>,
        _color_step: i32,
        _variant: i32,
        _mode: i32,
    ) -> Result<bool, BehaviorFault> {
        Err(BehaviorFault::MissingEffectServices)
    }
    fn reset_flashback_count(&mut self, _rng: &mut PsxRng) -> Result<(), BehaviorFault> {
        Err(BehaviorFault::MissingEffectServices)
    }
}
struct UnconnectedScriptHost;
impl EntityScriptHost for UnconnectedScriptHost {
    fn movement_matrix(&mut self, _angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault> {
        Err(BehaviorFault::MissingMotionServices)
    }
    fn notify(&mut self, _rng: &mut PsxRng, _code: i32) -> Result<(), BehaviorFault> {
        Err(BehaviorFault::MissingMotionServices)
    }
}
fn trigger_ratio(
    actor: &ModelActor,
    context: &BehaviorContext,
    input: BehaviorTick,
) -> Result<i32, BehaviorFault> {
    let distance = entity_distance(actor.pose.position, input.target);
    let range = i32::from(input.proximity).wrapping_shl(11);
    if range < distance {
        return Ok(-1);
    }
    distance
        .checked_div(
            range
                .checked_div(context.words[5])
                .ok_or(BehaviorFault::Division)?,
        )
        .ok_or(BehaviorFault::Division)
}
/// Original unlock-height adjustment and proximity argument widths.
pub fn entity_proximity_args(
    fields: [i8; 12],
    mut position: [i32; 3],
    forward: i32,
    radius: i32,
) -> ([i32; 3], i32, i32) {
    if (fields[2] as u8).wrapping_add(9) < 9 {
        position[1] = position[1].wrapping_add(i32::from(fields[2]).wrapping_shl(10));
    }
    let radius = if radius < 0 {
        2048 / radius.wrapping_neg()
    } else {
        radius.wrapping_shl(11)
    };
    (position, forward.wrapping_shl(11), radius)
}
fn move_local_with_host(
    actor: &mut ModelActor,
    axis: ActorAxis,
    amount: i32,
    host: &mut impl EntityScriptHost,
) -> Result<(), BehaviorFault> {
    let matrix = host.movement_matrix(actor.pose.angles)?;
    actor.move_local(axis, amount, matrix);
    Ok(())
}
fn rotate_yaw(actor: &mut ModelActor, yaw: i16) -> Result<(), BehaviorFault> {
    actor
        .rotate_packet(
            [
                0x0001_0000,
                0x0001_0000 | u32::from(yaw as u16),
                0x0001_0000,
            ],
            false,
        )
        .map_err(|_| BehaviorFault::Division)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityBehavior {
    pub entity: u8,
    pub script: NpcScript,
    pub context: BehaviorContext,
    pub ticks: i32,
    pub state: i32,
    pub triggered: i32,
    pub link_enabled: i32,
    pub active: i32,
    pub effect: Option<crate::EntityEffect>,
}
impl EntityBehavior {
    pub fn construct(
        entity: u8,
        script: NpcScript,
        fields: [i8; 12],
        actor: &mut ModelActor,
        sound: &mut impl GenerationSound,
    ) -> Self {
        let mut out = Self::construct_dormant(entity, script, fields, actor, sound);
        out.start(fields, actor);
        out
    }
    /// entity_create constructs a disabled actor without binding its scene/clock.
    pub fn construct_dormant(
        entity: u8,
        script: NpcScript,
        fields: [i8; 12],
        actor: &mut ModelActor,
        sound: &mut impl GenerationSound,
    ) -> Self {
        let mut out = Self {
            entity,
            script,
            context: BehaviorContext::default(),
            ticks: 0,
            state: 0,
            triggered: 0,
            link_enabled: 0,
            active: 0,
            effect: None,
        };
        if (fields[2] as u8).wrapping_sub(1) < 9 {
            for part in &mut actor.animation.parts {
                part.attributes = (part.attributes & !0x38) | 8;
            }
        }
        actor.action_kind = 66;
        out.disable_link(actor, sound);
        out
    }
    /// Apply entity_start policy after the inherited actor has been attached.
    pub fn start(&mut self, fields: [i8; 12], actor: &mut ModelActor) {
        if fields[3] == 0 {
            self.enable_link(actor);
            if fields[11] == 0 {
                self.start_behavior(actor);
            }
        }
    }
    pub fn start_behavior(&mut self, actor: &mut ModelActor) {
        self.context.initialize(
            i32::from(self.entity) + 1,
            1,
            i32::from(self.script != NpcScript::None),
        );
        actor.animation.enabled = true;
        actor.animation.action_enabled = true;
        self.ticks = 0;
        self.active = 1;
    }
    pub fn stop_behavior(&mut self, actor: &mut ModelActor, sound: &mut impl GenerationSound) {
        self.context.stop(sound);
        actor.animation.enabled = false;
        actor.animation.action_enabled = false;
        self.active = 0;
    }
    pub fn enable_link(&mut self, actor: &mut ModelActor) {
        actor.set_hidden(false);
        self.link_enabled = 1;
        actor.animation.ticks = 0;
    }
    pub fn disable_link(&mut self, actor: &mut ModelActor, sound: &mut impl GenerationSound) {
        actor.set_hidden(true);
        self.stop_behavior(actor, sound);
        self.triggered = 0;
        self.link_enabled = 0;
    }
    fn proximity(
        &self,
        fields: [i8; 12],
        position: [i32; 3],
        range: i32,
        radius: i32,
        host: &mut impl EntityInteractionHost,
    ) -> bool {
        let (position, forward, radius) = entity_proximity_args(fields, position, range, radius);
        host.proximity(position, forward, radius)
    }
    /// Entity-update policy before the inherited actor/body update.
    pub fn update_interaction(
        &mut self,
        actor: &mut ModelActor,
        fields: [i8; 12],
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
        host: &mut impl EntityInteractionHost,
    ) {
        let position = actor.pose.position;
        if self.link_enabled == 0 && self.state != 1 {
            let kind = fields[3];
            let mut enable = false;
            if kind != 0 {
                let random = if kind == 4 {
                    true
                } else if fields[5] != 0 {
                    let near = self.proximity(
                        fields,
                        position,
                        i32::from(fields[5]),
                        i32::from(fields[9]),
                        host,
                    );
                    if near && kind == 1 || !near && kind == 2 {
                        enable = true;
                    }
                    near && kind == 3
                } else {
                    false
                };
                if random && rng.next_u15() & 127 == 0 {
                    enable = true;
                }
            }
            if enable {
                self.enable_link(actor);
            }
        }
        if self.link_enabled != 0 {
            if fields[7] < 0
                && fields[8] != 0
                && host.facing_target(position, i32::from(fields[8]).wrapping_shl(9))
            {
                host.notify_actor(actor, rng, 10);
            }
            let kind = fields[4] as u8;
            let disable = if kind == 0 || kind == 3 {
                false
            } else if kind >= 10 {
                actor.animation.ticks as i32 == i32::from(kind) * 15
            } else if fields[5] != 0 {
                let near = self.proximity(
                    fields,
                    position,
                    i32::from(fields[5]),
                    i32::from(fields[9]),
                    host,
                );
                near && kind == 1 || !near && kind == 2
            } else {
                false
            };
            if disable {
                self.disable_link(actor, sound);
            }
        }
        if self.link_enabled != 0
            && self.active == 0
            && self.state != 1
            && fields[11] != 0
            && self.proximity(
                fields,
                position,
                i32::from(fields[11]).abs(),
                i32::from(fields[9]),
                host,
            )
        {
            self.start_behavior(actor);
        }
        if self.link_enabled != 0
            && self.active != 0
            && fields[11] < 0
            && !self.proximity(
                fields,
                position,
                i32::from(fields[11]).abs(),
                i32::from(fields[9]),
                host,
            )
        {
            self.stop_behavior(actor, sound);
        }
        if self.link_enabled != 0 {
            if self.triggered == 0
                && self.proximity(
                    fields,
                    position,
                    i32::from(fields[6]).abs(),
                    i32::from(fields[9]),
                    host,
                )
            {
                host.notify_actor(actor, rng, 9);
                self.triggered = 1;
            }
            if fields[6] < 0 {
                host.aim_at_target_actor(actor, rng);
            }
        }
    }
    pub fn tick(
        &mut self,
        actor: &mut ModelActor,
        clips: &[AnimationClip],
        input: BehaviorTick,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
    ) -> Result<(), BehaviorFault> {
        self.tick_with_services(actor, clips, input, rng, sound, &mut UnconnectedScriptHost)
    }
    pub fn tick_with_services(
        &mut self,
        actor: &mut ModelActor,
        clips: &[AnimationClip],
        input: BehaviorTick,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
        host: &mut impl EntityScriptHost,
    ) -> Result<(), BehaviorFault> {
        if self.script.pending() {
            return Err(BehaviorFault::PendingCallback);
        }
        actor.animation.ticks = actor.animation.ticks.wrapping_add(1);
        if actor.animation.action_enabled {
            let ticks = self.ticks;
            let script = self.script;
            let triggered = &mut self.triggered;
            let link_enabled = &mut self.link_enabled;
            let active = &mut self.active;
            let effect = &mut self.effect;
            let state = &mut self.state;
            let mutable_ticks = &mut self.ticks;
            self.context.update_with_sound(sound, |context, sound| {
                match script {
                    NpcScript::None | NpcScript::PrincePrincess => {}
                    NpcScript::Futon | NpcScript::Television => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if context.words[1] == 0 {
                            if script == NpcScript::Futon {
                                context.words[7] = 23;
                            } else {
                                context.words[7] = if rng.next_u15() & 1 != 0 { 2 } else { 1 };
                                context.words[8] = 2;
                            }
                        }
                    }
                    NpcScript::Drumstick => {
                        if ticks == 0 {
                            context.words[4] = 0;
                            context.words[7] = 15;
                        }
                    }
                    NpcScript::Yokai => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if context.words[1] % 10 == 0 {
                            context.words[7] = 3;
                        }
                        let variant = actor
                            .animation
                            .selected_clip
                            .ok_or(AnimationError::InvalidClip)?;
                        let duration = clips
                            .get(variant)
                            .ok_or(AnimationError::InvalidClip)?
                            .frames
                            .len();
                        if ticks == duration as i32 {
                            actor.animation.select_clip(clips, 1)?;
                        }
                        if actor.animation.selected_clip == Some(1) {
                            let matrix = host.movement_matrix(actor.pose.angles)?;
                            actor.move_local(ActorAxis::Z, -128, matrix);
                            host.notify_actor(actor, rng, 6)?;
                        }
                    }
                    NpcScript::Sailboat | NpcScript::Balloon => {
                        let (axis, amount) = if script == NpcScript::Sailboat {
                            (ActorAxis::Z, -30)
                        } else {
                            (ActorAxis::Y, -90)
                        };
                        let matrix = host.movement_matrix(actor.pose.angles)?;
                        actor.move_local(axis, amount, matrix);
                    }
                    NpcScript::FishGiant | NpcScript::SiblingsFlock | NpcScript::Halo => {
                        let packet = if script == NpcScript::Halo {
                            0x0002_0001
                        } else {
                            0x0001_0006
                        };
                        actor
                            .scale_packet([packet; 3], true)
                            .map_err(|_| BehaviorFault::Division)?;
                    }
                    NpcScript::BedBoneDead => actor.set_hidden(!rng.next_u15().is_multiple_of(20)),
                    NpcScript::KickingMan => {
                        if actor.animation.frame_index == 30 {
                            context.words[7] = 18;
                            context.words[4] = 0;
                            context.words[8] = -1;
                        }
                    }
                    NpcScript::GrayMan => {
                        if *triggered != 0 && host.start_entity_effect(effect, 10, 7, 0)? {
                            actor.set_hidden(true);
                            context.stop(sound);
                            actor.animation.enabled = false;
                            actor.animation.action_enabled = false;
                            *active = 0;
                            *triggered = 0;
                            *link_enabled = 0;
                            host.reset_flashback_count(rng)?;
                        }
                        let matrix = host.movement_matrix(actor.pose.angles)?;
                        actor.move_local(ActorAxis::Z, -30, matrix);
                        host.notify_actor(actor, rng, 6)?;
                    }
                    NpcScript::Peacock => host.face_player(actor)?,
                    NpcScript::FerrisWheel => {
                        if *triggered != 0 {
                            match *state {
                                0 => {
                                    *state = 12;
                                    *mutable_ticks = 0;
                                }
                                12 if ticks < 30 => {
                                    if host.player_action_pressed()? {
                                        host.clear_player_callbacks(false)?;
                                        *mutable_ticks = 0;
                                        *state = 11;
                                    }
                                }
                                12 => {
                                    host.notify_actor(actor, rng, 11)?;
                                    *state = 10;
                                }
                                11 if ticks == 100 => host.notify_actor(actor, rng, 12)?,
                                11 => host.move_player_local(ActorAxis::Y, -100)?,
                                _ => {}
                            }
                        }
                    }
                    NpcScript::BellStriker => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if context.words[1] == 0 {
                            context.words[7] = 12;
                            *state = state.wrapping_add(1);
                        } else if context.words[1]
                            >= animation_duration(actor, clips)?.wrapping_sub(1)
                        {
                            context.words[1] = -1;
                        }
                        if *state == 36 && rng.next_u15().is_multiple_of(3) {
                            host.notify_actor(actor, rng, 11)?;
                        }
                    }
                    NpcScript::Elephant => {
                        actor
                            .scale_packet([0x0001_0002; 3], true)
                            .map_err(|_| BehaviorFault::Division)?;
                        actor.translate([0, -64, 0], false);
                    }
                    NpcScript::Minotaur | NpcScript::OxCart => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        let half = animation_duration(actor, clips)? / 2;
                        let remainder = context.words[1]
                            .checked_rem(half)
                            .ok_or(BehaviorFault::Division)?;
                        if remainder == 0 {
                            context.words[7] = 10;
                        } else if script == NpcScript::OxCart && remainder == 3 {
                            context.words[12] = 13;
                        }
                        let matrix = host.movement_matrix(actor.pose.angles)?;
                        actor.move_local(ActorAxis::Z, -30, matrix);
                        if script == NpcScript::OxCart {
                            host.notify_actor(actor, rng, 6)?;
                        }
                    }
                    NpcScript::GiantMinotaur => {
                        context.words[4] = 0;
                        if context.words[1] == 0 {
                            for index in [7, 12, 17] {
                                context.words[index] = 11;
                            }
                        }
                        let matrix = host.movement_matrix(actor.pose.angles)?;
                        actor.move_local(ActorAxis::Z, -30, matrix);
                    }
                    NpcScript::Boatman | NpcScript::TeaDoll | NpcScript::Buddha => {
                        if script == NpcScript::Buddha
                            && ticks == 0
                            && rng.next_u15().is_multiple_of(7)
                        {
                            actor
                                .scale_packet([0x0001_0001, 0x0001_0002, 0x0001_0001], true)
                                .map_err(|_| BehaviorFault::Division)?;
                        }
                        let note = match script {
                            NpcScript::Boatman if actor.animation.frame_index == 10 => Some(13),
                            NpcScript::TeaDoll if context.words[1] % 10 == 0 => Some(17),
                            NpcScript::Buddha if context.words[1] & 3 == 0 => Some(28),
                            _ => None,
                        };
                        // Boatman and Tea Doll update distance even on silent ticks.
                        if script != NpcScript::Buddha || note.is_some() {
                            context.words[4] = trigger_ratio(actor, context, input)?;
                        }
                        if let Some(note) = note {
                            context.words[7] = note;
                        }
                        let amount = match script {
                            NpcScript::Boatman => -10,
                            NpcScript::TeaDoll => -256,
                            _ => -100,
                        };
                        let matrix = host.movement_matrix(actor.pose.angles)?;
                        actor.move_local(ActorAxis::Z, amount, matrix);
                    }
                    NpcScript::OperaSinger => {
                        if context.words[1] % 120 == 0 {
                            context.words[4] = trigger_ratio(actor, context, input)?;
                            context.words[7] = 1;
                        }
                    }
                    NpcScript::MarchingBand => {
                        if ticks < 20 {
                            actor.animation.enabled = false;
                            let matrix = host.movement_matrix(actor.pose.angles)?;
                            actor.move_local(ActorAxis::Z, -30, matrix);
                        } else if ticks == 20 {
                            actor.animation.enabled = true;
                            context.words[4] = 0;
                            context.words[7] = 5;
                        } else if ticks
                            .checked_rem(
                                animation_duration(actor, clips)?
                                    .wrapping_mul(3)
                                    .wrapping_add(20),
                            )
                            .ok_or(BehaviorFault::Division)?
                            == 0
                        {
                            actor.animation.enabled = false;
                            context.words[7] = -2;
                        }
                    }
                    NpcScript::Gears => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if actor.animation.frame_index as i32
                            == animation_duration(actor, clips)?.wrapping_sub(1)
                        {
                            context.words[7] = 25;
                            context.words[8] = -2;
                        }
                        if context.words[1] & 3 == 0 {
                            context.words[12] = 21;
                            context.words[13] = -1;
                        }
                        if context.words[1] % 200 == 0 {
                            context.words[17] = 13;
                            context.words[18] = 1;
                        }
                    }
                    NpcScript::GargoylePit => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if actor.animation.frame_index as i32
                            == animation_duration(actor, clips)? / 2
                        {
                            context.words[7] = 7;
                            context.words[8] = -2;
                            context.words[12] = 3;
                            context.words[13] = -2;
                        }
                        if context.words[1] % 90 < 3 {
                            context.words[17] = 6;
                            context.words[18] = -1;
                        }
                        if ticks >= 121 {
                            rotate_yaw(actor, 2)?;
                            move_local_with_host(actor, ActorAxis::Z, -320, host)?;
                        } else if ticks >= 56 || host.actor_proximity(actor, 1, 1)? {
                            actor.translate([0, -64, 0], false);
                        } else {
                            host.face_player(actor)?;
                            if ticks >= 10 {
                                move_local_with_host(actor, ActorAxis::Z, -256, host)?;
                            }
                        }
                    }
                    NpcScript::Mushroom => {
                        if ticks == 0 {
                            let day = host.day_number()? % 3;
                            if day == 2 || day == 0 && rng.next_u15().is_multiple_of(3) {
                                actor
                                    .scale_packet([0x0001_0001, 0x0001_0004, 0x0001_0001], true)
                                    .map_err(|_| BehaviorFault::Division)?;
                            }
                        }
                        if context.words[1] % 22 == 0 {
                            context.words[4] = trigger_ratio(actor, context, input)?;
                            context.words[7] = 2;
                        }
                        if rng.next_u15().is_multiple_of(12) {
                            actor.animation.enabled = false;
                        } else if rng.next_u15().is_multiple_of(6) {
                            actor.animation.enabled = true;
                        }
                    }
                    NpcScript::HumanFaceFlower => {
                        if ticks == 0 {
                            let day = host.day_number()? % 3;
                            if day == 1 || day == 0 && rng.next_u15().is_multiple_of(3) {
                                actor
                                    .scale_packet([0x0001_0006; 3], true)
                                    .map_err(|_| BehaviorFault::Division)?;
                            }
                        }
                        if context.words[1] == 0 {
                            context.words[4] = 0;
                            context.words[7] = 18;
                        }
                        host.face_player(actor)?;
                    }
                    NpcScript::GiantLips => {
                        if actor.animation.frame_index == 38 {
                            host.face_player(actor)?;
                            context.words[4] = trigger_ratio(actor, context, input)?;
                            context.words[7] = 6;
                        }
                        let vertical = if ticks % 10 < 5 { -30 } else { 30 };
                        move_local_with_host(actor, ActorAxis::Y, vertical, host)?;
                        move_local_with_host(actor, ActorAxis::Z, -30, host)?;
                        host.notify_actor(actor, rng, 6)?;
                    }
                    NpcScript::Whale => {
                        if ticks < animation_duration(actor, clips)?.wrapping_mul(5) {
                            if matches!(actor.animation.frame_index, 15 | 70) {
                                context.words[4] = 0;
                                context.words[7] = 7;
                                context.words[12] = 7;
                                context.words[17] = 7;
                            }
                        } else {
                            actor.set_hidden(true);
                            context.stop(sound);
                            actor.animation.enabled = false;
                            actor.animation.action_enabled = false;
                            *active = 0;
                            *triggered = 0;
                            *link_enabled = 0;
                            *state = 1;
                        }
                    }
                    NpcScript::HopscotchGirl => {
                        let mut frame = actor.animation.frame_index;
                        if ticks == 0 && rng.next_u15().is_multiple_of(3) {
                            actor
                                .scale_packet([0x0001_0001, 0x0001_0002, 0x0001_0001], true)
                                .map_err(|_| BehaviorFault::Division)?;
                        }
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if frame >= 32 {
                            frame -= 32;
                        }
                        if matches!(frame, 9 | 17 | 23) {
                            context.words[7] = 19;
                            if frame == 23 {
                                context.words[12] = 19;
                            }
                        }
                    }
                    NpcScript::Corpse => {
                        if ticks == 0 {
                            move_local_with_host(actor, ActorAxis::Y, -200, host)?;
                        }
                    }
                    NpcScript::Tennyo => {
                        host.face_player(actor)?;
                        if *triggered != 0 {
                            if ticks >= 65 {
                                *mutable_ticks = 0;
                            }
                            if *mutable_ticks >= 7 {
                                actor.translate([0, -64, 0], false);
                                move_local_with_host(actor, ActorAxis::Z, 10, host)?;
                            } else {
                                actor.translate([-64, 0, 0], false);
                            }
                        } else if ticks == 0 {
                            actor.translate([0, -4096, 0], false);
                        } else if ticks < 65 {
                            actor.translate([0, 64, 0], false);
                        } else if ticks < 71 {
                            actor.translate([0, -64, 0], false);
                            move_local_with_host(actor, ActorAxis::Z, -30, host)?;
                        } else {
                            let amount = if ticks < 256 { -ticks - 65 } else { 255 };
                            move_local_with_host(actor, ActorAxis::Z, amount, host)?;
                        }
                    }
                    NpcScript::Bird => {
                        if context.words[1] % 15 == 0 {
                            context.words[4] = trigger_ratio(actor, context, input)?;
                            context.words[7] = 7;
                            context.words[8] = -2;
                        }
                        actor
                            .scale_packet([0x0001_0002; 3], true)
                            .map_err(|_| BehaviorFault::Division)?;
                        rotate_yaw(actor, 2)?;
                        move_local_with_host(actor, ActorAxis::Z, -512, host)?;
                    }
                    NpcScript::Rocket => {
                        context.words[4] = 0;
                        if ticks < 100 {
                            if context.words[1] % 3 == 0 {
                                context.words[7] = 22;
                                context.words[8] = 1;
                            }
                            move_local_with_host(actor, ActorAxis::Y, -64, host)?;
                        } else if ticks < 300 {
                            for slot in [7, 12, 17] {
                                context.words[slot] = 12;
                                context.words[slot + 1] = -1;
                            }
                            move_local_with_host(actor, ActorAxis::Y, -256, host)?;
                        } else {
                            actor
                                .rotate_packet([0x0008_0001, 0x0001_0000, 0x0001_0000], false)
                                .map_err(|_| BehaviorFault::Division)?;
                            move_local_with_host(actor, ActorAxis::Y, -512, host)?;
                        }
                    }
                    NpcScript::Ufo => {
                        if context.words[1] % 70 == 0 {
                            context.words[4] = 0;
                            context.words[7] = 27;
                        }
                        move_local_with_host(actor, ActorAxis::Z, -128, host)?;
                        if ticks < 100 {
                            move_local_with_host(actor, ActorAxis::Y, 32, host)?;
                        } else if ticks >= 301 {
                            move_local_with_host(actor, ActorAxis::Y, -32, host)?;
                        }
                    }
                    NpcScript::Shark => {
                        if (ticks.wrapping_sub(400) as u32) < 10 {
                            rotate_yaw(actor, 9)?;
                        } else if (ticks.wrapping_sub(700) as u32) < 10
                            || (ticks.wrapping_sub(830) as u32) < 4
                        {
                            rotate_yaw(actor, -9)?;
                        } else if ticks >= 851 {
                            actor.set_hidden(true);
                            context.stop(sound);
                            actor.animation.enabled = false;
                            actor.animation.action_enabled = false;
                            *active = 0;
                            *triggered = 0;
                            *link_enabled = 0;
                        }
                        let amount = if *mutable_ticks < 800 { -60 } else { -512 };
                        move_local_with_host(actor, ActorAxis::Z, amount, host)?;
                        host.notify_actor(actor, rng, 6)?;
                    }
                    NpcScript::Fish => {
                        let vertical = if ticks % 6 < 3 { -64 } else { 64 };
                        move_local_with_host(actor, ActorAxis::Y, vertical, host)?;
                        let sideways = if ticks % 12 < 6 { -64 } else { 64 };
                        move_local_with_host(actor, ActorAxis::X, sideways, host)?;
                        let forward = if ticks % 64 < 32 { -128 } else { 128 };
                        move_local_with_host(actor, ActorAxis::Z, forward, host)?;
                        let remainder = ticks % 500;
                        if remainder < 32 {
                            actor.translate([0, -64, 0], false);
                        } else if remainder < 64 {
                            actor.translate([0, 64, 0], false);
                        }
                    }
                    NpcScript::Mirage => {
                        if *state == 0 {
                            let scale = if rng.next_u15() & 1 != 0 { 2 } else { 6 };
                            actor
                                .scale_packet([0x0001_0000 | scale; 3], true)
                                .map_err(|_| BehaviorFault::Division)?;
                            *state = 11;
                        }
                        host.face_player(actor)?;
                        if entity_distance(actor.pose.position, input.target) < 28672 {
                            move_local_with_host(actor, ActorAxis::Z, 256, host)?;
                        }
                    }
                    NpcScript::Astronaut => {
                        if context.words[1] == 0 && host.dream_color()? == 5 {
                            *state = 100;
                        }
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if *state == 0 {
                            if context.words[1] % 10 == 0 {
                                context.words[7] = 5;
                                context.words[8] = -2;
                            }
                            if ticks == 2400 {
                                *mutable_ticks = -1;
                            } else {
                                let amount = if ticks < 1200 { 50 } else { -50 };
                                move_local_with_host(actor, ActorAxis::Z, amount, host)?;
                            }
                        } else if ticks < 250 {
                            if context.words[1] % 10 == 0 {
                                context.words[7] = 5;
                                context.words[8] = -2;
                            }
                            if ticks < 100 {
                                move_local_with_host(actor, ActorAxis::Z, 50, host)?;
                            } else {
                                actor.translate([0, 64, -64], false);
                            }
                        } else if ticks == 250 {
                            actor.animation.enabled = false;
                            context.words[7] = -2;
                        } else if (ticks.wrapping_sub(261) as u32) < 307 {
                            move_local_with_host(actor, ActorAxis::Z, -50, host)?;
                            actor
                                .rotate_packet([0x0001_0000, 0x0001_ff88, 0x0001_0000], true)
                                .map_err(|_| BehaviorFault::Division)?;
                        } else if ticks >= 569 {
                            actor
                                .rotate_packet([0x0001_0032, 0x0001_ff88, 0x0001_001e], true)
                                .map_err(|_| BehaviorFault::Division)?;
                        }
                    }
                    NpcScript::Rainbow => {
                        if ticks == 0 {
                            if host.dream_color()? == 7 {
                                actor
                                    .scale_packet([0x0001_0003; 3], true)
                                    .map_err(|_| BehaviorFault::Division)?;
                                move_local_with_host(actor, ActorAxis::Y, -30720, host)?;
                            }
                            *state = i32::from(rng.next_u15() % 5);
                        }
                        if *state == 0 {
                            rotate_yaw(actor, 1)?;
                        }
                    }
                    NpcScript::BoatBird => {
                        if *state == 0 {
                            *state = if host.dream_color()? == 1 { 11 } else { 12 };
                        }
                        if *state == 12 {
                            actor
                                .scale_packet([0x0001_0002; 3], true)
                                .map_err(|_| BehaviorFault::Division)?;
                            move_local_with_host(actor, ActorAxis::Y, -30, host)?;
                        } else {
                            host.face_player(actor)?;
                            if *state == 11 {
                                move_local_with_host(actor, ActorAxis::Z, -100, host)?;
                                if (ticks.wrapping_sub(85) as u32) < 30 {
                                    move_local_with_host(actor, ActorAxis::Y, 80, host)?;
                                } else if ticks == 120 {
                                    *state = 13;
                                }
                            } else if *state == 13 {
                                actor.translate(host.player_local_position()?, true);
                                actor.translate([0, -1500, 1024], false);
                            }
                        }
                    }
                    NpcScript::GiantMaiko => {
                        actor.move_distance = -20;
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        let duration = animation_duration(actor, clips)? as u32;
                        let half = (duration.wrapping_add(duration >> 31) as i32) >> 1;
                        if context.words[1]
                            .checked_rem(half)
                            .ok_or(BehaviorFault::Division)?
                            == 0
                        {
                            context.words[7] = 10;
                            context.words[8] = 1;
                        }
                        let mut turn = None;
                        match *state {
                            11 => {
                                turn = match ticks {
                                    2700 | 3600 => Some(-90),
                                    3180 => Some(90),
                                    _ => None,
                                };
                                if (ticks.wrapping_sub(3421) as u32) < 120
                                    && host.player_action_pressed()?
                                {
                                    *mutable_ticks = 0;
                                    *state = 13;
                                }
                            }
                            12 if ticks == 1980 => turn = Some(-90),
                            13 => {
                                actor.move_distance = -120;
                                host.face_player(actor)?;
                                actor
                                    .scale_packet([0x0002_0001; 3], true)
                                    .map_err(|_| BehaviorFault::Division)?;
                                if entity_distance(actor.pose.position, input.target) < 1024 {
                                    host.notify_actor(actor, rng, 11)?;
                                }
                            }
                            _ => {}
                        }
                        if *mutable_ticks == 1560 {
                            if rng.next_u15() & 1 != 0 {
                                turn = Some(90);
                                *state = 11;
                            } else {
                                turn = Some(-90);
                                *state = 12;
                            }
                        }
                        if let Some(yaw) = turn {
                            rotate_yaw(actor, yaw)?;
                        }
                        let movement = i32::from(actor.move_distance);
                        crate::move_actor_on_floor(
                            actor,
                            ActorAxis::Z,
                            movement,
                            false,
                            rng,
                            host,
                        )?;
                        if *state != 12 && actor.body_contact.current.is_some() {
                            let matrix = host.movement_matrix(actor.pose.angles)?;
                            actor.move_local(ActorAxis::Y, -200, matrix);
                        }
                    }
                    NpcScript::Fox => {
                        if ticks == 0 && rng.next_u15() & 1 != 0 {
                            *state = 11;
                        }
                        if *state == 0 {
                            if ticks < 64 {
                                let matrix = host.movement_matrix(actor.pose.angles)?;
                                actor.move_local(ActorAxis::Z, -90, matrix);
                            } else if ticks == 64 {
                                let yaw = if rng.next_u15() & 1 != 0 { 90 } else { -90 };
                                rotate_yaw(actor, yaw)?;
                                actor.translate([0, 256, 0], false);
                            } else {
                                let notify = !rng.next_u15().is_multiple_of(2);
                                crate::move_actor_on_floor(
                                    actor,
                                    ActorAxis::Z,
                                    -374,
                                    notify,
                                    rng,
                                    host,
                                )?;
                            }
                        } else if *state == 11 {
                            if ticks % 5 == 0 {
                                rotate_yaw(actor, 90)?;
                            }
                            let matrix = host.movement_matrix(actor.pose.angles)?;
                            actor.move_local(ActorAxis::Z, -2048, matrix);
                            actor.set_hidden(!rng.next_u15().is_multiple_of(7));
                        }
                    }
                    NpcScript::StandingPenguin => {
                        if ticks == 0 && rng.next_u15().is_multiple_of(5) && *state == 0 {
                            actor
                                .scale_packet([0x0001_0006; 3], true)
                                .map_err(|_| BehaviorFault::Division)?;
                            let matrix = host.movement_matrix(actor.pose.angles)?;
                            actor.move_local(ActorAxis::Y, 800, matrix);
                            *state = 11;
                        }
                        if context.words[1] % 5 == 0 {
                            context.words[4] = trigger_ratio(actor, context, input)?;
                            context.words[7] = 8;
                        }
                        let yaw = match ticks {
                            90 => Some(-90),
                            160 => Some(90),
                            220 if rng.next_u15() & 1 != 0 => Some(180),
                            _ => None,
                        };
                        if let Some(yaw) = yaw {
                            rotate_yaw(actor, yaw)?;
                        }
                        let matrix = host.movement_matrix(actor.pose.angles)?;
                        actor.move_local(ActorAxis::Z, -80, matrix);
                        host.notify_actor(actor, rng, 6)?;
                    }
                    NpcScript::SlidingPenguin => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if ticks < 188 {
                            if ticks == 84 {
                                rotate_yaw(actor, 180)?;
                            }
                            if context.words[1] % 20 == 0 {
                                context.words[7] = 9;
                            }
                        } else if ticks < 200 {
                            rotate_yaw(actor, 9)?;
                        } else {
                            actor.set_hidden(true);
                            context.stop(sound);
                            actor.animation.enabled = false;
                            actor.animation.action_enabled = false;
                            *active = 0;
                            *triggered = 0;
                            *link_enabled = 0;
                            context.words[12] = 30;
                            *state = 1;
                        }
                        crate::move_actor_on_floor(actor, ActorAxis::Z, -512, false, rng, host)?;
                    }
                    NpcScript::Soldier | NpcScript::GiantTurtle => {
                        if script == NpcScript::Soldier {
                            if context.words[1] % 7 == 0 {
                                context.words[4] = trigger_ratio(actor, context, input)?;
                                context.words[7] = 3;
                                context.words[9] = 64;
                                context.words[10] = 64;
                            }
                            let turn = match ticks {
                                200 => Some(-90i16),
                                400 => Some(180),
                                600 => Some(90),
                                800 => {
                                    *mutable_ticks = -1;
                                    Some(180)
                                }
                                _ => None,
                            };
                            if let Some(turn) = turn {
                                actor
                                    .rotate_packet(
                                        [
                                            0x0001_0000,
                                            0x0001_0000 | u32::from(turn as u16),
                                            0x0001_0000,
                                        ],
                                        false,
                                    )
                                    .map_err(|_| BehaviorFault::Division)?;
                            }
                        } else if *triggered != 0 {
                            actor
                                .scale_packet([0x0004_0001; 3], true)
                                .map_err(|_| BehaviorFault::Division)?;
                        } else if context.words[1] % 30 == 0 {
                            context.words[4] = 0;
                            context.words[7] = 3;
                        }
                        crate::move_actor_on_floor(actor, ActorAxis::Z, -30, false, rng, host)?;
                        if actor.body_contact.current.is_some() {
                            let matrix = host.movement_matrix(actor.pose.angles)?;
                            actor.move_local(ActorAxis::Y, -200, matrix);
                        }
                    }
                    NpcScript::Horse => {
                        if context.words[1]
                            .checked_rem(animation_duration(actor, clips)?)
                            .ok_or(BehaviorFault::Division)?
                            == 0
                        {
                            context.words[4] = trigger_ratio(actor, context, input)?;
                            context.words[7] = 26;
                        }
                        crate::move_actor_on_floor(
                            actor,
                            ActorAxis::Z,
                            if ticks == 110 { -11520 } else { -384 },
                            false,
                            rng,
                            host,
                        )?;
                    }
                    NpcScript::Gargoyle => {
                        context.words[4] = 0;
                        if context.words[1] == 0 {
                            for index in [7, 12, 17] {
                                context.words[index] = 20;
                            }
                            host.clear_player_callbacks(true)?;
                        }
                        host.face_player(actor)?;
                        let matrix = host.movement_matrix(actor.pose.angles)?;
                        actor.move_local(ActorAxis::Z, -90, matrix);
                        if ticks == 30 {
                            host.notify_actor(actor, rng, 10)?;
                        }
                    }
                    NpcScript::DemonKing => {
                        host.face_player(actor)?;
                        host.clear_player_callbacks(true)?;
                        if context.words[1] % 10 < 3 {
                            context.words[4] = 0;
                            for index in [7, 12, 17] {
                                context.words[index] = 13;
                            }
                        }
                        let duration = actor
                            .animation
                            .selected_clip
                            .and_then(|i| clips.get(i))
                            .ok_or(AnimationError::InvalidClip)?
                            .frames
                            .len() as i32;
                        if ticks == duration {
                            actor.animation.enabled = false;
                            host.notify_actor(actor, rng, 10)?;
                        }
                    }
                    NpcScript::Dog => {
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        if context.words[1] == 0 {
                            actor.move_distance = if rng.next_u15() & 1 != 0 { -374 } else { -192 };
                        }
                        if *state == 0 {
                            if context.words[1] % 10 == 0 {
                                context.words[7] = 28;
                            }
                            if context.words[1] % 20 == 0 {
                                context.words[12] = 23;
                                context.words[13] = -1;
                                context.words[17] = 23;
                                context.words[18] = -1;
                            } else if context.words[1] % 20 == 14 {
                                context.words[12] = -2;
                                context.words[17] = -2;
                            }
                            if ticks & 1 == 0 && host.player_action_pressed()? {
                                *mutable_ticks = -1;
                                *state = 10;
                                context.words[7] = 18;
                            }
                            host.face_player(actor)?;
                            let matrix = host.movement_matrix(actor.pose.angles)?;
                            actor.move_local(ActorAxis::Z, i32::from(actor.move_distance), matrix);
                            host.notify_actor(actor, rng, 6)?;
                        } else if *state == 10 {
                            if ticks < 8 {
                                actor
                                    .rotate_packet([0x0001_0000, 0x0001_0000, 0x0001_0009], false)
                                    .map_err(|_| BehaviorFault::Division)?;
                                actor.translate([0, 8, 0], false);
                            } else {
                                context.words[7] = 18;
                                context.words[12] = 3;
                                context.stop(sound);
                                actor.animation.enabled = false;
                                actor.animation.action_enabled = false;
                                *active = 0;
                                *state = i32::from(rng.next_u15().is_multiple_of(2));
                            }
                        }
                    }
                    NpcScript::Bear | NpcScript::Rabbit => {
                        if ticks == 0 {
                            let roll = rng.next_u15() % 10;
                            if roll >= 8 {
                                actor
                                    .scale_packet([0x0001_0003, 0x0001_0001, 0x0001_0001], true)
                                    .map_err(|_| BehaviorFault::Division)?;
                            } else if roll >= 5 {
                                *state = 10;
                            }
                        }
                        if *state == 10 && ticks >= 201 {
                            actor.translate([0, 0, -256], false);
                        }
                        context.words[4] = trigger_ratio(actor, context, input)?;
                        let frame = actor.animation.frame_index;
                        if script == NpcScript::Bear {
                            if frame == 0 || frame == 15 {
                                context.words[7] = 18;
                                context.words[12] = 18;
                            }
                            if ticks >= 321 {
                                let amount = if rng.next_u15() & 1 != 0 { -60 } else { 60 };
                                let matrix = host.movement_matrix(actor.pose.angles)?;
                                actor.move_local(ActorAxis::X, amount, matrix);
                                let yaw = if rng.next_u15() & 3 != 0 { -9_i16 } else { 9 };
                                actor
                                    .rotate_packet(
                                        [
                                            0x0001_0000,
                                            0x0001_0000 | u32::from(yaw as u16),
                                            0x0001_0000,
                                        ],
                                        false,
                                    )
                                    .map_err(|_| BehaviorFault::Division)?;
                            }
                        } else {
                            if frame == 7 || frame == 22 {
                                context.words[7] = 3;
                            }
                            if (ticks.wrapping_sub(300) as u32) < 20 {
                                let matrix = host.movement_matrix(actor.pose.angles)?;
                                actor.move_local(ActorAxis::Z, -60, matrix);
                            } else if (ticks.wrapping_sub(321) as u32) < 19 {
                                actor
                                    .rotate_packet([0x0001_0000, 0x0001_fff7, 0x0001_0000], false)
                                    .map_err(|_| BehaviorFault::Division)?;
                            } else if ticks >= 321 {
                                let amount = if rng.next_u15() & 1 != 0 { 128 } else { -128 };
                                let matrix = host.movement_matrix(actor.pose.angles)?;
                                actor.move_local(ActorAxis::X, amount, matrix);
                                host.notify_actor(actor, rng, 7)?;
                                let yaw = if rng.next_u15() & 3 != 0 { -9_i16 } else { 9 };
                                actor
                                    .rotate_packet(
                                        [
                                            0x0001_0000,
                                            0x0001_0000 | u32::from(yaw as u16),
                                            0x0001_0000,
                                        ],
                                        false,
                                    )
                                    .map_err(|_| BehaviorFault::Division)?;
                            }
                        }
                    }
                    NpcScript::Pending(_) => return Err(BehaviorFault::PendingCallback),
                }
                Ok(())
            })?;
            *mutable_ticks = mutable_ticks.wrapping_add(1);
        }
        actor.animation.advance_frame(clips)?;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviorTick {
    pub target: [i32; 3],
    pub proximity: i8,
}

/// Identified target coordinate/view state. Matrix construction and global
/// scheduling are supplied by the scene; this preserves the original query math.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityTarget {
    pub position: [i32; 3],
    pub local_y: i32,
    pub matrix: crate::FixedMatrix,
    pub eye: [i32; 3],
    pub target: [i32; 3],
}
impl EntityTarget {
    pub fn resolve(self, forward: i32) -> Result<[i32; 3], BehaviorFault> {
        let mut position = self.matrix.apply_long([0, 0, forward]);
        for (value, origin) in position.iter_mut().zip(self.position) {
            *value = value.wrapping_add(origin);
        }
        let multiplier = forward / 1024;
        let delta = self.target[2].wrapping_sub(self.eye[2]);
        let divisor = match delta / 1024 {
            0 => 1,
            value => value,
        };
        position[1] = self.target[1]
            .wrapping_sub(self.eye[1])
            .wrapping_mul(multiplier)
            .checked_div(divisor)
            .ok_or(BehaviorFault::Division)?
            .wrapping_add(self.eye[1])
            .wrapping_add(self.position[1]);
        Ok(position)
    }
    pub fn proximity(
        self,
        point: [i32; 3],
        forward: i32,
        radius: i32,
    ) -> Result<bool, BehaviorFault> {
        let origin = self.resolve(forward)?;
        Ok(origin.into_iter().zip(point).all(|(origin, point)| {
            point >= origin.wrapping_sub(radius) && point <= origin.wrapping_add(radius)
        }))
    }
    pub fn facing(self, position: [i32; 3], distance: i32) -> bool {
        !(self.local_y.wrapping_add(512) < position[1]
            || position[1] < self.local_y.wrapping_sub(512))
            && entity_distance(position, self.position) < distance
    }
}
