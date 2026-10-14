use crate::{
    BodyGeometryQuery, CollisionBounds, CollisionFace, CollisionFault, MotionAxis, MotionRequest,
    PsxMathTables,
};

#[derive(Debug, Clone)]
pub struct CollisionModel {
    pub bounds: Option<CollisionBounds>,
    pub faces: Vec<CollisionFace>,
}
impl CollisionModel {
    /// Original model cuboid, optional movement extension, then cached world rotation.
    pub fn prepared_cloud(
        &self,
        world: crate::FixedMatrix,
        extension: Option<(bool, i16)>,
    ) -> Option<crate::CollisionCloud> {
        let mut cloud = self.bounds?.cloud();
        if let Some((strafe, amount)) = extension {
            cloud.extend(strafe, amount);
        }
        Some(crate::CollisionCloud(
            cloud.0.map(|point| world.apply_saturated(point)),
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CollisionTile {
    pub model: usize,
    pub translation: [i32; 3],
    pub heading: i16,
    pub category: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct BodyQueryState {
    pub position: [i32; 3],
    pub heading: i16,
    pub bounds: CollisionBounds,
    pub motion: MotionRequest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollisionPolicy {
    pub ignore_low_midpoint: bool,
}

impl CollisionPolicy {
    /// map_scene startup sets the original global collision filter by location.
    pub const fn for_stage(stage: u8) -> Self {
        Self {
            ignore_low_midpoint: matches!(stage, 0 | 3 | 5 | 6),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CollisionChunk {
    pub policy: CollisionPolicy,
    pub models: Vec<CollisionModel>,
    pub cells: Vec<Vec<CollisionTile>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldContact {
    pub cell: usize,
    pub tile: usize,
    pub model: usize,
    pub position: [i32; 3],
}

impl CollisionChunk {
    /// The single-cell floor query tries the negative Y segment before positive Y.
    pub fn floor(
        &self,
        position: [i32; 3],
        tables: &PsxMathTables,
    ) -> Result<Option<WorldContact>, CollisionFault> {
        let x = position[0] / 2048;
        let z = position[2] / 2048;
        if !(0..20).contains(&x) || !(0..20).contains(&z) {
            return Ok(None);
        }
        let cell = z as usize * 20 + x as usize;
        self.floor_in_cell(cell, position, tables)
    }

    /// Streamed floor traversal retains the owner's cell index in neighboring layers.
    pub fn floor_in_cell(
        &self,
        cell: usize,
        position: [i32; 3],
        tables: &PsxMathTables,
    ) -> Result<Option<WorldContact>, CollisionFault> {
        let Some(tiles) = self.cells.get(cell) else {
            return Ok(None);
        };
        for (index, tile) in tiles.iter().enumerate() {
            let model = &self.models[tile.model];
            let delta: [i16; 3] = std::array::from_fn(|axis| {
                position[axis].wrapping_sub(tile.translation[axis]) as i16
            });
            let inverse = tables.rotation([0, tile.heading.wrapping_neg(), 0]);
            let start = inverse.apply_saturated(delta);
            for offset in [-1024i16, 1024] {
                let mut end = start;
                end[1] = end[1].wrapping_add(offset);
                if let Some(hit) = crate::intersect_model(&model.faces, start, end, tables)? {
                    let mut corrected = position;
                    for axis in 0..3 {
                        corrected[axis] = corrected[axis]
                            .wrapping_add(i32::from(hit.position[axis]) - i32::from(start[axis]));
                    }
                    return Ok(Some(WorldContact {
                        cell,
                        tile: index,
                        model: tile.model,
                        position: corrected,
                    }));
                }
            }
        }
        Ok(None)
    }

    /// Legacy one-chunk traversal. Streamed queries use the original selected rectangles.
    pub fn body(
        &self,
        position: [i32; 3],
        heading: i16,
        bounds: CollisionBounds,
        motion: MotionRequest,
        tables: &PsxMathTables,
    ) -> Result<Option<WorldContact>, CollisionFault> {
        let query = BodyQueryState {
            position,
            heading,
            bounds,
            motion,
        };
        let mut contact = None;
        for (cell, tiles) in self.cells.iter().enumerate() {
            for index in 0..tiles.len() {
                if let Some(hit) = self.body_tile(query, cell, index, tables)? {
                    contact = Some(hit);
                }
            }
        }
        Ok(contact)
    }

    pub fn body_tile(
        &self,
        query: BodyQueryState,
        cell: usize,
        index: usize,
        tables: &PsxMathTables,
    ) -> Result<Option<WorldContact>, CollisionFault> {
        let tile = &self.cells[cell][index];
        if tile.flags & 0x80 == 0 {
            return Ok(None);
        }
        let model = &self.models[tile.model];
        let Some(model_bounds) = model.bounds else {
            return Ok(None);
        };
        let delta: [i32; 3] =
            std::array::from_fn(|axis| query.position[axis].wrapping_sub(tile.translation[axis]));
        if delta.iter().any(|value| value.wrapping_abs() > 16384) {
            return Ok(None);
        }
        let mut cloud = query.bounds.cloud();
        cloud.extend(
            query.motion.axis == MotionAxis::Strafe,
            query.motion.amount as i16,
        );
        let rotation = tables.rotation([0, query.heading, 0]);
        cloud.0 = cloud.0.map(|point| rotation.apply_saturated(point));
        let inverse = tables.inverse_rotation_chain([0, tile.heading, 0], &[]);
        Ok(BodyGeometryQuery {
            world_delta: delta,
            inverse,
            cloud,
            ignore_low_midpoint: self.policy.ignore_low_midpoint,
        }
        .intersect(model_bounds, &model.faces, tables)?
        .map(|hit| {
            let rotated = tables
                .rotation([0, tile.heading, 0])
                .apply_saturated(hit.position);
            WorldContact {
                cell,
                tile: index,
                model: tile.model,
                position: std::array::from_fn(|axis| {
                    tile.translation[axis].wrapping_add(i32::from(rotated[axis]))
                }),
            }
        }))
    }
}
