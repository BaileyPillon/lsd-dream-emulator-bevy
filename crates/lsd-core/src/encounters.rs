//! Original chunk encounter selection. NPC behavior and the resource host are separate.
use crate::{PsxRng, SpawnLocation};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterChunk {
    pub chunk: i16,
    pub day_gate: i8,
    pub objects: [i8; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterObject {
    pub used: i8,
    pub gate: i8,
    pub model: i8,
    pub entity: u8,
    pub variants: [i8; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterVariant {
    pub tile: i16,
    pub rotation: i8,
    pub position: i8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterTables {
    pub chunks: [Vec<EncounterChunk>; 14],
    pub objects: [Vec<EncounterObject>; 14],
    pub color_lookup: Vec<i8>,
    pub variants: Vec<EncounterVariant>,
    pub positions: Vec<[i16; 3]>,
    pub rotations: Vec<[u32; 3]>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterFrame {
    pub stage: i32,
    pub day: i32,
    pub palette: i32,
    pub generation_type: i32,
    pub chunk: i16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterSpawn {
    pub entity: u8,
    pub model: i8,
    pub variant: u8,
    pub spawn: SpawnLocation,
    pub rotation: [u32; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterFault {
    Stage,
    Chunk,
    Object,
    Variant,
    Color,
    Model,
}
impl fmt::Display for EncounterFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported encounter input: {self:?}")
    }
}
impl std::error::Error for EncounterFault {}
pub trait EncounterHost {
    fn prepare_library(&mut self) -> bool;
    /// Called before actor creation, including unavailable models.
    fn enable_teleports(&mut self);
    fn select_model(&mut self, index: i8) -> bool;
    fn create_actor(&mut self, spawn: EncounterSpawn) -> bool;
    fn activate_gray_man(&mut self);
}
pub fn encounter_object_gate(
    object: &mut EncounterObject,
    day: i32,
    palette: i32,
    generation_type: i32,
    colors: &[i8],
) -> Result<bool, EncounterFault> {
    let raw = i32::from(object.gate);
    if raw == 1 {
        object.used = 1;
        return Ok(true);
    }
    if raw < 0 && object.used != 0 {
        return Ok(false);
    }
    let code = raw.abs();
    let pass = match code {
        2..=4 => {
            let month = day.wrapping_sub(1) / 30 + 1;
            (0..4).any(|i| month == code - 1 + i * 3)
        }
        5 => day % 3 == 0,
        6 => day % 3 != 0,
        7 => generation_type & 1 == 0,
        8 | 9 => day % 3 == code - 7,
        20 => day & 1 == 0,
        21 => day & 1 != 0,
        0..10 => true,
        _ => i32::from(*colors.get(code as usize).ok_or(EncounterFault::Color)?) == palette,
    };
    if pass {
        object.used = 1;
    }
    Ok(pass)
}
impl EncounterTables {
    pub fn find_chunk(&self, frame: EncounterFrame) -> Result<Option<usize>, EncounterFault> {
        let stage = usize::try_from(frame.stage)
            .ok()
            .filter(|&v| v < 14)
            .ok_or(EncounterFault::Stage)?;
        let Some(mut index) = self.chunks[stage]
            .iter()
            .position(|v| v.chunk == frame.chunk)
        else {
            return Ok(None);
        };
        if frame.stage == 4 && index == 16 && frame.palette == 4 {
            index += 5;
        }
        if index >= self.chunks[stage].len() {
            return Err(EncounterFault::Chunk);
        }
        Ok(Some(index))
    }
    pub fn spawn_chunk(
        &mut self,
        frame: EncounterFrame,
        rng: &mut PsxRng,
        host: &mut impl EncounterHost,
    ) -> Result<bool, EncounterFault> {
        let Some(index) = self.find_chunk(frame)? else {
            return Ok(false);
        };
        let stage = frame.stage as usize;
        let chunk = self.chunks[stage][index];
        if chunk.day_gate != 0 && i32::from(chunk.day_gate) == frame.day % 2 + 1 {
            if stage != 0 && rng.next_u15().is_multiple_of(12) && frame.day & 1 == 0 {
                host.activate_gray_man();
            }
            return Ok(false);
        }
        if !host.prepare_library() {
            return Ok(false);
        }
        for object in chunk.objects {
            if object == -1 {
                break;
            }
            let index = usize::try_from(object).map_err(|_| EncounterFault::Object)?;
            self.spawn_object(index, chunk.chunk, frame, host)?;
        }
        Ok(true)
    }
    fn spawn_object(
        &mut self,
        index: usize,
        chunk: i16,
        frame: EncounterFrame,
        host: &mut impl EncounterHost,
    ) -> Result<(), EncounterFault> {
        let stage = frame.stage as usize;
        let object = self.objects[stage]
            .get_mut(index)
            .ok_or(EncounterFault::Object)?;
        if !encounter_object_gate(
            object,
            frame.day,
            frame.palette,
            frame.generation_type,
            &self.color_lookup,
        )? {
            return Ok(());
        }
        let object = *object;
        if matches!(object.entity, 0x4e | 0xb | 0x38 | 0x5d) {
            host.enable_teleports();
        }
        if host.select_model(object.model) {
            for variant in object.variants {
                if variant == -1 {
                    break;
                }
                let raw = variant as u8;
                let data = *self
                    .variants
                    .get(usize::from(raw))
                    .ok_or(EncounterFault::Variant)?;
                let position = *usize::try_from(data.position)
                    .ok()
                    .and_then(|v| self.positions.get(v))
                    .ok_or(EncounterFault::Variant)?;
                let rotation = *usize::try_from(data.rotation)
                    .ok()
                    .and_then(|v| self.rotations.get(v))
                    .ok_or(EncounterFault::Variant)?;
                if !host.create_actor(EncounterSpawn {
                    entity: object.entity,
                    model: object.model,
                    variant: raw,
                    spawn: SpawnLocation {
                        chunk,
                        tile: data.tile,
                        position,
                    },
                    rotation,
                }) {
                    return Ok(());
                }
            }
        }
        if object.entity == 2 {
            self.spawn_object(index + 7, chunk, frame, host)?;
        }
        Ok(())
    }
}
