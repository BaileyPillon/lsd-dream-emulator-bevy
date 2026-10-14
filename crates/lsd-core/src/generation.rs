use crate::{
    GenerationEffectTables, GenerationLayers, GenerationObjectTables, GenerationStructure, PsxRng,
    SceneGrid, SpawnLocation, VramMove, generation_border_moves,
};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationFrame {
    pub position: [i32; 3],
    /// Original renderer eye Y and target Y, before presentation conversion.
    pub view: [i32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationFault {
    InvalidStage,
    InvalidEntry,
    InvalidDivision,
    InvalidEffect,
    InvalidContext,
}
impl fmt::Display for GenerationFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported generation input: {self:?}")
    }
}
impl std::error::Error for GenerationFault {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationEntry {
    pub kind: i8,
    pub height: i8,
    pub effect_color: i8,
    pub scene_color: i8,
}
impl From<[i8; 4]> for GenerationEntry {
    fn from(value: [i8; 4]) -> Self {
        Self {
            kind: value[0],
            height: value[1],
            effect_color: value[2],
            scene_color: value[3],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationActorEntry {
    pub key: [i8; 4],
    pub style: u8,
    pub effect: i8,
    pub reserved: [u8; 2],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationTables {
    pub entries: [Vec<GenerationEntry>; 4],
    pub cycle: [i8; 16],
    pub fixed: [Option<GenerationEntry>; 14],
    pub palette: Vec<[u8; 3]>,
    pub heights: [i32; 7],
    pub sample_offsets: [i32; 5],
    pub rotations: [[u32; 3]; 7],
    pub scales: [[u32; 3]; 5],
    pub fixed_scale: [u32; 3],
    pub colors_a: [[u8; 3]; 3],
    pub colors_b: [[u8; 3]; 3],
    pub effect_palettes: [[[u8; 3]; 18]; 2],
    pub entity_ranges: [i32; 15],
    pub actor_entries: [Vec<GenerationActorEntry>; 14],
    pub styles: Vec<[i16; 3]>,
    pub alternate_parameter: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationStyle {
    pub kind: i32,
    pub height: i32,
    pub scene_color: [u8; 3],
    pub environment_color: [u8; 3],
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GenerationSpawn {
    pub position: [i32; 3],
    pub rotation: [u32; 3],
    pub scale: [u32; 3],
    pub index: i32,
    pub random: i32,
    pub color: [u8; 3],
    pub alternate_color: Option<[u8; 3]>,
}
#[derive(Debug, Clone)]
pub struct DreamGeneration {
    pub owner_active: bool,
    pub location: i32,
    pub day: i32,
    pub submode: i32,
    pub timer: i32,
    pub generation_type: i32,
    pub index: i32,
    pub flags: i32,
    pub effect_palette: usize,
    pub effect_data_color: [u8; 3],
    pub environment_effect: Option<[u8; 3]>,
    pub style: Option<GenerationStyle>,
    pub spawn: GenerationSpawn,
    pub entry_index: i32,
    pub actor_entries: Vec<GenerationActorEntry>,
    pub actors: [GeneratedActor; 2],
    pub structures: Vec<GenerationStructure>,
    pub structure_count: i32,
    pub structure_view_origin: i32,
    pub layers: GenerationLayers,
}
impl Default for DreamGeneration {
    fn default() -> Self {
        Self {
            owner_active: false,
            location: 0,
            day: 0,
            submode: 0,
            timer: 0,
            generation_type: -1,
            index: 0,
            flags: 0,
            effect_palette: 0,
            effect_data_color: [0; 3],
            environment_effect: None,
            style: None,
            spawn: GenerationSpawn::default(),
            entry_index: 0,
            actor_entries: Vec::new(),
            actors: [GeneratedActor::default(); 2],
            structures: Vec::new(),
            structure_count: 0,
            structure_view_origin: 0,
            layers: GenerationLayers::default(),
        }
    }
}
impl DreamGeneration {
    pub fn start(
        &mut self,
        tables: &GenerationTables,
        location: i32,
        day: i32,
        submode: i32,
    ) -> Result<bool, GenerationFault> {
        if self.owner_active {
            return Ok(false);
        }
        let location_index = usize::try_from(location)
            .ok()
            .filter(|&i| i < 14)
            .ok_or(GenerationFault::InvalidStage)?;
        self.owner_active = true;
        self.location = location;
        self.day = day;
        self.submode = submode;
        self.generation_type = -1;
        self.timer = 0;
        self.layers.stage_color = None;
        for actor in &mut self.actors {
            actor.active = false;
        }
        self.actor_entries = tables.actor_entries[location_index].clone();
        let entry = if let Some(entry) = tables.fixed[location_index] {
            entry
        } else {
            let selector = day.wrapping_add(location);
            let kind = tables.cycle[(selector & 15) as usize];
            let table = usize::try_from(kind)
                .ok()
                .and_then(|i| tables.entries.get(i))
                .ok_or(GenerationFault::InvalidEntry)?;
            if table.is_empty() {
                return Err(GenerationFault::InvalidDivision);
            }
            let index = selector % table.len() as i32;
            let entry = *usize::try_from(index)
                .ok()
                .and_then(|i| table.get(i))
                .ok_or(GenerationFault::InvalidEntry)?;
            self.generation_type = i32::from(kind);
            self.index = index;
            if kind == 0 {
                self.effect_palette = usize::from(entry.effect_color == 18);
                self.effect_data_color = *usize::try_from(entry.scene_color)
                    .ok()
                    .and_then(|i| tables.palette.get(i))
                    .ok_or(GenerationFault::InvalidEntry)?;
                if index < 4 {
                    self.flags = 1;
                } else if index < 6 {
                    self.flags = 2;
                }
            }
            entry
        };
        let color = |index: i8| {
            usize::try_from(index)
                .ok()
                .and_then(|i| tables.palette.get(i))
                .copied()
                .ok_or(GenerationFault::InvalidEntry)
        };
        self.style = Some(GenerationStyle {
            kind: i32::from(entry.kind),
            height: *usize::try_from(entry.height)
                .ok()
                .and_then(|i| tables.heights.get(i))
                .ok_or(GenerationFault::InvalidEntry)?,
            scene_color: color(entry.scene_color)?,
            environment_color: color(entry.effect_color)?,
        });
        if entry.height >= 4 {
            self.environment_effect = Some(color(entry.effect_color)?);
        }
        Ok(true)
    }
    pub fn seed_random(&mut self, tables: &GenerationTables, rng: &mut PsxRng, mut sample: i32) {
        if sample == 0 {
            sample = tables.sample_offsets[(rng.next_u15() & 3) as usize];
        }
        self.spawn.position[1] = sample;
        let x = i32::from(rng.next_u15() % 23) << 11;
        self.spawn.position[0] = if rng.next_u15() & 1 != 0 { -x } else { x };
        let z = i32::from(rng.next_u15() % 23) << 11;
        self.spawn.position[2] = if rng.next_u15() & 1 != 0 { -z } else { z };
        self.spawn.rotation = tables.rotations[(rng.next_u15() % 7) as usize];
        self.spawn.index = i32::from(rng.next_u15() % 5);
    }
    pub fn seed_daily(&mut self, tables: &GenerationTables, rng: &mut PsxRng) {
        rng.next_u15();
        self.spawn.position[1] = tables.sample_offsets[1];
        self.spawn.position[0] = i32::from(rng.next_u15() % 20) << 11;
        self.spawn.position[2] = 0xa000;
        // The original field named SpawnRoll is position z, not the rotation packet pointer.
        match self.day % 3 {
            1 => self.spawn.position[2] = -0xa000,
            2 => self.spawn.position[2] = 0x800,
            _ => {}
        }
        self.spawn.rotation = tables.rotations[(rng.next_u15() % 7) as usize];
        self.spawn.index = i32::from(rng.next_u15() % 5);
    }

    pub fn build_structures(
        &mut self,
        tables: &GenerationTables,
        objects: &GenerationObjectTables,
        rng: &mut PsxRng,
        base: [i32; 3],
        view_height: i32,
    ) -> Result<(), GenerationFault> {
        if self.generation_type < 0 {
            return Ok(());
        }
        let count = i32::from(tables.heights[6].to_le_bytes()[(rng.next_u15() & 3) as usize] as i8);
        let extra = if self.generation_type == 2 {
            16_i32.wrapping_sub(count)
        } else {
            0
        };
        self.structure_count = count.wrapping_add(extra);
        self.structures.clear();
        self.spawn.random = i32::from(rng.next_u15() % 7);
        self.spawn.scale = tables.scales[(rng.next_u15() % 5) as usize];
        let sample_index = usize::from(rng.next_u15() % 5);
        let sample = if sample_index == 0 {
            0
        } else {
            tables.sample_offsets[sample_index]
        };
        for _ in 0..count.max(0) {
            if self.day % 7 == 0 {
                self.seed_daily(tables, rng);
            } else {
                self.seed_random(tables, rng, sample);
            }
            self.structures.push(GenerationStructure::construct(
                0, self.spawn, objects, base, rng,
            )?);
        }
        // This assignment occurs even when no extra structures are created.
        self.spawn.scale = tables.fixed_scale;
        for _ in 0..extra.max(0) {
            self.seed_random(tables, rng, tables.sample_offsets[2]);
            self.structures.push(GenerationStructure::construct(
                1, self.spawn, objects, base, rng,
            )?);
        }
        match self.generation_type {
            0 => {
                self.seed_random(tables, rng, tables.sample_offsets[2]);
                if self.flags != 0 && self.effect_palette == 1 {
                    self.spawn.position = [-0xb000, -0x2000, 0];
                    self.spawn.color = tables.colors_a[1];
                } else {
                    if self.spawn.position[2] > 0 {
                        self.spawn.position[2] = self.spawn.position[2].wrapping_neg();
                    }
                    self.spawn.position[2] = self.spawn.position[2].max(-0x7800);
                    self.spawn.color = tables.colors_a[(rng.next_u15() % 3) as usize];
                }
                self.spawn.rotation = tables.rotations[0];
                self.structures.push(GenerationStructure::construct(
                    3, self.spawn, objects, base, rng,
                )?);
                self.structure_count = self.structure_count.wrapping_add(1);
            }
            2 => {
                self.spawn.color = tables.colors_b[(rng.next_u15() % 3) as usize];
                // apply_entry aliases GenerationInfo + 0xc with the first word
                // of EntityParamsAlt. It contains the current scene-color pointer.
                self.spawn.alternate_color = if self.day % 20 == 0 {
                    None
                } else {
                    Some(
                        self.style
                            .ok_or(GenerationFault::InvalidContext)?
                            .scene_color,
                    )
                };
                self.seed_random(tables, rng, tables.sample_offsets[2]);
                self.spawn.rotation = tables.rotations[0];
                self.spawn.random = i32::from(rng.next_u15() % 6);
                self.structures.push(GenerationStructure::construct(
                    2, self.spawn, objects, base, rng,
                )?);
                self.structure_count = self.structure_count.wrapping_add(1);
            }
            _ => {}
        }
        if !self.structures.is_empty() {
            self.structure_view_origin = view_height;
        }
        Ok(())
    }

    /// The first update constructs objects and then updates them immediately.
    pub fn tick_structures(
        &mut self,
        tables: &GenerationTables,
        objects: &GenerationObjectTables,
        rng: &mut PsxRng,
        base: [i32; 3],
        view_height: i32,
    ) -> Result<(), GenerationFault> {
        let previous = self.timer;
        self.timer = self.timer.wrapping_add(1);
        if previous == 0 {
            self.build_structures(tables, objects, rng, base, view_height)?;
        }
        self.update_structures(objects, rng, base, view_height)
    }

    /// First-frame layers are allocated before structures. Every frame then
    /// updates the gradient, structures and palette copies, in native order.
    /// Proximity sound slots follow this call, using the same RNG and timer.
    pub fn tick_objects_and_layers(
        &mut self,
        tables: &GenerationTables,
        objects: &GenerationObjectTables,
        geometry: &GenerationEffectTables,
        rng: &mut PsxRng,
        frame: GenerationFrame,
    ) -> Result<Vec<VramMove>, GenerationFault> {
        let previous = self.timer;
        self.timer = self.timer.wrapping_add(1);
        if previous == 0 {
            self.layers.create(
                tables,
                geometry,
                self.environment_effect,
                self.flags,
                self.effect_palette,
            );
            self.build_structures(tables, objects, rng, frame.position, frame.view[0])?;
        }
        self.layers.update(
            tables,
            geometry,
            self.flags,
            self.effect_palette,
            self.effect_data_color,
            frame.view,
        );
        self.update_structures(objects, rng, frame.position, frame.view[0])?;
        Ok(generation_border_moves(self.location, geometry))
    }

    fn update_structures(
        &mut self,
        objects: &GenerationObjectTables,
        rng: &mut PsxRng,
        base: [i32; 3],
        view_height: i32,
    ) -> Result<(), GenerationFault> {
        let delta = view_height.wrapping_sub(self.structure_view_origin);
        for object in &mut self.structures {
            object.update(objects, base, delta, rng)?;
        }
        Ok(())
    }
    pub fn find_actor(
        &mut self,
        tables: &GenerationTables,
        grid: SceneGrid,
        base: [i32; 3],
        position: Option<[i32; 3]>,
        output: &mut [i32; 3],
        distance: &mut i32,
    ) -> Result<Option<usize>, GenerationFault> {
        let Some(position) = position else {
            return Ok(None);
        };
        let start = usize::try_from(self.entry_index).map_err(|_| GenerationFault::InvalidEntry)?;
        for index in start..self.actor_entries.len() {
            self.entry_index = self.entry_index.wrapping_add(1);
            let entry = self.actor_entries[index];
            if entry.effect <= 0 {
                continue;
            }
            let offset = *tables
                .styles
                .get(usize::from(entry.style))
                .ok_or(GenerationFault::InvalidEntry)?;
            let key = SpawnLocation {
                chunk: i16::from_le_bytes([entry.key[0] as u8, entry.key[1] as u8]),
                tile: i16::from_le_bytes([entry.key[2] as u8, entry.key[3] as u8]),
                position: offset,
            };
            *output = grid.place_spawn(base, key).position;
            let dx = output[0].wrapping_sub(position[0]);
            let dz = output[2].wrapping_sub(position[2]);
            *distance = dx.wrapping_abs().wrapping_add(dz.wrapping_abs());
            let range = *tables
                .entity_ranges
                .get(entry.effect as usize)
                .ok_or(GenerationFault::InvalidEffect)?;
            if *distance < range {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }
    pub fn stop(&mut self) {
        self.stop_with_sound(&mut StoppedSound);
    }

    pub fn stop_with_sound(&mut self, sound: &mut impl GenerationSound) {
        self.environment_effect = None;
        self.flags = 0;
        self.layers.stop();
        if self.generation_type >= 0 {
            self.structures.clear();
        }
        for actor in &mut self.actors {
            if actor.active {
                actor.stop(sound);
                if let Some(entry) = self.actor_entries.get_mut(actor.entry) {
                    entry.effect = entry.effect.wrapping_neg();
                }
            }
        }
        self.owner_active = false;
    }

    /// The two original generation slots are visited in order. Placement and
    /// sound-context storage survive deactivation; a dropped slot can only be
    /// selected again on a subsequent visit, as in the original update loop.
    pub fn tick_actors(
        &mut self,
        tables: &GenerationTables,
        grid: SceneGrid,
        base: [i32; 3],
        position: Option<[i32; 3]>,
        mut wanted: i32,
        sound: &mut impl GenerationSound,
    ) -> Result<i32, GenerationFault> {
        self.entry_index = 0;
        for slot in 0..2 {
            let mut actor = self.actors[slot];
            if actor.active {
                let effect = self
                    .actor_entries
                    .get(actor.entry)
                    .ok_or(GenerationFault::InvalidEntry)?
                    .effect
                    .wrapping_neg();
                let range = *usize::try_from(effect)
                    .ok()
                    .and_then(|i| tables.entity_ranges.get(i))
                    .ok_or(GenerationFault::InvalidEffect)?;
                let in_range = if let Some(position) = position {
                    actor.distance = actor.position[0]
                        .wrapping_sub(position[0])
                        .wrapping_abs()
                        .wrapping_add(actor.position[2].wrapping_sub(position[2]).wrapping_abs());
                    actor.distance < range
                } else {
                    false
                };
                if in_range {
                    actor.update_sound(effect, range, sound)?;
                } else {
                    actor.stop(sound);
                    self.actor_entries[actor.entry].effect = effect;
                }
            } else if let Some(entry) = self.find_actor(
                tables,
                grid,
                base,
                position,
                &mut actor.position,
                &mut actor.distance,
            )? {
                let effect = self.actor_entries[entry].effect;
                actor.entry = entry;
                actor.active = true;
                // Sound initialization retains the other context words and is
                // rejected when its state is already nonzero.
                if actor.context[0] == 0 {
                    actor.context[0] = i32::from(effect);
                    actor.context[1] = 0;
                    actor.context[2] = slot as i32; // Typed owner-slot identity.
                    actor.context[3] = i32::from(effect); // Typed callback identity.
                    actor.context[5] = 10;
                    for index in [6, 11, 16] {
                        actor.context[index] = -1;
                    }
                }
                if wanted == i32::from(effect) {
                    wanted = -i32::from(effect);
                }
                self.actor_entries[entry].effect = effect.wrapping_neg();
            }
            self.actors[slot] = actor;
        }
        Ok(wanted)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GeneratedActor {
    pub active: bool,
    pub entry: usize,
    pub position: [i32; 3],
    pub distance: i32,
    /// Sound context; words 2/3 use typed owner-slot/callback identities in Rust.
    pub context: [i32; 21],
}
impl GeneratedActor {
    fn stop(&mut self, sound: &mut impl GenerationSound) {
        for index in [6, 11, 16] {
            if self.context[index] >= 0 {
                self.context[index] = sound.stop_note(self.context[index]);
            }
        }
        self.context[0] = 0;
        self.active = false;
    }
    fn update_sound(
        &mut self,
        effect: i8,
        range: i32,
        sound: &mut impl GenerationSound,
    ) -> Result<(), GenerationFault> {
        if self.context[0] <= 0 {
            return Ok(());
        }
        for index in [6, 11, 16] {
            self.context[index + 1] = -1;
            self.context[index + 2] = 0;
            self.context[index + 3] = 127;
            self.context[index + 4] = 64;
        }
        self.context[4] = 0;
        let divisor = self.context[5];
        crate::generation_effect(effect, &mut self.context, self.distance, range, divisor)?;
        let motion = self.context[4];
        if motion >= 0 {
            for index in [6, 11, 16] {
                let id = self.context[index + 1];
                if id >= 0 {
                    if self.context[index] >= 0 {
                        sound.stop_note(self.context[index]);
                    }
                    sound.volume_offset(self.context[index + 2]);
                    let period = self.context[index + 3];
                    let counter = self.context[index + 4];
                    self.context[index] = sound.play_note(
                        id.wrapping_shl(4),
                        period.wrapping_sub(
                            period
                                .checked_div(divisor)
                                .ok_or(GenerationFault::InvalidDivision)?
                                .wrapping_mul(motion),
                        ),
                        counter.wrapping_sub(
                            counter
                                .checked_div(divisor)
                                .ok_or(GenerationFault::InvalidDivision)?
                                .wrapping_mul(motion),
                        ),
                    );
                } else if id == -2 && self.context[index] >= 0 {
                    sound.stop_note(self.context[index]);
                }
            }
        }
        self.context[1] = self.context[1].wrapping_add(1);
        Ok(())
    }
}

/// Original sound scheduling supplies commands to a separately implemented
/// audio backend; this does not claim SPU playback or hardware voice timing.
pub trait GenerationSound {
    fn stop_note(&mut self, handle: i32) -> i32;
    fn volume_offset(&mut self, offset: i32);
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32;
}
struct StoppedSound;
impl GenerationSound for StoppedSound {
    fn stop_note(&mut self, _handle: i32) -> i32 {
        -1
    }
    fn volume_offset(&mut self, _offset: i32) {}
    fn play_note(&mut self, _note: i32, _period: i32, _counter: i32) -> i32 {
        -1
    }
}

pub fn choose_texture(day_number: i32, rng: &mut PsxRng) -> Result<usize, GenerationFault> {
    let modulus = day_number.wrapping_sub(1) % 40 / 10 + 1;
    let value = i32::from(rng.next_u15())
        .checked_rem(modulus)
        .ok_or(GenerationFault::InvalidDivision)?;
    usize::try_from(value).map_err(|_| GenerationFault::InvalidEntry)
}
pub fn choose_music(stage: i32, track_override: &mut i32, rng: &mut PsxRng) -> usize {
    let mut index = usize::from(rng.next_u15() % 5);
    if stage == 9 {
        if index == 2 {
            index = 3;
        }
        if *track_override == 3 {
            *track_override = 4;
        }
    }
    if *track_override != 0 {
        track_override.wrapping_sub(1) as usize
    } else {
        index
    }
}
pub fn shift_generation_color(color: [u8; 3], step: i32) -> [u8; 3] {
    [
        color[0].wrapping_sub(step as u8),
        color[1].wrapping_sub(step as u8),
        color[2].wrapping_add(step as u8),
    ]
}
