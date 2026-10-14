use crate::{
    AutoMoveTables, BodyQueryState, BodyVisit, ChunkPlayer, DreamContactEffects, DreamLinkEffects,
    DreamLinks, FloorEffectStep, FloorLinkRequest, LinkActionHost, LinkCatalog, LinkContext,
    LinkError, LinkExecution, LinkMotionRules, LinkPalette, MapLinkCommand, MapLinkPolicy,
    MotionAxis, MotionRequest, PAD_MASKS, PlayerControls, PsxMathTables, PsxRng, SceneActorKeys,
    SceneCollision, SceneCollisionError, SceneContact, SpawnLocation, WallLinkRequest,
    trigger_link_action,
};

#[derive(Debug, Clone)]
pub struct SceneLinkTables {
    pub catalog: LinkCatalog,
    pub motion: LinkMotionRules,
    pub auto_move: AutoMoveTables,
}
#[derive(Debug, Clone, Copy)]
struct SavedPlayerPose {
    position: [i32; 3],
    angles: [i16; 3],
}

#[derive(Debug, Clone)]
pub struct LinkedPlayer {
    pub links: DreamLinks,
    pub context: LinkContext,
    pub keys: SceneActorKeys,
    pub map: MapLinkPolicy,
    pub palette: LinkPalette,
    pub rng: PsxRng,
    pub interval: u32,
    pub can_dynamic: bool,
    pub floor_surface: i32,
    pub commands: Vec<MapLinkCommand>,
    pub sounds: Vec<[i32; 3]>,
    pub body_contact: Option<SceneContact>,
    pub floor_contact: Option<SceneContact>,
    pub floor_effect: Option<FloorEffectStep>,
    pub tables: SceneLinkTables,
    saved: SavedPlayerPose,
    base: [i32; 3],
}
impl LinkedPlayer {
    /// Dispatch a contact using the current cached body key and previously saved pose.
    pub fn wall_contact(
        &mut self,
        player: &mut ChunkPlayer,
        scene: &SceneCollision,
        code: i32,
    ) -> Result<(), SceneCollisionError> {
        let mut effects = Effects {
            position: &mut player.position,
            map: &mut self.map,
            commands: &mut self.commands,
            sounds: &mut self.sounds,
            saved: self.saved,
            grid: scene.slots.grid,
            base: self.base,
            palette: self.palette,
            error: None,
        };
        self.links.wall_at(
            &mut player.controls,
            &self.tables.catalog,
            &mut self.context,
            WallLinkRequest {
                code,
                contact: self.keys.contact,
                can_dynamic: self.can_dynamic,
            },
            &mut self.rng,
            &mut effects,
        )?;
        if let Some(error) = effects.error {
            return Err(error.into());
        }
        Ok(())
    }

    /// Scene host has already placed the actor and applied its target rotation.
    pub fn prepare_link_scene(
        &mut self,
        player: &mut ChunkPlayer,
        scene: &SceneCollision,
        stage: u8,
    ) -> Result<crate::SceneMotionReset, SceneCollisionError> {
        let reset = crate::reset_scene_motion(
            &mut player.controls,
            &mut self.links,
            &mut self.map,
            if stage == 0 { 2 } else { 3 },
            16,
        )?;
        self.floor_surface = reset.floor_surface;
        self.floor_effect = None;
        self.interval = reset.interval;
        self.can_dynamic = reset.can_dynamic != 0;
        self.body_contact = None;
        self.floor_contact = None;
        self.map.state = 5;
        self.map.location = i32::from(stage);
        self.map.tick = 0;
        self.commands.clear();
        self.sounds.clear();
        self.saved = SavedPlayerPose {
            position: player.position,
            angles: player.controls.angles,
        };
        self.refresh_key(scene, player.position)?;
        Ok(reset)
    }

    pub fn new(
        stage: u8,
        player: &ChunkPlayer,
        scene: &SceneCollision,
        tables: SceneLinkTables,
        seed: u32,
    ) -> Result<Self, SceneCollisionError> {
        let key = scene
            .slots
            .locate(player.position)?
            .ok_or(crate::SceneGridError::InvalidSlot)?
            .spawn_key();
        Ok(Self {
            links: DreamLinks {
                execution: LinkExecution {
                    state: 0,
                    stage: i32::from(stage),
                    dream_timer: 0,
                    flashback: false,
                },
                spawn: key,
                source_transform: None,
                target_transform: None,
                auto_action: None,
                auto_step: 0,
                auto_target: SpawnLocation::default(),
                time_limit: -1,
                default_teleport_transform: false,
            },
            context: LinkContext::default(),
            keys: SceneActorKeys {
                current: key,
                contact: SpawnLocation::default(),
            },
            map: MapLinkPolicy {
                state: 0,
                tick: 0,
                location: i32::from(stage),
                move_callback: 1,
                reset_view_pending: false,
                view_active: false,
            },
            palette: LinkPalette {
                dream: 0,
                flashback: 0,
            },
            rng: PsxRng::new(seed),
            interval: 16,
            can_dynamic: false,
            floor_surface: 0,
            commands: Vec::new(),
            sounds: Vec::new(),
            body_contact: None,
            floor_contact: None,
            floor_effect: None,
            tables,
            saved: SavedPlayerPose {
                position: player.position,
                angles: player.controls.angles,
            },
            base: [0; 3],
        })
    }

