use crate::{
    BodyContactState, BodyQueryState, CollisionBounds, CollisionChunk, CollisionFault, LinkError,
    MotionRequest, PsxMathTables, SceneGridError, SceneLocation, SceneSlots, WorldContact,
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
pub struct BodyCandidate {
    pub slot: usize,
    pub cell: usize,
    pub tile: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyVisit {
    Cell {
        slot: usize,
        cell: [i8; 2],
    },
    Hit {
        candidate: BodyCandidate,
        contact: SceneContact,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneCollisionError {
    Day(crate::DayFault),
    Grid(SceneGridError),
    Geometry(CollisionFault),
    MissingReadyChunk(usize),
    Link(LinkError),
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
impl From<LinkError> for SceneCollisionError {
    fn from(error: LinkError) -> Self {
        Self::Link(error)
    }
}
impl From<crate::DayFault> for SceneCollisionError {
    fn from(error: crate::DayFault) -> Self {
        Self::Day(error)
    }
}
impl fmt::Display for SceneCollisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "scene collision query failed: {self:?}")
    }
}
impl std::error::Error for SceneCollisionError {}

impl SceneCollision {
    /// A failed owner lookup does not issue the original local floor-miss signal.
    pub fn actor_floor_query(
        &self,
        position: [i32; 3],
        tables: &PsxMathTables,
    ) -> Result<crate::ActorFloorQuery, SceneCollisionError> {
        if self.slots.locate(position)?.is_none() {
            return Ok(crate::ActorFloorQuery::Unlocated);
        }
        Ok(match self.floor(position, tables)? {
            Some(contact) => crate::ActorFloorQuery::Hit(contact),
            None => crate::ActorFloorQuery::Miss,
        })
    }
    pub fn body_candidates(
        &self,
        location: SceneLocation,
    ) -> Result<Vec<BodyCandidate>, SceneCollisionError> {
        let mut result = Vec::new();
        for query in self.slots.body_queries(location)? {
            let Some(chunk) = self.ready_chunk(query.slot)? else {
                continue;
            };
            for dz in 0..query.cells.rows {
                for dx in 0..query.cells.columns {
                    let x = i32::from(query.cells.x) + dx;
                    let z = i32::from(query.cells.z) + dz;
                    if !(0..20).contains(&x) || !(0..20).contains(&z) {
                        return Err(SceneGridError::InvalidSlot.into());
                    }
                    let cell = z as usize * 20 + x as usize;
                    for (tile, entry) in chunk.cells[cell].iter().enumerate() {
                        if entry.flags & 0x80 != 0 {
                            result.push(BodyCandidate {
                                slot: query.slot,
                                cell,
                                tile,
                            });
                        }
                    }
                }
            }
        }
        Ok(result)
    }
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

    /// Preserves rectangle/chain order and the last successful geometry contact.
    /// Original actor callback side effects remain separate.
    pub fn body(
        &self,
        position: [i32; 3],
        heading: i16,
        bounds: CollisionBounds,
        motion: MotionRequest,
        tables: &PsxMathTables,
    ) -> Result<Option<SceneContact>, SceneCollisionError> {
        self.body_with(
            &mut BodyQueryState {
                position,
                heading,
                bounds,
                motion,
            },
            tables,
            |_, _| Ok(()),
        )
    }

    /// Rectangles are selected once; each later tile reads the live query after callbacks.
    pub fn body_with(
        &self,
        query: &mut BodyQueryState,
        tables: &PsxMathTables,
        mut observer: impl FnMut(&mut BodyQueryState, BodyVisit) -> Result<(), SceneCollisionError>,
    ) -> Result<Option<SceneContact>, SceneCollisionError> {
        let Some(location) = self.slots.locate(query.position)? else {
            return Ok(None);
        };
        if !self.slots.slots[location.slot].ready {
            return Ok(None);
        }
        let mut contact = BodyContactState { current: None };
        for rectangle in self.slots.body_queries(location)? {
            let Some(chunk) = self.ready_chunk(rectangle.slot)? else {
                continue;
            };
            for dz in 0..rectangle.cells.rows {
                for dx in 0..rectangle.cells.columns {
                    let x = i32::from(rectangle.cells.x) + dx;
                    let z = i32::from(rectangle.cells.z) + dz;
                    if !(0..20).contains(&x) || !(0..20).contains(&z) {
                        return Err(SceneGridError::InvalidSlot.into());
                    }
                    observer(
                        query,
                        BodyVisit::Cell {
                            slot: rectangle.slot,
                            cell: [x as i8, z as i8],
                        },
                    )?;
                    let cell = z as usize * 20 + x as usize;
                    for (tile, entry) in chunk.cells[cell].iter().enumerate() {
                        if entry.flags & 0x80 == 0 {
                            continue;
                        }
                        let origin = self.slots.slots[rectangle.slot].origin;
                        let local = std::array::from_fn(|axis| {
                            query.position[axis].wrapping_sub(origin[axis])
                        });
                        if let Some(hit) = chunk.body_tile(
                            BodyQueryState {
                                position: local,
                                ..*query
                            },
                            cell,
                            tile,
                            tables,
                        )? {
                            let hit = self.world_contact(rectangle.slot, hit);
                            contact.notify(hit, 4);
                            observer(
                                query,
                                BodyVisit::Hit {
                                    candidate: BodyCandidate {
                                        slot: rectangle.slot,
                                        cell,
                                        tile,
                                    },
                                    contact: hit,
                                },
                            )?;
                        }
                    }
                }
            }
        }
        Ok(contact.current)
    }
}
