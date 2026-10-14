use crate::{
    EncounterEventHost, EncounterNotification, EncounterNotificationKind, StageSessionEvent,
};
use lsd_core::{
    ActorJournalEvent, ActorJournalHost, DayFault, DreamDay, DreamDayEffects, DreamLinkEffects,
    DreamLinks, LinkCatalog, LinkContext, LinkPalette, MapLinkCommand, MapLinkPolicy,
    PlayerControls, PsxRng, SpawnLocation, actor_journal,
};

pub struct ActorJournalServices<'a> {
    pub catalog: &'a LinkCatalog,
    pub context: &'a mut LinkContext,
    pub map: &'a mut MapLinkPolicy,
    pub palette: LinkPalette,
    pub commands: &'a mut Vec<MapLinkCommand>,
    pub sounds: &'a mut Vec<[i32; 3]>,
    pub current_key: SpawnLocation,
    pub attached: bool,
    pub pending: &'a mut Vec<EncounterNotification>,
    pub events: &'a mut Vec<StageSessionEvent>,
    pub body_contact: &'a mut lsd_core::BodyContactState<lsd_core::BodyPeer>,
    source: Option<EncounterNotification>,
    pub body_query: Option<ActorJournalBody<'a>>,
    pub view_callback: Option<&'a mut i32>,
    pub player_motion: Option<ActorJournalPlayerMotion<'a>>,
    pub floor_scene: Option<&'a lsd_core::SceneCollision>,
}
pub struct ActorJournalPlayerMotion<'a> {
    pub position: &'a mut [i32; 3],
    pub move_distance: &'a mut i16,
    pub math: &'a lsd_core::PsxMathTables,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActorBodyQueryCounts {
    pub queries: u64,
    pub contacts: u64,
}
pub struct ActorJournalBody<'a> {
    pub model: &'a lsd_core::CollisionModel,
    pub position: [i32; 3],
    pub inverse: lsd_core::FixedMatrix,
    pub policy: lsd_core::CollisionPolicy,
    pub math: &'a lsd_core::PsxMathTables,
    pub counts: &'a mut ActorBodyQueryCounts,
}
impl<'a> ActorJournalServices<'a> {
    pub fn new(catalog: &'a LinkCatalog, bindings: ActorJournalBindings<'a>) -> Self {
        Self {
            catalog,
            context: bindings.context,
            map: bindings.map,
            palette: bindings.palette,
            commands: bindings.commands,
            sounds: bindings.sounds,
            current_key: bindings.current_key,
            attached: bindings.attached,
            pending: bindings.pending,
            events: bindings.events,
            body_contact: bindings.body_contact,
            source: None,
            body_query: None,
            view_callback: None,
            player_motion: None,
            floor_scene: None,
        }
    }
}
pub struct ActorJournalBindings<'a> {
    pub context: &'a mut LinkContext,
    pub map: &'a mut MapLinkPolicy,
    pub palette: LinkPalette,
    pub commands: &'a mut Vec<MapLinkCommand>,
    pub sounds: &'a mut Vec<[i32; 3]>,
    pub current_key: SpawnLocation,
    pub attached: bool,
    pub pending: &'a mut Vec<EncounterNotification>,
    pub events: &'a mut Vec<StageSessionEvent>,
    pub body_contact: &'a mut lsd_core::BodyContactState<lsd_core::BodyPeer>,
}
struct LinkEffects<'a> {
    map: &'a mut MapLinkPolicy,
    palette: LinkPalette,
    commands: &'a mut Vec<MapLinkCommand>,
    sounds: &'a mut Vec<[i32; 3]>,
    error: Option<lsd_core::LinkError>,
}
impl DreamLinkEffects for LinkEffects<'_> {
    fn notify(&mut self, links: &mut DreamLinks, controls: &mut PlayerControls, event: i32) {
        match self.map.notify(links, controls, event, self.palette) {
            Ok(commands) => self.commands.extend(commands),
            Err(error) => self.error = Some(error),
        }
    }
    fn sound(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        parameters: [i32; 3],
    ) {
        self.sounds.push(parameters);
    }
    fn rotate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _absolute: bool,
        _packet: [u32; 3],
    ) {
        unreachable!("direct actor dynamic link does not rotate")
    }
    fn translate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _delta: [i32; 3],
    ) {
        unreachable!("direct actor dynamic link does not translate")
    }
    fn relocate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _spawn: SpawnLocation,
    ) {
        unreachable!("direct actor dynamic link does not relocate")
    }
}
impl DreamDayEffects for ActorJournalServices<'_> {
    fn notify(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _code: i32,
    ) {
    }
    fn attached_actor(&self) -> bool {
        self.attached
    }
    fn current_spawn(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
    ) -> SpawnLocation {
        self.current_key
    }
}
impl ActorJournalHost for ActorJournalServices<'_> {
    fn base_journal(&mut self, event: ActorJournalEvent) {
        if event.code == 4 {
            self.body_contact.current = Some(lsd_core::BodyPeer {
                type_id: event.sender_type,
                identity: event.sender_identity,
            });
        } else if matches!(event.code, 2 | 3 | 5 | 6 | 7 | 8)
            && let Some(mut source) = self.source
        {
            // Full inherited DreamSys geometry/collision queries remain pending.
            source.kind = EncounterNotificationKind::InheritedBodyQuery(event.code);
            self.pending.push(source);
        }
    }
    fn acknowledge_actor(&mut self, code: i32) -> Option<i32> {
        if let Some(mut source) = self.source {
            source.kind = EncounterNotificationKind::Acknowledge(code);
            self.pending.push(source);
        }
        None
    }
    fn capture_flashback(
        &mut self,
        day: &mut DreamDay,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        day.save_flashback(links, controls, rng, 0, 16, self)?;
        Ok(())
    }
    fn dynamic_link(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        let mut effects = LinkEffects {
            map: self.map,
            palette: self.palette,
            commands: self.commands,
            sounds: self.sounds,
            error: None,
        };
        links.force_dynamic(controls, self.catalog, self.context, rng, &mut effects)?;
        if let Some(error) = effects.error {
            return Err(error.into());
        }
        Ok(())
    }
}
pub struct ActorJournalDispatcher<'a> {
    pub day: &'a mut DreamDay,
    pub tables: &'a lsd_core::DayTables,
    pub links: &'a mut DreamLinks,
    pub controls: &'a mut PlayerControls,
    pub services: ActorJournalServices<'a>,
}
struct BoundActorJournal<'a, 'services> {
    services: &'a mut ActorJournalServices<'services>,
    actor: &'a mut lsd_core::ModelActor,
    fields: [i8; 12],
    query_resolved: bool,
}
impl ActorJournalHost for BoundActorJournal<'_, '_> {
    fn base_journal(&mut self, event: ActorJournalEvent) {
        if self.query_resolved && matches!(event.code, 2 | 3 | 5 | 6 | 7 | 8) {
            return;
        }
        self.services.base_journal(event);
    }
    fn acknowledge_actor(&mut self, code: i32) -> Option<i32> {
        let event = lsd_core::EntityBodyEvent {
            sender: lsd_core::BodyPeer {
                type_id: 0x1f34,
                identity: 0,
            },
            code,
        };
        let plan = lsd_core::entity_body_plan(self.fields, event);
        if plan.store_contact {
            self.actor.body_contact.current = Some(event.sender.into());
        }
        // Journal acknowledgements use the DreamSys 0x34 route and code four;
        // no geometry query or behavior disable occurs in this route.
        debug_assert!(!plan.query_geometry && !plan.disable_link);
        plan.notification
    }
    fn capture_flashback(
        &mut self,
        day: &mut DreamDay,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        self.services.capture_flashback(day, links, controls, rng)
    }
    fn dynamic_link(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        self.services.dynamic_link(links, controls, rng)
    }
}
impl EncounterEventHost for ActorJournalDispatcher<'_> {
    fn dream_color(&mut self) -> Result<i32, lsd_core::BehaviorFault> {
        Ok(self
            .tables
            .color(self.day.chart().map_err(lsd_core::BehaviorFault::Journal)?))
    }
    fn player_local_position(&mut self) -> Result<[i32; 3], lsd_core::BehaviorFault> {
        self.services
            .player_motion
            .as_ref()
            .map(|motion| *motion.position)
            .ok_or(lsd_core::BehaviorFault::MissingPlayerServices)
    }
    fn actor_floor_query(
        &mut self,
        position: [i32; 3],
        math: &lsd_core::PsxMathTables,
    ) -> Result<lsd_core::ActorFloorQuery, lsd_core::BehaviorFault> {
        use lsd_core::BehaviorFault;
        let scene = self
            .services
            .floor_scene
            .ok_or(BehaviorFault::MissingMotionServices)?;
        scene
            .actor_floor_query(position, math)
            .map_err(BehaviorFault::Scene)
    }
    fn move_player_local(
        &mut self,
        axis: lsd_core::ActorAxis,
        amount: i32,
    ) -> Result<(), lsd_core::BehaviorFault> {
        let motion = self
            .services
            .player_motion
            .as_mut()
            .ok_or(lsd_core::BehaviorFault::MissingPlayerServices)?;
        *motion.move_distance = amount as i16;
        let mut vector = [0; 3];
        vector[match axis {
            lsd_core::ActorAxis::X => 0,
            lsd_core::ActorAxis::Y => 1,
            lsd_core::ActorAxis::Z => 2,
        }] = i32::from(*motion.move_distance);
        let delta = motion
            .math
            .rotation(self.controls.angles)
            .apply_long(vector);
        for (position, amount) in motion.position.iter_mut().zip(delta) {
            *position = position.wrapping_add(amount);
        }
        Ok(())
    }
    fn player_action_pressed(&mut self) -> Result<bool, lsd_core::BehaviorFault> {
        Ok(self.controls.action_pressed)
    }
    fn clear_player_callbacks(&mut self, with_view: bool) -> Result<(), lsd_core::BehaviorFault> {
        if with_view && self.services.view_callback.is_none() {
            return Err(lsd_core::BehaviorFault::MissingPlayerServices);
        }
        if self.services.map.move_callback == 2 {
            self.services.map.reset_view_pending = false;
            self.services.map.view_active = false;
        }
        self.services.map.move_callback = 0;
        if with_view {
            let view = self.services.view_callback.as_mut().unwrap();
            **view = 0;
        }
        Ok(())
    }
    fn actor_destroyed(&mut self, _serial: u64, entity: u8) -> Result<(), DayFault> {
        self.services
            .events
            .push(StageSessionEvent::ActorDestroyed { entity });
        Ok(())
    }
    fn dispatch(
        &mut self,
        event: EncounterNotification,
        fields: [i8; 12],
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        self.dispatch_subject(None, event, fields, rng, false)
    }
    fn dispatch_actor(
        &mut self,
        actor: &mut lsd_core::ModelActor,
        event: EncounterNotification,
        fields: [i8; 12],
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        self.dispatch_subject(Some(actor), event, fields, rng, false)
    }
    fn dispatch_actor_cloud(
        &mut self,
        actor: &mut lsd_core::ModelActor,
        event: EncounterNotification,
        fields: [i8; 12],
        cloud: lsd_core::PreparedActorCloud,
        rng: &mut PsxRng,
    ) -> Result<(), lsd_core::BehaviorFault> {
        let EncounterNotificationKind::Notify(code @ (2 | 3 | 5 | 6 | 7 | 8)) = event.kind else {
            return self
                .dispatch_actor(actor, event, fields, rng)
                .map_err(lsd_core::BehaviorFault::Journal);
        };
        let Some(body) = &mut self.services.body_query else {
            return self
                .dispatch_actor(actor, event, fields, rng)
                .map_err(lsd_core::BehaviorFault::Journal);
        };
        body.counts.queries = body.counts.queries.wrapping_add(1);
        let contact = if let Some(bounds) = body.model.bounds {
            lsd_core::BodyGeometryQuery {
                world_delta: std::array::from_fn(|axis| {
                    cloud.world_position[axis].wrapping_sub(body.position[axis])
                }),
                inverse: body.inverse,
                cloud: cloud.cloud,
                ignore_low_midpoint: body.policy.ignore_low_midpoint,
            }
            .intersect(bounds, &body.model.faces, body.math)
            .map_err(lsd_core::BehaviorFault::Collision)?
            .is_some()
        } else {
            false
        };
        if contact {
            body.counts.contacts = body.counts.contacts.wrapping_add(1);
            // The receiver stores the sender before invoking sender.OnNotify(4).
            self.services.body_contact.current = Some(lsd_core::BodyPeer {
                type_id: 0x1f234,
                identity: event.serial,
            });
            let body_event = lsd_core::EntityBodyEvent {
                sender: lsd_core::BodyPeer {
                    type_id: 0x1f34,
                    identity: 0,
                },
                code: 4,
            };
            let plan = lsd_core::entity_body_plan(fields, body_event);
            if plan.store_contact {
                actor.body_contact.current = Some(body_event.sender.into());
            }
            if let Some(notification) = plan.notification {
                self.dispatch_subject(
                    Some(actor),
                    EncounterNotification {
                        kind: EncounterNotificationKind::Notify(notification),
                        ..event
                    },
                    fields,
                    rng,
                    false,
                )
                .map_err(lsd_core::BehaviorFault::Journal)?;
            }
        }
        self.services
            .events
            .push(StageSessionEvent::ActorBodyQuery {
                entity: event.entity,
                code,
                contact,
            });
        self.dispatch_subject(Some(actor), event, fields, rng, true)
            .map_err(lsd_core::BehaviorFault::Journal)
    }
}
impl ActorJournalDispatcher<'_> {
    fn dispatch_subject(
        &mut self,
        actor: Option<&mut lsd_core::ModelActor>,
        event: EncounterNotification,
        fields: [i8; 12],
        rng: &mut PsxRng,
        query_resolved: bool,
    ) -> Result<(), DayFault> {
        if event.kind == EncounterNotificationKind::ResetFlashbackCount {
            self.day.flashback_count = 0;
            self.services
                .events
                .push(StageSessionEvent::ActorFlashbackReset {
                    entity: event.entity,
                });
            return Ok(());
        }
        let EncounterNotificationKind::Notify(code) = event.kind else {
            self.services.pending.push(event);
            return Ok(());
        };
        self.services.source = Some(event);
        self.services.palette.dream = self.tables.color(self.day.chart()?);
        let applied = self.links.execution.state == 0
            && (matches!(code, 4 | 10)
                || (!self.links.execution.flashback && matches!(code, 9 | 11 | 12)));
        let journal_event = ActorJournalEvent {
            sender_type: 0x1f234,
            sender_identity: event.serial,
            code,
            fields,
        };
        if let Some(actor) = actor {
            let mut host = BoundActorJournal {
                services: &mut self.services,
                actor,
                fields,
                query_resolved,
            };
            actor_journal(
                journal_event,
                self.day,
                self.links,
                self.controls,
                rng,
                &mut host,
            )?;
        } else {
            actor_journal(
                journal_event,
                self.day,
                self.links,
                self.controls,
                rng,
                &mut self.services,
            )?;
        }
        self.services.events.push(StageSessionEvent::ActorJournal {
            entity: event.entity,
            code,
            applied,
        });
        Ok(())
    }
}