    /// Loaded-scene integration. Startup, fade actors, audio and entity callbacks remain separate.
    pub fn step(
        &mut self,
        player: &mut ChunkPlayer,
        scene: &SceneCollision,
        math: &PsxMathTables,
        ready: [i16; 7],
        buttons: u32,
    ) -> Result<(), SceneCollisionError> {
        self.step_with_clock(player, scene, math, ready, buttons, None)
    }

    pub fn step_with_clock(
        &mut self,
        player: &mut ChunkPlayer,
        scene: &SceneCollision,
        math: &PsxMathTables,
        ready: [i16; 7],
        buttons: u32,
        clock: Option<(&mut crate::DreamDay, &crate::DayTables)>,
    ) -> Result<(), SceneCollisionError> {
        self.begin_frame();
        self.map.tick = self.map.tick.wrapping_add(1);
        self.tick_clock(player, scene, math, ready, clock, crate::FrameSignal::Tick)?;
        Self::poll_pad(player, buttons);
        Ok(())
    }

    /// Clear this adapter's per-frame reports before other clock subscribers run.
    pub fn begin_frame(&mut self) {
        self.commands.clear();
        self.sounds.clear();
        self.body_contact = None;
        self.floor_contact = None;
        self.floor_effect = None;
    }

    /// The display timer polls and dispatches the pad after its clock traversal.
    pub fn poll_pad(player: &mut ChunkPlayer, buttons: u32) {
        player.pad.poll(buttons);
        for code in player.pad.notifications(&PAD_MASKS) {
            player.controls.notify_pad(code);
        }
    }

    /// Run the player's original clock handler using input from the previous poll.
    /// Map/generation/renderer ordering is supplied by the surrounding scene host.
    pub fn tick_clock(
        &mut self,
        player: &mut ChunkPlayer,
        scene: &SceneCollision,
        math: &PsxMathTables,
        ready: [i16; 7],
        clock: Option<(&mut crate::DreamDay, &crate::DayTables)>,
        signal: crate::FrameSignal,
    ) -> Result<(), SceneCollisionError> {
        if signal != crate::FrameSignal::Tick {
            return Ok(());
        }
        if self.interval == 0 {
            return Err(LinkError::InvalidMovement.into());
        }
        let run_callbacks = if let Some((day, tables)) = clock {
            self.palette.dream = tables.color(day.chart()?);
            let index = usize::try_from(day.day)
                .ok()
                .filter(|&i| i < 365)
                .ok_or(crate::DayFault::InvalidDay)?;
            self.palette.flashback = tables.color(day.history[index]);
            let mut effects = ClockEffects {
                map: &mut self.map,
                commands: &mut self.commands,
                key: self.keys.current,
                palette: self.palette,
                error: None,
            };
            let run = day.tick(
                &mut self.links,
                &mut player.controls,
                &mut self.rng,
                2,
                &mut effects,
            )?;
            if let Some(error) = effects.error {
                return Err(error.into());
            }
            run
        } else {
            self.links.execution.dream_timer = self.links.execution.dream_timer.wrapping_add(1);
            true
        };
        if run_callbacks && !player.controls.input_locked {
            player.controls.action_pressed = false;
            self.can_dynamic = self
                .links
                .execution
                .dream_timer
                .is_multiple_of(self.interval);
        }
        if !run_callbacks {
            return Ok(());
        }
        player.controls.update_view();
        if self.map.move_callback != 1 {
            return Ok(());
        }
        let intent = player.controls.update_movement_intent();
        self.floor_effect = intent.map(|value| value.floor_effect);
        if let Some(action) = intent.and_then(|value| value.link_action) {
            let rules = self.tables.motion.clone();
            let mut host = MovementHost {
                state: self,
                player,
                scene,
                math,
                ready,
                error: None,
            };
            trigger_link_action(&mut host, &rules, action)?;
            if let Some(error) = host.error {
                return Err(error);
            }
        }
        Ok(())
    }

