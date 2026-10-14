use crate::StreamedStage;
use lsd_core::{
    ChunkPlayer, DreamSession, DreamSessionHost, LinkedPlayer, MapEffect, SceneMotionReset,
    SpawnLocation,
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
        }
    }
}
impl StageLinkSession {
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
        if self.pending_day_event.is_some() {
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
        };
        self.policy.tick(&mut host, 2)?;
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
        Err("Original start-day service is pending".into())
    }
    fn end_day(&mut self, _mode: i32) -> Result<i32, Self::Error> {
        Err("Original end-day service is pending".into())
    }
    fn cinematic(&mut self) -> Result<[i16; 4], Self::Error> {
        Err("Original cinematic service is pending".into())
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
        self.effect.start_scene(
            self.linked.links.execution.flashback,
            self.linked.palette.flashback,
        )?;
        self.rebuilt = true;
        Ok(())
    }
    fn finish(&mut self, _code: i32) -> Result<(), Self::Error> {
        Err("Original scene finish service is pending".into())
    }
}
