//! Original dream-system journal consequences for entity notifications.
use crate::{DayFault, DreamDay, DreamLinks, Mood, PlayerControls, PsxRng};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorJournalEvent {
    pub sender_type: u32,
    pub sender_identity: u64,
    pub code: i32,
    pub fields: [i8; 12],
}
pub trait ActorJournalHost {
    fn base_journal(&mut self, event: ActorJournalEvent);
    /// Return the actor's synchronous link/video/end notification, when emitted.
    fn acknowledge_actor(&mut self, code: i32) -> Option<i32>;
    fn capture_flashback(
        &mut self,
        day: &mut DreamDay,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault>;
    fn dynamic_link(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault>;
}
pub fn actor_link_stage(raw: i8) -> i32 {
    if raw < 0 {
        !i32::from(raw)
    } else {
        i32::from(raw) - 1
    }
}
pub fn actor_journal(
    event: ActorJournalEvent,
    day: &mut DreamDay,
    links: &mut DreamLinks,
    controls: &mut PlayerControls,
    rng: &mut PsxRng,
    host: &mut impl ActorJournalHost,
) -> Result<(), DayFault> {
    host.base_journal(event);
    if event.sender_type & 0xfffff != 0x1f234 || links.execution.state != 0 {
        return Ok(());
    }
    match event.code {
        4 => {
            if let Some(code) = host.acknowledge_actor(4) {
                actor_journal(
                    ActorJournalEvent { code, ..event },
                    day,
                    links,
                    controls,
                    rng,
                    host,
                )?;
            }
        }
        9 => {
            if !links.execution.flashback {
                day.instances.log(Mood([event.fields[0], event.fields[1]]));
                day.instance_score = day
                    .instance_score
                    .wrapping_add(i32::from(event.fields[2]) * 1000);
                host.capture_flashback(day, links, controls, rng)?;
            }
        }
        10 => {
            let previous = links.execution.stage;
            links.execution.stage = actor_link_stage(event.fields[7]).wrapping_neg();
            host.dynamic_link(links, controls, rng)?;
            if links.execution.stage < 0 {
                links.execution.stage = previous;
            }
        }
        11 => {
            if !links.execution.flashback {
                day.cinematic[0] = -1;
                links.execution.dream_timer = links.time_limit as u32;
                day.cinematic[1] = (i32::from(event.fields[8]) - 1) as i16;
            }
        }
        12 if !links.execution.flashback => {
            links.execution.dream_timer = links.time_limit as u32;
        }
        _ => {}
    }
    Ok(())
}
