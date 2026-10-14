use crate::{EncounterSubscription, StageLinkSession, StreamedStage};
use lsd_core::{
    ChunkPlayer, FrameSignal, FrameSubscribers, LinkedPlayer, MapCompletionCommand, MapEffect,
    PsxMathTables,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneClockMember {
    Actor(u64),
    Effect,
    Map,
    Stream,
    Renderer,
    Session,
    Player,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneFrameVisit {
    Clock(SceneClockMember),
    Pad,
    Handoff,
}
pub(crate) fn initial_clock_members() -> FrameSubscribers<SceneClockMember> {
    let mut members = FrameSubscribers::default();
    for member in [
        SceneClockMember::Player,
        SceneClockMember::Session,
        SceneClockMember::Renderer,
        SceneClockMember::Stream,
        SceneClockMember::Map,
    ] {
        members.subscribe(member);
    }
    members
}
impl StageLinkSession {
    pub(crate) fn reset_frame(&mut self) {
        if let Some(encounters) = &mut self.encounters {
            encounters.stop_with_audio(&mut self.encounter_audio);
            encounters.take_frame_changes();
        }
        self.encounter_frame.reset(0);
        self.frame_members = initial_clock_members();
        self.frame_trace.clear();
        self.session_ticks = 0;
        self.rendered_player = None;
        self.rendered_rebuild = self.rebuilds;
    }
    fn sync_frame_members(&mut self, effect: &MapEffect) {
        if let Some(encounters) = &mut self.encounters {
            for change in encounters.take_frame_changes() {
                match change {
                    EncounterSubscription::Attach(serial) => {
                        self.frame_members
                            .subscribe(SceneClockMember::Actor(serial));
                    }
                    EncounterSubscription::Detach(serial) => {
                        self.frame_members.remove(SceneClockMember::Actor(serial));
                    }
                }
            }
        }
        let registered = self.frame_members.contains(SceneClockMember::Effect);
        if effect.render_attached && !registered {
            self.frame_members.subscribe(SceneClockMember::Effect);
        } else if !effect.render_attached && registered {
            self.frame_members.remove(SceneClockMember::Effect);
        }
    }
    /// One display-driven simulation step in the imported scene.
    /// The renderer phase records presentation state; original GPU/CD timing is separate.
    pub fn step_frame(
        &mut self,
        stage: &mut StreamedStage,
        linked: &mut LinkedPlayer,
        player: &mut ChunkPlayer,
        math: &PsxMathTables,
        effect: &mut MapEffect,
        buttons: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.events.clear();
        self.frame_trace.clear();
        if self.finished.is_some() {
            player.controls.input_locked = true;
            return Ok(());
        }
        linked.begin_frame();
        self.sync_frame_members(effect);
        let signal = self.encounter_frame.advance();
        self.frame_members.begin_notification();
        let result = (|| {
            while let Some(member) = self.frame_members.next_subscriber() {
                self.frame_trace.push(SceneFrameVisit::Clock(member));
                match member {
                    SceneClockMember::Actor(serial) => {
                        self.tick_encounter_actor(serial, signal, linked, player, math, effect)?;
                    }
                    SceneClockMember::Effect => {
                        for command in
                            effect.tick_signal(&mut linked.map, &mut player.controls, signal)?
                        {
                            if let MapCompletionCommand::Notify(code) = command {
                                self.receive(code);
                            }
                        }
                    }
                    SceneClockMember::Map => {
                        linked.map.tick = linked.map.tick.wrapping_add(1);
                        self.tick_generation_actors(stage, linked, &player.controls)?;
                    }
                    SceneClockMember::Stream => {
                        stage.complete_frame()?;
                        stage.update_for_position(player.position)?;
                        let previous_chunk = linked.keys.current.chunk;
                        linked.refresh_key(&stage.collision, player.position)?;
                        if previous_chunk != linked.keys.current.chunk {
                            self.log_current_chunk(linked)?;
                        }
                        self.bind_encounters(stage, linked, player, math)?;
                    }
                    SceneClockMember::Renderer if signal != FrameSignal::Finished => {
                        self.rendered_player = Some(player.clone());
                        self.rendered_rebuild = self.rebuilds;
                    }
                    SceneClockMember::Renderer => {}
                    SceneClockMember::Session => {
                        self.session_ticks = self.session_ticks.wrapping_add(1);
                    }
                    SceneClockMember::Player => {
                        let first_command = linked.commands.len();
                        let ready = std::array::from_fn(|i| stage.protocol.slots[i].ready);
                        linked.tick_clock(
                            player,
                            &stage.collision,
                            math,
                            ready,
                            self.day.as_mut().zip(self.day_tables.as_ref()),
                            signal,
                        )?;
                        for command in linked.commands[first_command..].iter().copied() {
                            for command in effect.consume(command)? {
                                if let MapCompletionCommand::Notify(code) = command {
                                    self.receive(code);
                                }
                            }
                        }
                    }
                }
                self.sync_frame_members(effect);
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        })();
        self.frame_members.end_notification();
        result?;
        self.frame_trace.push(SceneFrameVisit::Pad);
        LinkedPlayer::poll_pad(player, buttons);
        self.frame_trace.push(SceneFrameVisit::Handoff);
        if self.tick_services(stage, player, linked, effect)? {
            if let Some(encounters) = &mut self.encounters {
                encounters.stop_with_audio(&mut self.encounter_audio);
            }
            self.frame_members.remove(SceneClockMember::Map);
            self.frame_members.subscribe(SceneClockMember::Map);
            self.rendered_player = None;
        }
        self.sync_frame_members(effect);
        Ok(())
    }
}
