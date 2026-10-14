use crate::{PsxExeHeader, StreamedStage, formats::Mom, is_canonical_reference};
use lsd_core::{
    EncounterFrame, EncounterHost, EncounterSpawn, EncounterTables, ModelActor, PsxRng,
    transform_number,
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
    pub properties: EntityProperties,
    pub behavior_active: bool,
    pub behavior_pending: bool,
}
#[derive(Debug, Clone)]
pub struct EncounterScene {
    pub tables: EncounterTables,
    pub properties: Vec<EntityProperties>,
    pub actors: Vec<SceneActor>,
    pub gray_man_activation_requests: u64,
    pub teleports_requested: bool,
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
            seen: [None; 7],
            epoch: u64::MAX,
            serial: 0,
        }
    }
    pub fn stop(&mut self) {
        self.actors.clear();
        self.seen = [None; 7];
        self.teleports_requested = false;
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
        if self.epoch != epoch {
            self.stop();
            self.epoch = epoch;
        }
        for slot in 0..7 {
            let ready = stage.collision.slots.slots[slot].ready;
            let map = stage.protocol.slots[slot].map_index;
            let identity = ready.then_some((stage.stage_id(), map));
            if self.seen[slot] != identity {
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
    /// Common animation playback is connected only for callback-free active
    /// actors. Named NPC callbacks retain an explicit pending state.
    pub fn tick_animations(&mut self) -> Result<(), lsd_core::AnimationError> {
        for actor in &mut self.actors {
            if !actor.behavior_pending {
                actor.actor.tick_animation(&actor.resource.animations)?;
            }
        }
        Ok(())
    }
}
struct Host<'a> {
    stage: &'a StreamedStage,
    slot: usize,
    map: u16,
    properties: &'a [EntityProperties],
    actors: &'a mut Vec<SceneActor>,
    gray_requests: &'a mut u64,
    teleports: &'a mut bool,
    serial: &'a mut u64,
    selected: Option<Arc<Mom>>,
}
impl EncounterHost for Host<'_> {
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
        actor.action_kind = 66;
        // entity_init's positive unlock gate clears bit six on every part.
        if (properties.fields[2] as u8).wrapping_sub(1) < 9 {
            for part in &mut actor.animation.parts {
                part.attributes &= !(1 << 6);
            }
        }
        let active = properties.fields[3] == 0 && properties.fields[11] == 0;
        actor.animation.enabled = active;
        actor.animation.action_enabled = active;
        // Callback dispatch/interaction remains pending for named NPC behavior.
        let pending = properties.callback.is_some();
        *self.serial = self.serial.wrapping_add(1);
        self.actors.push(SceneActor {
            serial: *self.serial,
            slot: self.slot,
            map: self.map,
            entity: spawn.entity,
            model: spawn.model,
            resource: Arc::clone(resource),
            actor,
            properties,
            behavior_active: active,
            behavior_pending: pending,
        });
        true
    }
    fn activate_gray_man(&mut self) {
        *self.gray_requests = self.gray_requests.wrapping_add(1);
    }
}
