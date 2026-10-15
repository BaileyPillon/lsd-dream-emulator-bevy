use crate::{EncounterScene, EntityProperties, GenerationPaths, StreamedStage};
use lsd_core::{
    ChunkPlayer, DayStartInputs, DayTables, DreamDay, DreamDayEffects, DreamGeneration, DreamLinks,
    DreamSession, DreamSessionHost, GenerationEffectTables, GenerationFrame,
    GenerationObjectTables, GenerationSound, GenerationTables, LinkedPlayer, MapEffect,
    PlayerControls, SceneMotionReset, SpawnLocation, VramMove, choose_music, choose_texture,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageSessionEvent {
    ActorEffectCompleted {
        entity: u8,
        code: i32,
    },
    ActorDestroyed {
        entity: u8,
    },
    ActorFlashbackReset {
        entity: u8,
    },
    StopMap,
    DestroyMap,
    CreateMap(i32),
    AttachMap,
    RunMap,
    MoodRequested(SpawnLocation),
    ActorJournal {
        entity: u8,
        code: i32,
        applied: bool,
    },
    ActorBodyQuery {
        entity: u8,
        code: i32,
        contact: bool,
    },
    DayServicePending(i32),
    DayFinished(i32),
    ContentSelected {
        texture: char,
        music: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneContentChoice {
    pub stage: u8,
    pub texture: char,
    pub texture_path: String,
    pub music_index: usize,
    pub music_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationAudioRequest {
    Stop(i32),
    VolumeOffset(i32),
    Play {
        note: i32,
        period: i32,
        counter: i32,
    },
}
/// A connected shared backend returns real voice handles synchronously. With
/// no backend, retain requests and return the original failed-voice handle (-1).
#[derive(Debug, Clone, Default)]
pub struct GenerationAudioRequests {
    pub requests: Vec<GenerationAudioRequest>,
    backend: Option<crate::SceneAudio>,
}
impl GenerationAudioRequests {
    /// A shared handle for other callbacks on the same original sound object.
    pub fn connected_backend(&self) -> Option<crate::SceneAudio> {
        self.backend.clone()
    }
    /// Attach before any requests: replaying queued calls cannot recover handles
    /// that actor state already stored as -1. Both scene channels share one backend.
    pub fn connect(&mut self, backend: crate::SceneAudio) -> Result<(), &'static str> {
        if self.backend.is_some() || !self.requests.is_empty() {
            return Err("Connect scene audio before the first sound request");
        }
        self.backend = Some(backend);
        Ok(())
    }
    pub fn take_requests(&mut self) -> Vec<GenerationAudioRequest> {
        std::mem::take(&mut self.requests)
    }
}
impl GenerationSound for GenerationAudioRequests {
    fn stop_note(&mut self, handle: i32) -> i32 {
        if let Some(backend) = &mut self.backend {
            return backend.stop_note(handle);
        }
        self.requests.push(GenerationAudioRequest::Stop(handle));
        -1
    }
    fn volume_offset(&mut self, offset: i32) {
        if let Some(backend) = &mut self.backend {
            backend.volume_offset(offset);
            return;
        }
        self.requests
            .push(GenerationAudioRequest::VolumeOffset(offset));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        if let Some(backend) = &mut self.backend {
            return backend.play_note(note, period, counter);
        }
        self.requests.push(GenerationAudioRequest::Play {
            note,
            period,
            counter,
        });
        -1
    }
}

/// Loaded-scene consumer of the original session policy.
/// Startup/day/cinematic services and original asynchronous construction remain open.
#[derive(Debug, Clone)]
pub struct StageLinkSession {
    pub policy: DreamSession,
    pub rebuilds: u64,
    pub events: Vec<StageSessionEvent>,
    pub pending_day_event: Option<i32>,
    pub last_reset: Option<SceneMotionReset>,
    pub day: Option<DreamDay>,
    pub day_tables: Option<DayTables>,
    pub finished: Option<i32>,
    pub generation: Option<DreamGeneration>,
    pub generation_tables: Option<GenerationTables>,
    pub generation_object_tables: Option<GenerationObjectTables>,
    pub generation_effect_tables: Option<GenerationEffectTables>,
    pub generation_vram_moves: Vec<VramMove>,
    pub generation_paths: Option<GenerationPaths>,
    pub content_choice: Option<SceneContentChoice>,
    pub music_override: i32,
    pub generation_audio: GenerationAudioRequests,
    pub encounters: Option<EncounterScene>,
    pub encounter_audio: GenerationAudioRequests,
    pub journal_body_contact: lsd_core::BodyContactState<lsd_core::BodyPeer>,
    pub player_collision: Option<lsd_core::CollisionModel>,
    pub actor_body_queries: crate::ActorBodyQueryCounts,
    pub encounter_frame: lsd_core::FramePhase,
    pub frame_members: lsd_core::FrameSubscribers<crate::SceneClockMember>,
    pub frame_trace: Vec<crate::SceneFrameVisit>,
    pub session_ticks: u32,
    pub rendered_player: Option<ChunkPlayer>,
    pub rendered_rebuild: u64,
    pub scene_scale: Option<lsd_core::SceneScale>,
    pub rendered_scene_scale: [i32; 3],
}
impl Default for StageLinkSession {
    fn default() -> Self {
        Self {
            policy: DreamSession {
                state: 2,
                result: 0,
            },
            rebuilds: 0,
            events: Vec::new(),
            pending_day_event: None,
            last_reset: None,
            day: None,
            day_tables: None,
            finished: None,
            generation: None,
            generation_tables: None,
            generation_object_tables: None,
            generation_effect_tables: None,
            generation_vram_moves: Vec::new(),
            generation_paths: None,
            content_choice: None,
            music_override: 0,
            generation_audio: Default::default(),
            encounters: None,
            encounter_audio: Default::default(),
            journal_body_contact: lsd_core::BodyContactState { current: None },
            player_collision: None,
            actor_body_queries: Default::default(),
            encounter_frame: Default::default(),
            frame_members: crate::scene_frame::initial_clock_members(),
            frame_trace: Vec::new(),
            session_ticks: 0,
            rendered_player: None,
            rendered_rebuild: 0,
            scene_scale: None,
            rendered_scene_scale: [4096; 3],
        }
    }
}
impl StageLinkSession {
    pub fn enable_encounters(
        &mut self,
        tables: lsd_core::EncounterTables,
        properties: Vec<EntityProperties>,
    ) {
        self.encounters = Some(EncounterScene::new(tables, properties));
    }
    /// Combined binding/update entry point for supplied encounter fixtures.
    pub fn tick_encounters(
        &mut self,
        stage: &StreamedStage,
        linked: &mut LinkedPlayer,
        player: &mut ChunkPlayer,
        math: &lsd_core::PsxMathTables,
        effect: &mut MapEffect,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.bind_encounters(stage, linked, player, math)?;
        let signal = self.encounter_frame.advance();
        self.tick_encounter_callbacks(
            (None, signal),
            &stage.collision,
            linked,
            player,
            math,
            effect,
        )
    }
    /// Stream notifications construct and attach actors without updating them.
    pub fn bind_encounters(
        &mut self,
        stage: &StreamedStage,
        linked: &mut LinkedPlayer,
        player: &ChunkPlayer,
        math: &lsd_core::PsxMathTables,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.scene_scale.is_none() {
            self.scene_scale = Some(lsd_core::SceneScale::new(stage.scene_scale_tables()?));
        }
        let Some(encounters) = &mut self.encounters else {
            return Ok(());
        };
        if self.finished.is_some() || !self.generation.as_ref().is_some_and(|g| g.owner_active) {
            encounters.stop_with_audio(&mut self.encounter_audio);
            return Ok(());
        }
        let generation = self.generation.as_ref().unwrap();
        let day = self
            .day
            .as_mut()
            .ok_or("Encounter channel requires original day state")?;
        let palette = self
            .day_tables
            .as_ref()
            .ok_or("Encounter channel requires original day content")?
            .color(day.chart()?);
        encounters.set_encounter_player(player.position, math.rotation(player.controls.angles));
        encounters.update_with_audio(
            stage,
            crate::EncounterUpdate {
                epoch: self.rebuilds,
                day: day.day.wrapping_add(1),
                palette,
                generation_type: generation.generation_type,
            },
            &mut linked.rng,
            &mut self.encounter_audio,
        )?;
        if let Some(enabled) = encounters.teleports_initialization.take() {
            linked.context.teleports_enabled = i32::from(enabled);
        }
        if std::mem::take(&mut encounters.teleports_requested) {
            linked.context.teleports_enabled = 1;
        }
        Ok(())
    }
    /// Execute one entity at its current position in the shared clock list.
    pub fn tick_encounter_actor(
        &mut self,
        clock_actor: (u64, lsd_core::FrameSignal),
        scene: &lsd_core::SceneCollision,
        linked: &mut LinkedPlayer,
        player: &mut ChunkPlayer,
        math: &lsd_core::PsxMathTables,
        effect: &mut MapEffect,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.tick_encounter_callbacks(
            (Some(clock_actor.0), clock_actor.1),
            scene,
            linked,
            player,
            math,
            effect,
        )
    }
    fn tick_encounter_callbacks(
        &mut self,
        clock_actor: (Option<u64>, lsd_core::FrameSignal),
        scene: &lsd_core::SceneCollision,
        linked: &mut LinkedPlayer,
        player: &mut ChunkPlayer,
        math: &lsd_core::PsxMathTables,
        effect: &mut MapEffect,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (serial, signal) = clock_actor;
        let Some(encounters) = &mut self.encounters else {
            return Ok(());
        };
        if self.finished.is_some() || !self.generation.as_ref().is_some_and(|g| g.owner_active) {
            encounters.stop_with_audio(&mut self.encounter_audio);
            return Ok(());
        }
        let day = self
            .day
            .as_mut()
            .ok_or("Encounter channel requires original day state")?;
        let target = lsd_core::EntityTarget {
            position: player.position,
            local_y: player.position[1],
            matrix: math.rotation(player.controls.angles),
            eye: player.controls.eye,
            target: player.controls.target,
        };
        let mut pending = std::mem::take(&mut encounters.pending_notifications);
        let mut services = crate::ActorJournalServices::new(
            &linked.tables.catalog,
            crate::ActorJournalBindings {
                context: &mut linked.context,
                map: &mut linked.map,
                palette: linked.palette,
                commands: &mut linked.commands,
                sounds: &mut linked.sounds,
                current_key: linked.keys.current,
                attached: true,
                pending: &mut pending,
                events: &mut self.events,
                body_contact: &mut self.journal_body_contact,
            },
        );
        if let Some(model) = &self.player_collision {
            services.body_query = Some(crate::ActorJournalBody {
                model,
                position: player.position,
                inverse: math.inverse_rotation_chain(player.controls.angles, &[]),
                policy: lsd_core::CollisionPolicy::for_stage(linked.links.execution.stage as u8),
                math,
                counts: &mut self.actor_body_queries,
            });
        }
        services.view_callback = Some(&mut linked.view_callback);
        services.link_audio = self.encounter_audio.connected_backend();
        services.floor_scene = Some(scene);
        services.scene_scale = self.scene_scale.as_mut();
        services.player_motion = Some(crate::ActorJournalPlayerMotion {
            position: &mut player.position,
            move_distance: &mut linked.move_distance,
            math,
        });
        let first_command = services.commands.len();
        let mut events = crate::ActorJournalDispatcher {
            day,
            tables: self.day_tables.as_ref().unwrap(),
            links: &mut linked.links,
            controls: &mut player.controls,
            services,
        };
        let result = if let Some(serial) = serial {
            encounters.tick_actor_frame(
                crate::ActorFrame {
                    serial,
                    target,
                    signal,
                },
                &mut linked.rng,
                &mut self.encounter_audio,
                math,
                &mut events,
            )
        } else {
            encounters.tick_frame(
                target,
                &mut linked.rng,
                &mut self.encounter_audio,
                math,
                &mut events,
                signal,
            )
        };
        encounters.pending_notifications = pending;
        result?;
        linked.palette.dream = self.day_tables.as_ref().unwrap().color(day.chart()?);
        for command in linked.commands[first_command..].iter().copied() {
            for signal in effect.consume(command)? {
                if let lsd_core::MapCompletionCommand::Notify(code) = signal {
                    self.receive(code);
                }
            }
        }
        Ok(())
    }
    pub fn enable_generation(&mut self, tables: GenerationTables, paths: GenerationPaths) {
        self.generation = Some(DreamGeneration::default());
        self.generation_tables = Some(tables);
        self.generation_paths = Some(paths);
    }
    pub fn enable_generated_objects(&mut self, tables: GenerationObjectTables) {
        self.generation_object_tables = Some(tables);
    }
    pub fn enable_generation_layers(&mut self, tables: GenerationEffectTables) {
        self.generation_effect_tables = Some(tables);
    }
    pub fn take_generation_vram_moves(&mut self) -> Vec<VramMove> {
        std::mem::take(&mut self.generation_vram_moves)
    }

    /// Original generated-object channel precedes the two proximity sound slots.
    /// The recovered layer and palette channels precede sound when enabled.
    /// Original global scheduling and GPU presentation require further work.
    pub fn tick_generation_actors(
        &mut self,
        stage: &StreamedStage,
        linked: &mut LinkedPlayer,
        controls: &PlayerControls,
    ) -> Result<(), lsd_core::GenerationFault> {
        if self.finished.is_some() {
            return Ok(());
        }
        if let (Some(generation), Some(tables)) = (&mut self.generation, &self.generation_tables)
            && generation.owner_active
        {
            let position = stage
                .collision
                .slots
                .grid
                .place_spawn([0; 3], linked.keys.current)
                .position;
            if let Some(objects) = &self.generation_object_tables {
                // Original Renderer + 0x18 is eye Y; constructor/update uses
                // its change from the initial eye Y, rather than the style height.
                if let Some(geometry) = &self.generation_effect_tables {
                    self.generation_vram_moves
                        .extend(generation.tick_objects_and_layers(
                            tables,
                            objects,
                            geometry,
                            &mut linked.rng,
                            GenerationFrame {
                                position,
                                view: [controls.eye[1], controls.target[1]],
                            },
                        )?);
                } else {
                    generation.tick_structures(
                        tables,
                        objects,
                        &mut linked.rng,
                        position,
                        controls.eye[1],
                    )?;
                }
            } else {
                generation.timer = generation.timer.wrapping_add(1);
            }
            generation.tick_actors(
                tables,
                stage.collision.slots.grid,
                [0; 3],
                Some(position),
                0,
                &mut self.generation_audio,
            )?;
        }
        Ok(())
    }
    /// Menu/session handoff starts another dream while retaining calendar, scores and RNG.
    pub fn begin_next_dream(&mut self) -> Result<(), &'static str> {
        if self.day.is_none() {
            return Err("No recovered day state is attached");
        }
        self.policy.state = 1;
        self.finished = None;
        self.pending_day_event = None;
        self.reset_frame();
        Ok(())
    }

    pub fn begin_original_day(&mut self, linked: &mut LinkedPlayer, tables: DayTables) {
        self.reset_frame();
        let mut day = DreamDay::default();
        day.init_new_game(&tables, &mut linked.context);
        self.day = Some(day);
        self.day_tables = Some(tables);
        self.policy.state = 1;
        self.pending_day_event = None;
        self.finished = None;
    }

    pub fn log_current_chunk(&mut self, linked: &LinkedPlayer) -> Result<(), lsd_core::DayFault> {
        if let (Some(day), Some(tables)) = (&mut self.day, &self.day_tables) {
            day.log_chunk(tables, linked.links.execution.stage, linked.keys.current)?;
        }
        Ok(())
    }
    pub fn receive(&mut self, code: i32) {
        if matches!(code, 4 | 12 | 13) {
            self.pending_day_event = Some(code);
            self.events.push(StageSessionEvent::DayServicePending(code));
        } else if matches!(code, 5 | 6 | 7 | 8 | 10) {
            self.policy.state = 3;
        }
    }

    pub fn tick(
        &mut self,
        stage: &mut StreamedStage,
        player: &mut ChunkPlayer,
        linked: &mut LinkedPlayer,
        effect: &mut MapEffect,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        self.events.clear();
        self.tick_services(stage, player, linked, effect)
    }
    pub(crate) fn tick_services(
        &mut self,
        stage: &mut StreamedStage,
        player: &mut ChunkPlayer,
        linked: &mut LinkedPlayer,
        effect: &mut MapEffect,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.finished.is_some() || (self.pending_day_event.is_some() && self.day.is_none()) {
            if let Some(encounters) = &mut self.encounters {
                encounters.stop_with_audio(&mut self.encounter_audio);
            }
            return Ok(false);
        }
        let mut host = Host {
            stage,
            player,
            linked,
            effect,
            events: &mut self.events,
            reset: &mut self.last_reset,
            position: None,
            rebuilt: false,
            day: self.day.as_mut(),
            tables: self.day_tables.as_ref(),
            finished: &mut self.finished,
            generation: self.generation.as_mut(),
            generation_tables: self.generation_tables.as_ref(),
            generation_paths: self.generation_paths.as_ref(),
            content_choice: &mut self.content_choice,
            music_override: &mut self.music_override,
            generation_audio: &mut self.generation_audio,
        };
        if let Some(code) = self.pending_day_event.take() {
            self.policy.link_code(&mut host, 1, code)?;
            if host.finished.is_some() {
                *host.finished = Some(self.policy.result);
                if let Some(encounters) = &mut self.encounters {
                    encounters.stop_with_audio(&mut self.encounter_audio);
                }
                return Ok(false);
            }
        }
        self.policy.tick(&mut host, 2)?;
        if host.finished.is_some() {
            *host.finished = Some(self.policy.result);
            if let Some(encounters) = &mut self.encounters {
                encounters.stop_with_audio(&mut self.encounter_audio);
            }
        }
        if host.rebuilt {
            self.rebuilds = self.rebuilds.wrapping_add(1);
            self.scene_scale = None;
            self.rendered_scene_scale = [4096; 3];
        }
        Ok(host.rebuilt)
    }
}
struct Host<'a> {
    stage: &'a mut StreamedStage,
    player: &'a mut ChunkPlayer,
    linked: &'a mut LinkedPlayer,
    effect: &'a mut MapEffect,
    events: &'a mut Vec<StageSessionEvent>,
    reset: &'a mut Option<SceneMotionReset>,
    position: Option<[i32; 3]>,
    rebuilt: bool,
    day: Option<&'a mut DreamDay>,
    tables: Option<&'a DayTables>,
    finished: &'a mut Option<i32>,
    generation: Option<&'a mut DreamGeneration>,
    generation_tables: Option<&'a GenerationTables>,
    generation_paths: Option<&'a GenerationPaths>,
    content_choice: &'a mut Option<SceneContentChoice>,
    music_override: &'a mut i32,
    generation_audio: &'a mut GenerationAudioRequests,
}
impl DreamSessionHost for Host<'_> {
    type Error = Box<dyn std::error::Error>;
    fn base_tick(&mut self, _session: &mut DreamSession, _code: i32) -> Result<(), Self::Error> {
        Ok(())
    }
    fn base_notify(&mut self, _session: &mut DreamSession, _code: i32) -> Result<(), Self::Error> {
        Ok(())
    }
    fn sender_type(&self) -> u32 {
        0x2f230
    }
    fn start_day(&mut self) -> Result<i32, Self::Error> {
        let day = self
            .day
            .as_deref_mut()
            .ok_or("Original start-day service is pending")?;
        let tables = self.tables.ok_or("Missing original day content")?;
        self.player
            .controls
            .apply_rotation_packet(true, tables.start_rotation)?;
        self.player.controls.eye = [0, -1200, 0];
        self.player.controls.target = [0, -1200, 10000];
        day.start(
            &mut self.linked.links,
            &mut self.player.controls,
            DayStartInputs {
                tables,
                catalog: &self.linked.tables.catalog,
                context: &mut self.linked.context,
                rng: &mut self.linked.rng,
            },
            &mut StartupEffects,
        )
        .map_err(Into::into)
    }
    fn end_day(&mut self, mode: i32) -> Result<i32, Self::Error> {
        self.day
            .as_deref_mut()
            .ok_or("Original end-day service is pending")?
            .end(
                &self.linked.links,
                self.tables.ok_or("Missing original day content")?,
                &mut self.linked.context,
                mode,
            )
            .map_err(Into::into)
    }
    fn cinematic(&mut self) -> Result<[i16; 4], Self::Error> {
        let value = self
            .day
            .as_deref()
            .ok_or("Original cinematic service is pending")?
            .cinematic;
        Ok([value[0], value[1], 0, 0])
    }
    fn current_map(&mut self) -> Result<i32, Self::Error> {
        Ok(self.linked.links.execution.stage)
    }
    fn stop_map(&mut self, _source: Option<u32>) -> Result<(), Self::Error> {
        self.events.push(StageSessionEvent::StopMap);
        self.player.controls.input_locked = true;
        if let Some(generation) = self.generation.as_deref_mut() {
            generation.stop_with_sound(self.generation_audio);
        }
        Ok(())
    }
    fn destroy_map(&mut self) -> Result<(), Self::Error> {
        self.events.push(StageSessionEvent::DestroyMap);
        Ok(())
    }
    fn create_map(&mut self, stage: i32) -> Result<(), Self::Error> {
        self.events.push(StageSessionEvent::CreateMap(stage));
        let id =
            u8::try_from(stage).map_err(|_| "Destination stage is outside the original range")?;
        let (next, position) = self.stage.load_destination(id, self.linked.links.spawn)?;
        *self.stage = next;
        self.position = Some(position);
        Ok(())
    }
    fn attach_map(&mut self) -> Result<(), Self::Error> {
        self.events.push(StageSessionEvent::AttachMap);
        Ok(())
    }
    fn run_map(&mut self) -> Result<(), Self::Error> {
        self.events.push(StageSessionEvent::RunMap);
        self.player.position = self
            .position
            .take()
            .ok_or("No prepared destination placement")?;
        if let (Some(generation), Some(tables), Some(paths), Some(day)) = (
            self.generation.as_deref_mut(),
            self.generation_tables,
            self.generation_paths,
            self.day.as_deref(),
        ) {
            let stage = self.stage.stage_id();
            let number = day.day.wrapping_add(1);
            // Original func_80052F10 always selects music before texture,
            // including forced-track selections and the stage-nine remapping.
            let music = choose_music(i32::from(stage), self.music_override, &mut self.linked.rng);
            let texture = choose_texture(number, &mut self.linked.rng)?;
            let texture_path = paths.textures[usize::from(stage)]
                .get(texture)
                .ok_or("Invalid generation texture index")?
                .clone();
            let music_path = paths.music[usize::from(stage)]
                .get(music)
                .ok_or("Invalid generation music index")?
                .clone();
            let texture = char::from(b'A' + texture as u8);
            generation.start(tables, i32::from(stage), number, 0)?;
            *self.content_choice = Some(SceneContentChoice {
                stage,
                texture,
                texture_path,
                music_index: music,
                music_path,
            });
            self.events
                .push(StageSessionEvent::ContentSelected { texture, music });
        }
        if let Some(day) = self.day.as_deref_mut() {
            day.attach_flashback(&mut self.linked.links, &mut self.player.controls)?;
        }
        if self.player.controls.paused != 0
            && let Some(packet) = self.linked.links.target_transform
        {
            self.player.controls.apply_rotation_packet(true, packet)?;
        }
        let reset = self.linked.prepare_link_scene(
            self.player,
            &self.stage.collision,
            self.stage.stage_id(),
        )?;
        self.events
            .push(StageSessionEvent::MoodRequested(reset.mood_key));
        *self.reset = Some(reset);
        if let (Some(day), Some(tables)) = (self.day.as_deref_mut(), self.tables) {
            day.log_chunk(tables, self.linked.links.execution.stage, reset.mood_key)?;
            day.cinematic[1] = reset.cinematic_variant;
            self.linked.palette.dream = tables.color(day.chart()?);
            let index = usize::try_from(day.day)
                .ok()
                .filter(|&i| i < 365)
                .ok_or("Invalid original day index")?;
            self.linked.palette.flashback = tables.color(day.history[index]);
        }
        self.effect.start_scene(
            self.linked.links.execution.flashback,
            self.linked.palette.flashback,
        )?;
        self.rebuilt = true;
        Ok(())
    }
    fn finish(&mut self, code: i32) -> Result<(), Self::Error> {
        self.events.push(StageSessionEvent::DayFinished(code));
        *self.finished = Some(code);
        self.player.controls.input_locked = true;
        Ok(())
    }
}
struct StartupEffects;
impl DreamDayEffects for StartupEffects {
    fn notify(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _code: i32,
    ) {
    }
    fn attached_actor(&self) -> bool {
        false
    }
    fn current_spawn(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
    ) -> SpawnLocation {
        SpawnLocation::default()
    }
}