    /// The stream/context host controls when its original position cache is refreshed.
    pub fn refresh_key(
        &mut self,
        scene: &SceneCollision,
        position: [i32; 3],
    ) -> Result<(), SceneCollisionError> {
        self.keys.current_spawn(true, &scene.slots, position)?;
        Ok(())
    }
}

struct ClockEffects<'a> {
    map: &'a mut MapLinkPolicy,
    commands: &'a mut Vec<MapLinkCommand>,
    key: SpawnLocation,
    palette: LinkPalette,
    error: Option<LinkError>,
}
impl crate::DreamDayEffects for ClockEffects<'_> {
    fn notify(
        &mut self,
        _day: &mut crate::DreamDay,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        code: i32,
    ) {
        match self.map.notify(links, controls, code, self.palette) {
            Ok(commands) => self.commands.extend(commands),
            Err(error) => self.error = Some(error),
        }
    }
    fn attached_actor(&self) -> bool {
        true
    }
    fn current_spawn(
        &mut self,
        _day: &mut crate::DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
    ) -> SpawnLocation {
        self.key
    }
}

struct Effects<'a> {
    position: &'a mut [i32; 3],
    map: &'a mut MapLinkPolicy,
    commands: &'a mut Vec<MapLinkCommand>,
    sounds: &'a mut Vec<[i32; 3]>,
    saved: SavedPlayerPose,
    grid: crate::SceneGrid,
    base: [i32; 3],
    palette: LinkPalette,
    error: Option<LinkError>,
}
impl DreamLinkEffects for Effects<'_> {
    fn notify(&mut self, state: &mut DreamLinks, controls: &mut PlayerControls, event: i32) {
        match self.map.notify(state, controls, event, self.palette) {
            Ok(commands) => self.commands.extend(commands),
            Err(error) => self.error = Some(error),
        }
    }
    fn sound(
        &mut self,
        _state: &mut DreamLinks,
        _controls: &mut PlayerControls,
        parameters: [i32; 3],
    ) {
        self.sounds.push(parameters);
    }
    fn rotate(
        &mut self,
        _state: &mut DreamLinks,
        controls: &mut PlayerControls,
        absolute: bool,
        packet: [u32; 3],
    ) {
        if let Err(error) = controls.apply_rotation_packet(absolute, packet) {
            self.error = Some(error);
        }
    }
    fn translate(
        &mut self,
        _state: &mut DreamLinks,
        _controls: &mut PlayerControls,
        delta: [i32; 3],
    ) {
        *self.position = std::array::from_fn(|i| self.position[i].wrapping_add(delta[i]));
    }
    fn relocate(
        &mut self,
        _state: &mut DreamLinks,
        _controls: &mut PlayerControls,
        spawn: SpawnLocation,
    ) {
        *self.position = self.grid.place_spawn(self.base, spawn).position;
    }
}
impl DreamContactEffects for Effects<'_> {
    // Scene floor queries deliver -1/-2; the inherited native handler has no effect for those codes.
    fn base_floor(&mut self, _state: &mut DreamLinks, _controls: &mut PlayerControls, _code: i32) {}
    // The movement host retains every geometry contact before invoking the wall policy.
    fn base_wall(&mut self, _state: &mut DreamLinks, _controls: &mut PlayerControls, _code: i32) {}
    fn restore_link_pose(&mut self, _state: &mut DreamLinks, controls: &mut PlayerControls) {
        *self.position = self.saved.position;
        controls.angles = self.saved.angles;
    }
}

