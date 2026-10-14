use crate::{MotionAxis, MotionRequest, PsxRng};
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpawnLocation {
    pub chunk: i16,
    pub tile: i16,
    pub position: [i16; 3],
}

impl SpawnLocation {
    pub fn packed_key(self) -> u32 {
        u32::from(self.chunk as u16) | (u32::from(self.tile as u16) << 16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnEntry {
    pub chunk: i16,
    pub tile: i16,
    pub adjustment: u8,
    /// Navigation challenge for wall/tunnel entries; auto-move index for stairs.
    pub extra: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkTrigger {
    pub chunk: i16,
    pub tile: i16,
    pub target_stage: i8,
    pub target_spawn: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkSet {
    pub triggers: [Vec<LinkTrigger>; 14],
    pub spawns: [Vec<SpawnEntry>; 14],
    pub trigger_transforms: Option<[Vec<u8>; 14]>,
    pub spawn_transforms: Option<[Vec<u8>; 14]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Wall,
    Tunnel,
    Teleport,
    Staircase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCatalog {
    pub adjustments: Vec<[i16; 3]>,
    pub random_spawns: [Vec<SpawnEntry>; 14],
    pub wall: LinkSet,
    pub tunnel: LinkSet,
    pub teleport: LinkSet,
    pub staircase: LinkSet,
    pub transforms: Vec<[u32; 3]>,
    pub facing_angles: Vec<u16>,
    pub violence_key: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkContext {
    pub source_stage: i32,
    pub source_index: i32,
    pub target_stage: i32,
    pub target_index: i32,
    pub teleports_enabled: i32,
    pub navigation: [u8; 30],
    pub dynamic_penalty: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkTransforms {
    pub target: [u32; 3],
    pub source: [u32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    InvalidStage,
    MissingSpawn,
    InvalidAdjustment,
    InvalidNavigationChallenge,
    MissingTransform,
    InvalidMovement,
}
impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "original link input is outside the supported catalogue: {self:?}"
        )
    }
}
impl std::error::Error for LinkError {}

impl LinkCatalog {
    pub fn set(&self, kind: LinkKind) -> &LinkSet {
        match kind {
            LinkKind::Wall => &self.wall,
            LinkKind::Tunnel => &self.tunnel,
            LinkKind::Teleport => &self.teleport,
            LinkKind::Staircase => &self.staircase,
        }
    }

    fn location(&self, entry: SpawnEntry) -> Result<SpawnLocation, LinkError> {
        Ok(SpawnLocation {
            chunk: entry.chunk,
            tile: entry.tile,
            position: *self
                .adjustments
                .get(usize::from(entry.adjustment))
                .ok_or(LinkError::InvalidAdjustment)?,
        })
    }

    /// First matching trigger wins. Failed lookups preserve the output and link globals.
    pub fn static_spawn(
        &self,
        kind: LinkKind,
        key: SpawnLocation,
        stage: i32,
        output: &mut SpawnLocation,
        context: &mut LinkContext,
    ) -> Result<Option<i32>, LinkError> {
        if (kind == LinkKind::Teleport && context.teleports_enabled == 0)
            || (kind == LinkKind::Staircase && stage != 0)
        {
            return Ok(None);
        }
        let stage_index = usize::try_from(stage).map_err(|_| LinkError::InvalidStage)?;
        let set = self.set(kind);
        let triggers = set
            .triggers
            .get(stage_index)
            .ok_or(LinkError::InvalidStage)?;
        for (index, trigger) in triggers.iter().enumerate() {
            if key.chunk != trigger.chunk || (key.tile != trigger.tile && trigger.tile >= 0) {
                continue;
            }
            let target =
                usize::try_from(trigger.target_stage).map_err(|_| LinkError::InvalidStage)?;
            let entry = *set
                .spawns
                .get(target)
                .ok_or(LinkError::InvalidStage)?
                .get(usize::from(trigger.target_spawn))
                .ok_or(LinkError::MissingSpawn)?;
            let location = self.location(entry)?;
            let navigation = if matches!(kind, LinkKind::Wall | LinkKind::Tunnel) {
                let challenge = usize::try_from(entry.extra)
                    .map_err(|_| LinkError::InvalidNavigationChallenge)?;
                if challenge >= context.navigation.len() {
                    return Err(LinkError::InvalidNavigationChallenge);
                }
                Some(challenge)
            } else {
                None
            };
            context.source_stage = stage;
            context.source_index = index as i32;
            context.target_stage = i32::from(trigger.target_stage);
            context.target_index = i32::from(trigger.target_spawn);
            *output = location;
            if let Some(challenge) = navigation {
                context.navigation[challenge] = 1;
            }
            return Ok(Some(context.target_stage));
        }
        Ok(None)
    }

    /// Normal selection consumes two RNG results; a negative fixed stage consumes one.
    pub fn random_spawn(
        &self,
        stage: i32,
        output: &mut SpawnLocation,
        context: &mut LinkContext,
        rng: &mut PsxRng,
    ) -> Result<i32, LinkError> {
        let selected = if stage >= 0 {
            let mut chosen = i32::from(rng.next_u15() % 6);
            if chosen == stage {
                chosen = (chosen + 1) % 6;
            }
            chosen
        } else {
            stage.wrapping_neg()
        };
        let entries = self
            .random_spawns
            .get(usize::try_from(selected).map_err(|_| LinkError::InvalidStage)?)
            .ok_or(LinkError::InvalidStage)?;
        if entries.is_empty() {
            return Err(LinkError::MissingSpawn);
        }
        let index = usize::from(rng.next_u15()) % entries.len();
        *output = self.location(entries[index])?;
        context.dynamic_penalty = context.dynamic_penalty.wrapping_add(1);
        Ok(selected)
    }

    pub fn dynamic_spawn(
        &self,
        stage: i32,
        key: SpawnLocation,
        tick: u32,
        output: &mut SpawnLocation,
        context: &mut LinkContext,
        rng: &mut PsxRng,
    ) -> Result<Option<i32>, LinkError> {
        let eligible = match stage {
            1 | 3 | 12 => true,
            5 => key.position[1] < -4095 || key.packed_key() == self.violence_key,
            9 => key.position[1] >= 2048,
            _ => false,
        };
        if !eligible {
            return Ok(None);
        }
        let selected = self.random_spawn(
            if tick & 1 != 0 { -12 } else { stage },
            output,
            context,
            rng,
        )?;
        context.target_stage = selected;
        Ok(Some(selected))
    }

    /// Return target/source transforms after the original facing gate succeeds.
    pub fn facing_link(
        &self,
        kind: LinkKind,
        context: &LinkContext,
        yaw_degrees: i16,
    ) -> Result<Option<LinkTransforms>, LinkError> {
        let set = self.set(kind);
        let source = set
            .trigger_transforms
            .as_ref()
            .and_then(|rows| rows.get(usize::try_from(context.source_stage).ok()?))
            .and_then(|row| row.get(usize::try_from(context.source_index).ok()?))
            .copied()
            .ok_or(LinkError::MissingTransform)?;
        let angle = *self
            .facing_angles
            .get(usize::from(source))
            .ok_or(LinkError::MissingTransform)?;
        if !is_facing_link_angle(yaw_degrees, angle) {
            return Ok(None);
        }
        let target = set
            .spawn_transforms
            .as_ref()
            .and_then(|rows| rows.get(usize::try_from(context.target_stage).ok()?))
            .and_then(|row| row.get(usize::try_from(context.target_index).ok()?))
            .copied()
            .ok_or(LinkError::MissingTransform)?;
        Ok(Some(LinkTransforms {
            target: *self
                .transforms
                .get(usize::from(target))
                .ok_or(LinkError::MissingTransform)?,
            source: *self
                .transforms
                .get(usize::from(source))
                .ok_or(LinkError::MissingTransform)?,
        }))
    }

    pub fn staircase_action(&self, context: &LinkContext) -> Result<i8, LinkError> {
        self.staircase
            .spawns
            .get(usize::try_from(context.target_stage).map_err(|_| LinkError::InvalidStage)?)
            .and_then(|row| row.get(usize::try_from(context.target_index).ok()?))
            .map(|entry| entry.extra)
            .ok_or(LinkError::MissingSpawn)
    }
}

pub fn is_facing_link_angle(yaw_degrees: i16, angle: u16) -> bool {
    let difference = yaw_degrees.wrapping_sub(angle as i16);
    let wrapped = if difference >= 181 {
        difference.wrapping_sub(360)
    } else if difference < -180 {
        difference.wrapping_add(360)
    } else {
        difference
    };
    (wrapped.wrapping_add(44) as u16) < 89
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkExecution {
    pub state: i32,
    pub stage: i32,
    pub dream_timer: u32,
    pub flashback: bool,
}

impl LinkExecution {
    /// Notification can cancel or mutate the transition before the destination is committed.
    pub fn execute(
        &mut self,
        target: i32,
        event: i32,
        play_sound: bool,
        mut notify: impl FnMut(&mut Self, i32),
        mut sound: impl FnMut(&mut Self, [i32; 3]),
    ) -> bool {
        self.state = event;
        notify(self, event);
        if self.state == 0 {
            return false;
        }
        self.stage = target;
        if self.flashback {
            self.dream_timer = 0;
        }
        if play_sound {
            sound(self, [0x90, 0x6e, 0x6e]);
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkMotionRules {
    pub distances: [i32; 5],
    pub signs: [i8; 5],
}

/// Original virtual boundaries. Implementations may mutate live state during each callback.
pub trait LinkActionHost {
    fn move_direction(&self) -> i32;
    fn auto_move_active(&self) -> bool;
    fn stage(&self) -> i32;
    fn position(&self) -> [i32; 3];
    fn hook(&mut self);
    fn spawn_key(&mut self) -> SpawnLocation;
    fn staircase(&mut self, key: SpawnLocation) -> i32;
    fn teleport(&mut self, key: SpawnLocation) -> i32;
    fn tunnel(&mut self, key: SpawnLocation) -> i32;
    fn save_link_state(&mut self);
    fn apply_motion(&mut self, request: MotionRequest);
    fn wall_link(&mut self, code: i32);
    fn clear_transform_marker(&mut self);
}

pub fn trigger_link_action(
    host: &mut impl LinkActionHost,
    rules: &LinkMotionRules,
    action: i32,
) -> Result<(), LinkError> {
    if action == 0 {
        return Ok(());
    }
    if !(1..=4).contains(&action) {
        return Err(LinkError::InvalidMovement);
    }
    let direction =
        usize::try_from(host.move_direction()).map_err(|_| LinkError::InvalidMovement)?;
    let distance = *rules
        .distances
        .get(direction)
        .ok_or(LinkError::InvalidMovement)?;
    let amount = distance.wrapping_mul(i32::from(rules.signs[action as usize]));
    host.hook();
    let key = host.spawn_key();
    if host.staircase(key) == 0 && host.teleport(key) == 0 && host.tunnel(key) == 0 {
        host.save_link_state();
        host.apply_motion(MotionRequest {
            axis: if action <= 2 {
                MotionAxis::Forward
            } else {
                MotionAxis::Strafe
            },
            amount,
            collision_and_links: !host.auto_move_active(),
        });
        if host.stage() == 0 {
            let position = host.position();
            if position[1] < -2000 && position[0] >= -499 {
                host.wall_link(4);
            }
        }
    }
    host.clear_transform_marker();
    Ok(())
}
