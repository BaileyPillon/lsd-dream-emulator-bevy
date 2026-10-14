use crate::{
    CollisionBounds, CollisionCloud, CollisionFace, CollisionFault, FixedMatrix, PsxMathTables,
    SegmentHit,
};

#[derive(Debug, Clone, Copy)]
pub struct BodyGeometryQuery {
    pub world_delta: [i32; 3],
    pub inverse: FixedMatrix,
    pub cloud: CollisionCloud,
    pub ignore_low_midpoint: bool,
}
impl BodyGeometryQuery {
    pub fn intersect(
        self,
        bounds: CollisionBounds,
        faces: &[CollisionFace],
        tables: &PsxMathTables,
    ) -> Result<Option<SegmentHit>, CollisionFault> {
        if self
            .world_delta
            .iter()
            .any(|value| value.wrapping_abs() > 16384)
        {
            return Ok(None);
        }
        let offset = self
            .inverse
            .apply_saturated(self.world_delta.map(|value| value as i16));
        let local = CollisionCloud(self.cloud.0.map(|point| {
            let rotated = self.inverse.apply_saturated(point);
            std::array::from_fn(|axis| rotated[axis].wrapping_add(offset[axis]))
        }));
        if !bounds.overlaps(CollisionBounds::from_points(&local.0).unwrap()) {
            return Ok(None);
        }
        local.intersection(bounds, faces, self.ignore_low_midpoint, tables)
    }
}