struct MovementHost<'a> {
    state: &'a mut LinkedPlayer,
    player: &'a mut ChunkPlayer,
    scene: &'a SceneCollision,
    math: &'a PsxMathTables,
    ready: [i16; 7],
    error: Option<SceneCollisionError>,
}
impl MovementHost<'_> {
    fn special(&mut self, key: SpawnLocation, kind: u8) -> i32 {
        if self.error.is_some() {
            return 1;
        }
        let state = &mut *self.state;
        let mut effects = Effects {
            position: &mut self.player.position,
            map: &mut state.map,
            commands: &mut state.commands,
            sounds: &mut state.sounds,
            saved: state.saved,
            grid: self.scene.slots.grid,
            base: state.base,
            palette: state.palette,
            error: None,
        };
        let result = match kind {
            0 => state.links.staircase_at(
                &mut self.player.controls,
                &state.tables.catalog,
                &mut state.context,
                key,
                &state.tables.auto_move,
                &mut effects,
            ),
            1 => state.links.teleport_at(
                &mut self.player.controls,
                &state.tables.catalog,
                &mut state.context,
                key,
                &mut effects,
            ),
            _ => state.links.tunnel_at(
                &mut self.player.controls,
                &state.tables.catalog,
                &mut state.context,
                key,
                &mut effects,
            ),
        };
        if let Some(error) = effects.error {
            self.error = Some(error.into());
            return 1;
        }
        match result {
            Ok(value) => i32::from(value),
            Err(error) => {
                self.error = Some(error.into());
                1
            }
        }
    }
    fn wall(&mut self, code: i32) -> Result<(), SceneCollisionError> {
        self.state.wall_contact(self.player, self.scene, code)
    }
    fn motion(&mut self, request: MotionRequest) -> Result<(), SceneCollisionError> {
        let local = if request.axis == MotionAxis::Forward {
            [0, 0, request.amount as i16]
        } else {
            [request.amount as i16, 0, 0]
        };
        let delta = self
            .math
            .rotation(self.player.controls.angles)
            .apply_short(local);
        self.player.position =
            std::array::from_fn(|i| self.player.position[i].wrapping_add(delta[i]));
        self.state.body_contact = None;
        if request.collision_and_links {
            let mut query = BodyQueryState {
                position: self.player.position,
                heading: self.player.controls.angles[1],
                bounds: self.player.bounds,
                motion: request,
            };
            let scene = self.scene;
            let math = self.math;
            let contact = scene.body_with(&mut query, math, |query, event| {
                match event {
                    BodyVisit::Cell { cell, .. } => self.state.keys.stamp_body_candidate(cell),
                    BodyVisit::Hit { contact, .. } => {
                        self.state.body_contact = Some(contact);
                        self.wall(4)?;
                        query.position = self.player.position;
                        query.heading = self.player.controls.angles[1];
                    }
                }
                Ok(())
            })?;
            self.state.body_contact = contact;
        }
        if self.state.body_contact.is_some() {
            return Ok(());
        }
        let Some(location) = self.scene.slots.locate(self.player.position)? else {
            return Ok(());
        };
        let contact = self.scene.floor(self.player.position, self.math)?;
        self.state.floor_contact = contact;
        let flags = if let Some(contact) = contact {
            self.player.position = contact.geometry.position;
            self.scene.chunks[contact.slot].as_ref().unwrap().cells[contact.geometry.cell]
                [contact.geometry.tile]
                .flags
        } else {
            0
        };
        let state = &mut *self.state;
        let mut effects = Effects {
            position: &mut self.player.position,
            map: &mut state.map,
            commands: &mut state.commands,
            sounds: &mut state.sounds,
            saved: state.saved,
            grid: self.scene.slots.grid,
            base: state.base,
            palette: state.palette,
            error: None,
        };
        let response = state.links.floor_event(
            &mut self.player.controls,
            &state.tables.catalog,
            &mut state.context,
            FloorLinkRequest {
                code: if contact.is_some() { -1 } else { -2 },
                tile_flags: flags,
                owner_ready: Some(self.ready[location.slot]),
                current_key: state.keys.current,
                previous_surface: state.floor_surface,
            },
            &mut state.rng,
            &mut effects,
        )?;
        state.floor_surface = response.surface;
        if let Some(error) = effects.error {
            return Err(error.into());
        }
        Ok(())
    }
}
impl LinkActionHost for MovementHost<'_> {
    fn move_direction(&self) -> i32 {
        self.player.controls.move_direction
    }
    fn auto_move_active(&self) -> bool {
        self.player.controls.auto_move_active
    }
    fn stage(&self) -> i32 {
        self.state.links.execution.stage
    }
    fn position(&self) -> [i32; 3] {
        self.player.position
    }
    fn hook(&mut self) {}
    fn spawn_key(&mut self) -> SpawnLocation {
        self.state.keys.current
    }
    fn staircase(&mut self, key: SpawnLocation) -> i32 {
        self.special(key, 0)
    }
    fn teleport(&mut self, key: SpawnLocation) -> i32 {
        self.special(key, 1)
    }
    fn tunnel(&mut self, key: SpawnLocation) -> i32 {
        self.special(key, 2)
    }
    fn save_link_state(&mut self) {
        self.state.saved = SavedPlayerPose {
            position: self.player.position,
            angles: self.player.controls.angles,
        };
    }
    fn apply_motion(&mut self, request: MotionRequest) {
        if self.error.is_none()
            && let Err(error) = self.motion(request)
        {
            self.error = Some(error);
        }
    }
    fn wall_link(&mut self, code: i32) {
        if self.error.is_none()
            && let Err(error) = self.wall(code)
        {
            self.error = Some(error);
        }
    }
    fn clear_transform_marker(&mut self) {}
}
