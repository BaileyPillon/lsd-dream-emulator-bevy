use crate::{
    ActorAxis, ActorContact, BehaviorFault, EntityScriptHost, ModelActor, PsxRng, SceneContact,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorFloorQuery {
    Unlocated,
    Miss,
    Hit(SceneContact),
}

/// Original forward/sideways wrapper: clear contact, move/notify, then follow floor
/// only if the synchronous movement callback did not already assign a contact.
pub fn move_actor_on_floor(
    actor: &mut ModelActor,
    axis: ActorAxis,
    amount: i32,
    notify: bool,
    rng: &mut PsxRng,
    host: &mut impl EntityScriptHost,
) -> Result<(), BehaviorFault> {
    actor.body_contact.current = None;
    let matrix = host.movement_matrix(actor.pose.angles)?;
    actor.move_local(axis, amount, matrix);
    if notify {
        host.notify_actor(actor, rng, axis.notification())?;
    }
    if actor.body_contact.current.is_some() {
        return Ok(());
    }
    match host.actor_floor_query(actor.pose.position)? {
        ActorFloorQuery::Unlocated => {}
        ActorFloorQuery::Miss => host.actor_floor_signal(actor, -2)?,
        ActorFloorQuery::Hit(contact) => {
            actor.body_contact.current = Some(ActorContact::Floor(contact));
            let delta = std::array::from_fn(|axis| {
                contact.geometry.position[axis].wrapping_sub(actor.pose.position[axis])
            });
            actor.translate(delta, false);
            host.actor_floor_signal(actor, -1)?;
        }
    }
    Ok(())
}
