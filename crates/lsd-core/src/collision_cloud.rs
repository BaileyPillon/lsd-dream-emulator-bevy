use crate::{CollisionFace, CollisionFault, PsxMathTables, SegmentHit, intersect_model};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionBounds {
    pub minimum: [i16; 3],
    pub maximum: [i16; 3],
}

impl CollisionBounds {
    pub fn from_points(points: &[[i16; 3]]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        Some(Self {
            minimum: std::array::from_fn(|axis| points.iter().map(|p| p[axis]).min().unwrap()),
            maximum: std::array::from_fn(|axis| points.iter().map(|p| p[axis]).max().unwrap()),
        })
    }

    fn outcode(self, point: [i16; 3]) -> u8 {
        let mut flags = 0;
        for (axis, low, high) in [(0, 4, 8), (1, 1, 2), (2, 16, 32)] {
            if self.maximum[axis] < point[axis] {
                flags |= high;
            } else if point[axis] < self.minimum[axis] {
                flags |= low;
            }
        }
        flags
    }

    /// Exact discrete subdivision used by func_8001E110 with no midpoint output.
    pub fn segment_code(self, a: [i16; 3], b: [i16; 3]) -> u8 {
        let fa = self.outcode(a);
        let fb = self.outcode(b);
        if fa == 0 {
            return if fb == 0 { 1 } else { 2 };
        }
        if fb == 0 {
            return 3;
        }
        if fa & fb != 0 {
            return 0;
        }
        let half = midpoint(a, b);
        if half == a || half == b {
            return 0;
        }
        let first = self.segment_code(a, half);
        if first != 0 {
            first
        } else {
            self.segment_code(half, b)
        }
    }

    pub fn overlaps(self, other: Self) -> bool {
        (0..3).all(|axis| {
            self.minimum[axis] <= other.maximum[axis] && other.minimum[axis] <= self.maximum[axis]
        })
    }

    pub fn cloud(self) -> CollisionCloud {
        let [a, b, c] = self.minimum;
        let [x, y, z] = self.maximum;
        CollisionCloud([
            [a, b, c],
            [a, y, c],
            [x, y, c],
            [x, b, c],
            [a, b, z],
            [a, y, z],
            [x, y, z],
            [x, b, z],
        ])
    }
}

fn midpoint(a: [i16; 3], b: [i16; 3]) -> [i16; 3] {
    std::array::from_fn(|axis| ((i32::from(a[axis]) + i32::from(b[axis])) >> 1) as i16)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionCloud(pub [[i16; 3]; 8]);

impl CollisionCloud {
    /// The native forward/strafe preparation reorders the cube for X motion.
    pub fn extend(&mut self, strafe: bool, amount: i16) {
        self.extend_with_direction(strafe, amount >= 0, i32::from(amount));
    }

    /// The original corner side follows the movement sign before margin adjustment.
    pub fn extend_with_direction(&mut self, strafe: bool, non_negative: bool, amount: i32) {
        if strafe {
            let old = self.0;
            self.0 = [
                old[4], old[5], old[1], old[0], old[7], old[6], old[2], old[3],
            ];
        }
        let indices = if non_negative { 4..8 } else { 0..4 };
        for index in indices {
            let axis = if strafe { 0 } else { 2 };
            self.0[index][axis] = self.0[index][axis].wrapping_add(amount as i16);
        }
    }

    pub fn intersection(
        self,
        bounds: CollisionBounds,
        faces: &[CollisionFace],
        ignore_low_midpoint: bool,
        tables: &PsxMathTables,
    ) -> Result<Option<SegmentHit>, CollisionFault> {
        let a = midpoint(self.0[0], self.0[2]);
        let b = midpoint(self.0[4], self.0[6]);
        if bounds.segment_code(a, b) != 0
            && let Some(hit) = intersect_model(faces, a, b, tables)?
            && (!ignore_low_midpoint || hit.height_from_face_minimum > 512)
        {
            return Ok(Some(hit));
        }
        let mut last_contact: Option<SegmentHit> = None;
        let mut collided = false;
        // The original only tests corner indices 1 and 2 after the midpoint miss.
        for index in [1, 2] {
            let a = self.0[index];
            let b = self.0[index + 4];
            if bounds.segment_code(a, b) != 0
                && let Some(hit) = intersect_model(faces, a, b, tables)?
            {
                collided |= hit.height_from_face_minimum > 512;
                // Each native geometry call resets its distance and can overwrite contact
                // storage even when this corner's low-height hit is rejected by the caller.
                last_contact = Some(hit);
            }
        }
        Ok(if collided { last_contact } else { None })
    }
}
