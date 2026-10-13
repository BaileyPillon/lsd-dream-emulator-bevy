use crate::{
    CollisionBounds, CollisionChunk, CollisionFault, MotionRequest, PsxMathTables, SceneGridError,
    SceneSlots, WorldContact,
};
use std::fmt;

#[derive(Debug, Clone)]
pub struct SceneCollision {
    pub slots: SceneSlots,
    pub chunks: [Option<CollisionChunk>; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneContact {
    pub slot: usize,
    pub map_index: i16,
    pub geometry: WorldContact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneCollisionError {
    Grid(SceneGridError),
    Geometry(CollisionFault),
    MissingReadyChunk(usize),
}
impl From<SceneGridError> for SceneCollisionError {
    fn from(error: SceneGridError) -> Self {
        Self::Grid(error)
    }
}
impl From<CollisionFault> for SceneCollisionError {
    fn from(error: CollisionFault) -> Self {
        Self::Geometry(error)
    }
}
impl fmt::Display for SceneCollisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "scene collision query failed: {self:?}")
    }
}
impl std::error::Error for SceneCollisionError {}

impl SceneCollision {
    fn ready_chunk(&self, index: usize) -> Result<Option<&CollisionChunk>, SceneCollisionError> {
        if !self.slots.slots[index].ready {
            return Ok(None);
        }
        self.chunks[index]
            .as_ref()
            .map(Some)
            .ok_or(SceneCollisionError::MissingReadyChunk(index))
    }
    fn world_contact(&self, slot: usize, mut geometry: WorldContact) -> SceneContact {
        let entry = self.slots.slots[slot];
        for (position, offset) in geometry.position.iter_mut().zip(entry.origin) {
            *position = position.wrapping_add(offset);
        }
        SceneContact {
            slot,
            map_index: entry.map_index,
            geometry,
        }
    }

    pub fn floor(
        &self,
        position: [i32; 3],
        tables: &PsxMathTables,
    ) -> Result<Option<SceneContact>, SceneCollisionError> {
        let Some(location) = self.slots.locate(position)? else {
            return Ok(None);
        };
        for query in self.slots.queries(location, 1)? {
            let Some(chunk) = self.ready_chunk(query.slot)? else {
                continue;
            };
            let [x, z] = [query.cells.x, query.cells.z];
            if !(0..20).contains(&x) || !(0..20).contains(&z) {
                continue;
            }
            let origin = self.slots.slots[query.slot].origin;
            let local = std::array::from_fn(|axis| position[axis].wrapping_sub(origin[axis]));
            if let Some(contact) =
                chunk.floor_in_cell(z as usize * 20 + x as usize, local, tables)?
            {
                return Ok(Some(self.world_contact(query.slot, contact)));
            }
        }
        Ok(None)
    }

    /// Current-owner body prototype. Original surrounding-cell/body notification traversal is pending.
    pub fn body(
        &self,
        position: [i32; 3],
        heading: i16,
        bounds: CollisionBounds,
        motion: MotionRequest,
        tables: &PsxMathTables,
    ) -> Result<Option<SceneContact>, SceneCollisionError> {
        let Some(location) = self.slots.locate(position)? else {
            return Ok(None);
        };
        let Some(chunk) = self.ready_chunk(location.slot)? else {
            return Ok(None);
        };
        let origin = self.slots.slots[location.slot].origin;
        let local = std::array::from_fn(|axis| position[axis].wrapping_sub(origin[axis]));
        Ok(chunk
            .body(local, heading, bounds, motion, tables)?
            .map(|hit| self.world_contact(location.slot, hit)))
    }
}
