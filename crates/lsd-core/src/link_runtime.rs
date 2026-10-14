use crate::{
    LinkCatalog, LinkContext, LinkError, LinkExecution, LinkKind, PlayerControls, PsxRng,
    SpawnLocation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoMoveTables {
    pub targets: [[i16; 3]; 4],
    pub clockwise: [u32; 3],
    pub counterclockwise: [u32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DreamLinks {
    pub execution: LinkExecution,
    pub spawn: SpawnLocation,
    pub source_transform: Option<[u32; 3]>,
    pub target_transform: Option<[u32; 3]>,
    pub auto_action: Option<u8>,
    pub auto_step: i32,
    pub auto_target: SpawnLocation,
    pub time_limit: i32,
    pub default_teleport_transform: bool,
}

/// Scene/notification effects occur at their original boundaries and may mutate live state.
pub trait DreamLinkEffects {
    fn notify(&mut self, state: &mut DreamLinks, controls: &mut PlayerControls, event: i32);
    fn sound(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        parameters: [i32; 3],
    );
    fn rotate(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        absolute: bool,
        packet: [u32; 3],
    );
    fn translate(&mut self, state: &mut DreamLinks, controls: &mut PlayerControls, delta: [i32; 3]);
    fn relocate(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        spawn: SpawnLocation,
    );
}

pub fn rotation_degrees(angles: [i16; 3]) -> [i16; 3] {
    angles.map(|angle| ((i32::from(angle) * 45) >> 9) as i16)
}

impl DreamLinks {
    /// Original direct dynamic-link handler used by wall and entity journals.
    pub fn force_dynamic(
        &mut self,
        controls: &mut PlayerControls,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        rng: &mut PsxRng,
        effects: &mut impl DreamLinkEffects,
    ) -> Result<bool, LinkError> {
        if self.execution.state != 0 {
            return Ok(false);
        }
        let target = catalog.random_spawn(self.execution.stage, &mut self.spawn, context, rng)?;
        Ok(self.execute(controls, target, 12, true, effects))
    }
    pub fn execute(
        &mut self,
        controls: &mut PlayerControls,
        target: i32,
        event: i32,
        play_sound: bool,
        effects: &mut impl DreamLinkEffects,
    ) -> bool {
        self.execution.state = event;
        effects.notify(self, controls, event);
        if self.execution.state == 0 {
            return false;
        }
        self.execution.stage = target;
        if self.execution.flashback {
            self.execution.dream_timer = 0;
        }
        if play_sound {
            effects.sound(self, controls, [0x90, 0x6e, 0x6e]);
        }
        true
    }

    pub fn tunnel_at(
        &mut self,
        controls: &mut PlayerControls,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        key: SpawnLocation,
        effects: &mut impl DreamLinkEffects,
    ) -> Result<bool, LinkError> {
        if self.execution.state != 0 {
            return Ok(false);
        }
        let Some(target) = catalog.static_spawn(
            LinkKind::Tunnel,
            key,
            self.execution.stage,
            &mut self.spawn,
            context,
        )?
        else {
            return Ok(false);
        };
        let Some(transforms) = catalog.facing_link(
            LinkKind::Tunnel,
            context,
            rotation_degrees(controls.angles)[1],
        )?
        else {
            return Ok(false);
        };
        self.source_transform = Some(transforms.source);
        self.target_transform = Some(transforms.target);
        if !controls.is_forward {
            return Ok(false);
        }
        self.execute(controls, target, 15, false, effects);
        Ok(true)
    }

    pub fn teleport_at(
        &mut self,
        controls: &mut PlayerControls,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        key: SpawnLocation,
        effects: &mut impl DreamLinkEffects,
    ) -> Result<bool, LinkError> {
        let Some(target) = catalog.static_spawn(
            LinkKind::Teleport,
            key,
            self.execution.stage,
            &mut self.spawn,
            context,
        )?
        else {
            return Ok(false);
        };
        let bonus = if context.source_stage == 0 { 10 } else { 0 };
        if self.execute(controls, target, 17, false, effects) {
            self.execution.state = 0;
            let spawn = self.spawn;
            effects.relocate(self, controls, spawn);
            if bonus != 0 && !self.execution.flashback {
                self.time_limit = ((self.execution.dream_timer / 15) as i32)
                    .wrapping_add(bonus)
                    .wrapping_mul(15);
            }
        }
        Ok(true)
    }

    pub fn dynamic_at(
        &mut self,
        controls: &mut PlayerControls,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        key: SpawnLocation,
        rng: &mut PsxRng,
        effects: &mut impl DreamLinkEffects,
    ) -> Result<bool, LinkError> {
        if self.execution.state != 0 {
            return Ok(false);
        }
        let Some(target) = catalog.dynamic_spawn(
            self.execution.stage,
            key,
            self.execution.dream_timer,
            &mut self.spawn,
            context,
            rng,
        )?
        else {
            return Ok(false);
        };
        self.default_teleport_transform = context.target_stage != 12;
        self.source_transform = None;
        self.target_transform = None;
        self.execute(controls, target, 16, false, effects);
        Ok(true)
    }

    pub fn staircase_at(
        &mut self,
        controls: &mut PlayerControls,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        key: SpawnLocation,
        tables: &AutoMoveTables,
        effects: &mut impl DreamLinkEffects,
    ) -> Result<bool, LinkError> {
        if self.execution.state != 0 {
            return Ok(false);
        }
        if let Some(action) = self.auto_action {
            if self.auto_move_tick(action, controls, tables, effects)? {
                controls.auto_move_locked = false;
                controls.auto_move_active = false;
                self.auto_action = None;
                if controls.move_direction == 4 {
                    controls.move_direction = controls.previous_move_direction;
                }
            }
        } else if catalog
            .static_spawn(
                LinkKind::Staircase,
                key,
                self.execution.stage,
                &mut self.spawn,
                context,
            )?
            .is_some()
            && let Some(transforms) = catalog.facing_link(
                LinkKind::Staircase,
                context,
                rotation_degrees(controls.angles)[1],
            )?
        {
            self.source_transform = Some(transforms.source);
            self.target_transform = Some(transforms.target);
            if controls.is_forward {
                let action = u8::try_from(catalog.staircase_action(context)?)
                    .map_err(|_| LinkError::InvalidMovement)?;
                if action >= 4 {
                    return Err(LinkError::InvalidMovement);
                }
                self.auto_target = key;
                controls.auto_move_locked = true;
                controls.auto_move_active = true;
                self.auto_step = 0;
                self.auto_action = Some(action);
                effects.rotate(self, controls, true, transforms.source);
                self.auto_move_tick(action, controls, tables, effects)?;
            }
        }
        // Original stair handling always returns zero, including initiation and completion.
        Ok(false)
    }

    pub fn auto_move_tick(
        &mut self,
        action: u8,
        controls: &mut PlayerControls,
        tables: &AutoMoveTables,
        effects: &mut impl DreamLinkEffects,
    ) -> Result<bool, LinkError> {
        let target = *tables
            .targets
            .get(usize::from(action))
            .ok_or(LinkError::InvalidMovement)?;
        if self.auto_step == 0 {
            let delta = [
                i32::from(target[0]) - i32::from(self.auto_target.position[0]),
                0,
                i32::from(target[2]) - i32::from(self.auto_target.position[2]),
            ];
            effects.translate(self, controls, delta);
        }
        let fast = controls.move_direction == 4;
        let step = self.auto_step;
        let complete = if fast {
            [19, 25, 15, 19][usize::from(action)]
        } else {
            [133, 149, 101, 113][usize::from(action)]
        };
        if step >= complete {
            return Ok(true);
        }
        let inside =
            |value: i32, start: i32, length: u32| (value.wrapping_sub(start) as u32) < length;
        let turning = match (action, fast) {
            (0, false) => inside(step, 43, 15) || inside(step, 75, 15),
            (1, false) => inside(step, 22, 15) || inside(step, 57, 16) || inside(step, 110, 15),
            (2, false) => inside(step, 43, 15),
            (3, false) => inside(step, 30, 15) || inside(step, 82, 15),
            (0, true) => inside(step, 8, 2) || inside(step, 13, 2),
            (1, true) => inside(step, 6, 2) || inside(step, 11, 2) || inside(step, 20, 2),
            (2, true) => inside(step, 8, 2),
            (3, true) => inside(step, 6, 2) || inside(step, 15, 2),
            _ => unreachable!(),
        };
        if turning {
            if fast {
                effects.rotate(
                    self,
                    controls,
                    false,
                    if action == 0 || action == 2 {
                        tables.clockwise
                    } else {
                        tables.counterclockwise
                    },
                );
            } else {
                controls.left_right = if action == 0 || action == 2 { 2 } else { 1 };
            }
        }
        let look = match (action, fast) {
            (1, false) => inside(self.auto_step, 57, 53),
            (1, true) => inside(self.auto_step, 3, 14),
            (3, false) => inside(self.auto_step, 30, 52),
            (3, true) => (self.auto_step as u32) < 9,
            _ => false,
        };
        if look {
            controls.look_state = 2;
        }
        controls.move_state = 1;
        self.auto_step = self.auto_step.wrapping_add(1);
        Ok(false)
    }
}
