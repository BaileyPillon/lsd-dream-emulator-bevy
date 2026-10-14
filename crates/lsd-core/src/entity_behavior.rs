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
        let mut position = position;
        if (fields[2] as u8).wrapping_add(9) < 9 {
            position[1] = position[1].wrapping_add(i32::from(fields[2]).wrapping_shl(10));
        }
        let radius = if radius < 0 {
            2048 / radius.wrapping_neg()
        } else {
            radius.wrapping_shl(11)
        };
        host.proximity(position, range.wrapping_shl(11), radius)
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
