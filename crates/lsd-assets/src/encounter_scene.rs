use crate::{PsxExeHeader, StreamedStage, formats::Mom, is_canonical_reference};
use lsd_core::{
    BehaviorTick, EncounterFrame, EncounterHost, EncounterSpawn, EncounterTables, EntityBehavior,
    EntityInteractionHost, EntityScriptHost, EntityTarget, GenerationSound, ModelActor, NpcScript,
    PsxRng, transform_number,
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityProperties {
    pub fields: [i8; 12],
    pub callback: Option<u32>,
}
pub fn load_entity_properties(executable: &[u8]) -> Result<Vec<EntityProperties>, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Entity properties require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let offset = (0x8008_9ea4 - header.load_address) as usize + 0x800;
    let bytes = executable
        .get(offset..offset + 130 * 16)
        .ok_or("Entity properties outside executable")?;
    Ok(bytes
        .chunks_exact(16)
        .map(|v| EntityProperties {
            fields: std::array::from_fn(|i| v[i] as i8),
            callback: match u32::from_le_bytes(v[12..].try_into().unwrap()) {
                0 => None,
                p => Some(p),
            },
        })
        .collect())
}
#[derive(Debug, Clone)]
pub struct SceneActor {
    pub serial: u64,
    pub slot: usize,
    pub map: u16,
    pub entity: u8,
    pub model: i8,
    pub resource: Arc<Mom>,
    pub actor: ModelActor,
    pub collision: Option<lsd_core::CollisionModel>,
    pub properties: EntityProperties,
    pub behavior_active: bool,
    pub behavior_pending: bool,
    pub behavior: EntityBehavior,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterNotificationKind {
    Notify(i32),
    AimAtPlayer,
    Acknowledge(i32),
    InheritedBodyQuery(i32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterNotification {
    pub serial: u64,
    pub entity: u8,
    pub kind: EncounterNotificationKind,
}
pub trait EncounterEventHost {
    fn dispatch(
        &mut self,
        event: EncounterNotification,
        fields: [i8; 12],
        rng: &mut PsxRng,
    ) -> Result<(), lsd_core::DayFault>;
    fn dispatch_actor(
        &mut self,
        _actor: &mut ModelActor,
        event: EncounterNotification,
        fields: [i8; 12],
        rng: &mut PsxRng,
    ) -> Result<(), lsd_core::DayFault> {
        self.dispatch(event, fields, rng)
    }
    /// The cloud exists only during the original synchronous parent notification.
    fn dispatch_actor_cloud(
        &mut self,
        actor: &mut ModelActor,
        event: EncounterNotification,
        fields: [i8; 12],
        _cloud: lsd_core::PreparedActorCloud,
        rng: &mut PsxRng,
    ) -> Result<(), lsd_core::BehaviorFault> {
        self.dispatch_actor(actor, event, fields, rng)
            .map_err(lsd_core::BehaviorFault::Journal)
    }
}
struct DeferredEvents<'a>(&'a mut Vec<EncounterNotification>);
impl EncounterEventHost for DeferredEvents<'_> {
    fn dispatch(
        &mut self,
        event: EncounterNotification,
        _fields: [i8; 12],
        _rng: &mut PsxRng,
    ) -> Result<(), lsd_core::DayFault> {
        self.0.push(event);
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterUpdate {
    pub epoch: u64,
    pub day: i32,
    pub palette: i32,
    pub generation_type: i32,
}
struct UnavailableAudio;
impl GenerationSound for UnavailableAudio {
    fn stop_note(&mut self, _handle: i32) -> i32 {
        -1
    }
    fn volume_offset(&mut self, _offset: i32) {}
    fn play_note(&mut self, _note: i32, _period: i32, _counter: i32) -> i32 {
        -1
    }
}
#[derive(Debug, Clone)]
pub struct EncounterScene {
    pub tables: EncounterTables,
    pub properties: Vec<EntityProperties>,
    pub actors: Vec<SceneActor>,
    pub gray_man_activation_requests: u64,
    pub teleports_requested: bool,
    pub pending_notifications: Vec<EncounterNotification>,
    seen: [Option<(u8, u16)>; 7],
    epoch: u64,
    serial: u64,
}
impl EncounterScene {
    pub fn new(mut tables: EncounterTables, properties: Vec<EntityProperties>) -> Self {
        // link_init_stage_objects resets all used bytes once at new-game setup.
        for stage in &mut tables.objects {
            for object in stage {
                object.used = 0;
            }
        }
        Self {
            tables,
            properties,
            actors: Vec::new(),
            gray_man_activation_requests: 0,
            teleports_requested: false,
            pending_notifications: Vec::new(),
            seen: [None; 7],
            epoch: u64::MAX,
            serial: 0,
        }
    }
    pub fn stop(&mut self) {
        self.stop_with_audio(&mut UnavailableAudio);
    }
    pub fn stop_with_audio(&mut self, sound: &mut impl GenerationSound) {
        for actor in &mut self.actors {
            actor.behavior.disable_link(&mut actor.actor, sound);
        }
        self.actors.clear();
        self.seen = [None; 7];
        self.teleports_requested = false;
        self.pending_notifications.clear();
    }
    pub fn update(
        &mut self,
        stage: &StreamedStage,
        epoch: u64,
        day: i32,
        palette: i32,
        generation_type: i32,
        rng: &mut PsxRng,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.update_with_audio(
            stage,
            EncounterUpdate {
                epoch,
                day,
                palette,
                generation_type,
            },
            rng,
            &mut UnavailableAudio,
        )
    }
    pub fn update_with_audio(
        &mut self,
        stage: &StreamedStage,
        input: EncounterUpdate,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let EncounterUpdate {
            epoch,
            day,
            palette,
            generation_type,
        } = input;
        if self.epoch != epoch {
            self.stop_with_audio(sound);
            self.epoch = epoch;
        }
        for slot in 0..7 {
            let ready = stage.collision.slots.slots[slot].ready;
            let map = stage.protocol.slots[slot].map_index;
            let identity = ready.then_some((stage.stage_id(), map));
            if self.seen[slot] != identity {
                for actor in self.actors.iter_mut().filter(|actor| actor.slot == slot) {
                    actor.behavior.disable_link(&mut actor.actor, sound);
                }
                self.actors.retain(|actor| actor.slot != slot);
                self.seen[slot] = identity;
                if ready {
                    let grid = stage.collision.slots.grid;
                    // Encounter keys use the catalogue's grid coordinates.
                    // Vertical maps still keep their row byte; their placement
                    // routine separately ignores it in the horizontal plane.
                    let chunk = i16::from_le_bytes([
                        (i32::from(map) % i32::from(grid.columns)) as u8,
                        (i32::from(map) / i32::from(grid.columns)) as u8,
                    ]);
                    let mut host = Host {
                        stage,
                        slot,
                        map,
                        properties: &self.properties,
                        actors: &mut self.actors,
                        gray_requests: &mut self.gray_man_activation_requests,
                        teleports: &mut self.teleports_requested,
                        serial: &mut self.serial,
                        selected: None,
                        sound,
                    };
                    self.tables.spawn_chunk(
                        EncounterFrame {
                            stage: i32::from(stage.stage_id()),
                            day,
                            palette,
                            generation_type,
                            chunk,
                        },
                        rng,
                        &mut host,
                    )?;
                }
            }
        }
        Ok(())
    }
    /// Native query math and policy precede action callbacks and animation.
    /// Parent/camera construction, body/mood dispatch and global cadence remain open.
    pub fn tick_behaviors(
        &mut self,
        target: EntityTarget,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
    ) -> Result<(), lsd_core::BehaviorFault> {
        let mut pending = std::mem::take(&mut self.pending_notifications);
        let result = self.tick_with_events(target, rng, sound, &mut DeferredEvents(&mut pending));
        self.pending_notifications = pending;
        result
    }
    pub fn tick_with_events(
        &mut self,
        target: EntityTarget,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
        events: &mut impl EncounterEventHost,
    ) -> Result<(), lsd_core::BehaviorFault> {
        self.tick_script_services(target, rng, sound, None, events)
    }
    pub fn tick_with_motion(
        &mut self,
        target: EntityTarget,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
        math: &lsd_core::PsxMathTables,
        events: &mut impl EncounterEventHost,
    ) -> Result<(), lsd_core::BehaviorFault> {
        self.tick_script_services(target, rng, sound, Some(math), events)
    }
    fn tick_script_services(
        &mut self,
        target: EntityTarget,
        rng: &mut PsxRng,
        sound: &mut impl GenerationSound,
        math: Option<&lsd_core::PsxMathTables>,
        events: &mut impl EncounterEventHost,
    ) -> Result<(), lsd_core::BehaviorFault> {
        for actor in &mut self.actors {
            let mut host = InteractionHost {
                target,
                serial: actor.serial,
                entity: actor.entity,
                fields: actor.properties.fields,
                events,
                error: None,
                math,
                collision: actor.collision.as_ref(),
                // This adapter supplies an identity parent frame at update entry.
                // Hold it through local movement; native notifications read cached world state.
                body_world: math.map(|math| lsd_core::BodyWorldFrame {
                    matrix: math
                        .rotation(actor.actor.pose.angles)
                        .scaled(actor.actor.pose.scale),
                    position: actor.actor.pose.position,
                }),
            };
            actor.behavior.update_interaction(
                &mut actor.actor,
                actor.properties.fields,
                rng,
                sound,
                &mut host,
            );
            if let Some(error) = host.error {
                return Err(lsd_core::BehaviorFault::Journal(error));
            }
            actor.behavior_active = actor.behavior.active != 0;
            if !actor.behavior_pending {
                actor.behavior.tick_with_services(
                    &mut actor.actor,
                    &actor.resource.animations,
                    BehaviorTick {
                        target: target.position,
                        proximity: actor.properties.fields[10],
                    },
                    rng,
                    sound,
                    &mut host,
                )?;
            } else {
                // The interaction timer is independent of a pending named action.
                actor.actor.animation.ticks = actor.actor.animation.ticks.wrapping_add(1);
            }
        }
        Ok(())
    }
}
struct InteractionHost<'a, H> {
    target: EntityTarget,
    serial: u64,
    entity: u8,
    fields: [i8; 12],
    events: &'a mut H,
    error: Option<lsd_core::DayFault>,
    math: Option<&'a lsd_core::PsxMathTables>,
    collision: Option<&'a lsd_core::CollisionModel>,
    body_world: Option<lsd_core::BodyWorldFrame>,
}
impl<H: EncounterEventHost> EntityScriptHost for InteractionHost<'_, H> {
    fn movement_matrix(
        &mut self,
        angles: [i16; 3],
    ) -> Result<lsd_core::FixedMatrix, lsd_core::BehaviorFault> {
        self.math
            .map(|math| math.rotation(angles))
            .ok_or(lsd_core::BehaviorFault::MissingMotionServices)
    }
    fn notify(&mut self, rng: &mut PsxRng, code: i32) -> Result<(), lsd_core::BehaviorFault> {
        self.events
            .dispatch(
                EncounterNotification {
                    serial: self.serial,
                    entity: self.entity,
                    kind: EncounterNotificationKind::Notify(code),
                },
                self.fields,
                rng,
            )
            .map_err(lsd_core::BehaviorFault::Journal)
    }
    fn notify_actor(
        &mut self,
        actor: &mut ModelActor,
        rng: &mut PsxRng,
        code: i32,
    ) -> Result<(), lsd_core::BehaviorFault> {
        if matches!(code, 2 | 3 | 5 | 6 | 7 | 8)
            && let (Some(model), Some(world)) = (self.collision, self.body_world)
            && let Some(cloud) =
                model.prepared_actor_cloud(world, code, actor.move_distance, actor.collision_margin)
        {
            actor.body_contact.current = None;
            return self.events.dispatch_actor_cloud(
                actor,
                EncounterNotification {
                    serial: self.serial,
                    entity: self.entity,
                    kind: EncounterNotificationKind::Notify(code),
                },
                self.fields,
                cloud,
                rng,
            );
        }
        self.events
            .dispatch_actor(
                actor,
                EncounterNotification {
                    serial: self.serial,
                    entity: self.entity,
                    kind: EncounterNotificationKind::Notify(code),
                },
                self.fields,
                rng,
            )
            .map_err(lsd_core::BehaviorFault::Journal)
    }
}
impl<H: EncounterEventHost> EntityInteractionHost for InteractionHost<'_, H> {
    fn proximity(&mut self, position: [i32; 3], forward: i32, radius: i32) -> bool {
        self.target
            .proximity(position, forward, radius)
            .expect("validated imported view divisor")
    }
    fn facing_target(&mut self, position: [i32; 3], distance: i32) -> bool {
        self.target.facing(position, distance)
    }
    fn aim_at_target(&mut self, rng: &mut PsxRng) {
        let event = EncounterNotification {
            serial: self.serial,
            entity: self.entity,
            kind: EncounterNotificationKind::AimAtPlayer,
        };
        if self.error.is_none() {
            self.error = self.events.dispatch(event, self.fields, rng).err();
        }
    }
    fn notify(&mut self, rng: &mut PsxRng, code: i32) {
        let event = EncounterNotification {
            serial: self.serial,
            entity: self.entity,
            kind: EncounterNotificationKind::Notify(code),
        };
        if self.error.is_none() {
            self.error = self.events.dispatch(event, self.fields, rng).err();
        }
    }
    fn notify_actor(&mut self, actor: &mut ModelActor, rng: &mut PsxRng, code: i32) {
        if self.error.is_none() {
            self.error = self
                .events
                .dispatch_actor(
                    actor,
                    EncounterNotification {
                        serial: self.serial,
                        entity: self.entity,
                        kind: EncounterNotificationKind::Notify(code),
                    },
                    self.fields,
                    rng,
                )
                .err();
        }
    }
}
struct Host<'a, S> {
    stage: &'a StreamedStage,
    slot: usize,
    map: u16,
    properties: &'a [EntityProperties],
    actors: &'a mut Vec<SceneActor>,
    gray_requests: &'a mut u64,
    teleports: &'a mut bool,
    serial: &'a mut u64,
    selected: Option<Arc<Mom>>,
    sound: &'a mut S,
}
impl<S: GenerationSound> EncounterHost for Host<'_, S> {
    fn prepare_library(&mut self) -> bool {
        self.stage.maps[self.slot]
            .as_ref()
            .and_then(|v| v.model_library.as_ref())
            .is_some()
    }
    fn enable_teleports(&mut self) {
        *self.teleports = true;
    }
    fn select_model(&mut self, index: i8) -> bool {
        self.selected = usize::try_from(index)
            .ok()
            .and_then(|index| {
                self.stage.maps[self.slot]
                    .as_ref()?
                    .model_library
                    .as_ref()?
                    .models
                    .get(index)
            })
            .map(|v| Arc::new(v.clone()));
        self.selected.is_some()
    }
    fn create_actor(&mut self, spawn: EncounterSpawn) -> bool {
        let Some(resource) = &self.selected else {
            return false;
        };
        let Some(properties) = self.properties.get(usize::from(spawn.entity)).copied() else {
            return false;
        };
        let Ok(mut actor) = resource.construct_actor() else {
            return false;
        };
        actor.pose.position = self
            .stage
            .collision
            .slots
            .grid
            .place_spawn([0; 3], spawn.spawn)
            .position;
        for (angle, word) in actor.pose.angles.iter_mut().zip(spawn.rotation) {
            let Some(value) = transform_number(word) else {
                return false;
            };
            *angle = (value / 360) as i16;
        }
        // The original entity_start applies its supplied origin and invalidates
        // the coordinate cache after construction.
        actor.translate(actor.pose.position, true);
        let script = NpcScript::from_reference_address(properties.callback);
        let behavior = EntityBehavior::construct(
            spawn.entity,
            script,
            properties.fields,
            &mut actor,
            self.sound,
        );
        let active = behavior.active != 0;
        let pending = script.pending();
        let Ok(collision) = resource.collision_model(&actor) else {
            return false;
        };
        *self.serial = self.serial.wrapping_add(1);
        self.actors.push(SceneActor {
            serial: *self.serial,
            slot: self.slot,
            map: self.map,
            entity: spawn.entity,
            model: spawn.model,
            resource: Arc::clone(resource),
            actor,
            collision,
            properties,
            behavior_active: active,
            behavior_pending: pending,
            behavior,
        });
        true
    }
    fn activate_gray_man(&mut self) {
        *self.gray_requests = self.gray_requests.wrapping_add(1);
    }
}
