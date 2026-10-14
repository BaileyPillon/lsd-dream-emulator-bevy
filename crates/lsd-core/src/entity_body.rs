use crate::{BehaviorFault, EntityBehavior, EntityScriptHost, GenerationSound, ModelActor, PsxRng};

/// Typed identity replaces a native body pointer; type bits retain vtable routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyPeer {
    pub type_id: u32,
    pub identity: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityBodyEvent {
    pub sender: BodyPeer,
    pub code: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntityBodyPlan {
    pub query_geometry: bool,
    pub store_contact: bool,
    pub disable_link: bool,
    pub notification: Option<i32>,
}
/// Original transform/scene/entity body notification gates and inherited ordering.
pub fn entity_body_plan(fields: [i8; 12], event: EntityBodyEvent) -> EntityBodyPlan {
    if event.sender.type_id & 15 != 4 {
        return EntityBodyPlan::default();
    }
    let interaction = event.sender.type_id as u8 == 0x34;
    let done = event.sender.type_id as u8 == 0x24;
    let raw_stage = fields[7];
    if !done && (!interaction || ((2..=8).contains(&event.code) && raw_stage <= 0)) {
        return EntityBodyPlan::default();
    }
    let notification = if interaction && event.code == 4 && raw_stage > 0 {
        Some(if raw_stage != 127 {
            10
        } else if fields[8] != 0 {
            11
        } else {
            12
        })
    } else {
        None
    };
    EntityBodyPlan {
        query_geometry: matches!(event.code, 2 | 3)
            || (interaction && (5..=8).contains(&event.code)),
        store_contact: event.code == 4,
        disable_link: done && event.code == 4,
        notification,
    }
}
pub trait EntityBodyHost: EntityScriptHost {
    /// Execute the inherited geometry query against this identified peer.
    fn query_body(
        &mut self,
        actor: &mut ModelActor,
        event: EntityBodyEvent,
    ) -> Result<(), BehaviorFault>;
}
impl EntityBehavior {
    pub fn receive_body(
        &mut self,
        actor: &mut ModelActor,
        fields: [i8; 12],
        event: EntityBodyEvent,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
        host: &mut impl EntityBodyHost,
    ) -> Result<(), BehaviorFault> {
        let plan = entity_body_plan(fields, event);
        if plan.store_contact {
            actor.body_contact.current = Some(event.sender.into());
        }
        if plan.query_geometry {
            host.query_body(actor, event)?;
        }
        if plan.disable_link {
            self.disable_link(actor, sound);
        }
        if let Some(code) = plan.notification {
            host.notify_actor(actor, rng, code)?;
        }
        Ok(())
    }
}
