use crate::StreamedStage;
use lsd_core::{
    ChunkPlayer, DayStartInputs, DayTables, DreamDay, DreamDayEffects, DreamLinks, DreamSession,
    DreamSessionHost, LinkedPlayer, MapEffect, PlayerControls, SceneMotionReset, SpawnLocation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageSessionEvent {
    StopMap,
    DestroyMap,
    CreateMap(i32),
    AttachMap,
    RunMap,
    MoodRequested(SpawnLocation),
    DayServicePending(i32),
    DayFinished(i32),
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
        }
    }
}
impl StageLinkSession {
    /// Menu/session handoff starts another dream while retaining calendar, scores and RNG.
    pub fn begin_next_dream(&mut self) -> Result<(), &'static str> {
        if self.day.is_none() {
            return Err("No recovered day state is attached");
        }
        self.policy.state = 1;
        self.finished = None;
        self.pending_day_event = None;
        Ok(())
    }

    pub fn begin_original_day(&mut self, linked: &mut LinkedPlayer, tables: DayTables) {
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
        if self.finished.is_some() || (self.pending_day_event.is_some() && self.day.is_none()) {
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
        };
        if let Some(code) = self.pending_day_event.take() {
            self.policy.link_code(&mut host, 1, code)?;
            if host.finished.is_some() {
                *host.finished = Some(self.policy.result);
                return Ok(false);
            }
        }
        self.policy.tick(&mut host, 2)?;
        if host.finished.is_some() {
            *host.finished = Some(self.policy.result);
        }
        if host.rebuilt {
            self.rebuilds = self.rebuilds.wrapping_add(1);
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
