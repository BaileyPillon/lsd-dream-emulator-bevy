use crate::PsxMathTables;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollisionFace {
    Triangle([[i16; 3]; 3]),
    Quad([[i16; 3]; 4]),
}

impl CollisionFace {
    pub fn points(&self) -> &[[i16; 3]] {
        match self {
            Self::Triangle(points) => points,
            Self::Quad(points) => points,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentHit {
    pub distance: i32,
    pub position: [i16; 3],
    pub face: usize,
    pub height_from_face_minimum: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionFault {
    DivideOverflow,
}

impl std::fmt::Display for CollisionFault {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("original collision arithmetic divide overflow")
    }
}
impl std::error::Error for CollisionFault {}

fn dot(left: [i32; 3], right: [i32; 3]) -> i32 {
    left.into_iter()
        .zip(right)
        .fold(0i32, |sum, (a, b)| sum.wrapping_add(a.wrapping_mul(b)))
}

fn length(vector: [i32; 3], tables: &PsxMathTables) -> i32 {
    // Square0 loads the signed halfword IR registers before squaring.
    let vector = vector.map(|value| i32::from(value as i16));
    tables.square_root(dot(vector, vector)) as i32
}

/// Original plane/expanded-box segment query, including its quantized Q16 fraction.
/// The reference uses each face's axis bounds, rather than a barycentric triangle test.
pub fn intersect_model(
    faces: &[CollisionFace],
    start: [i16; 3],
    end: [i16; 3],
    tables: &PsxMathTables,
) -> Result<Option<SegmentHit>, CollisionFault> {
    let origin = start.map(i32::from);
    let direction = std::array::from_fn(|axis| i32::from(end[axis].wrapping_sub(start[axis])));
    let mut nearest: Option<SegmentHit> = None;
    for (face_index, face) in faces.iter().enumerate() {
        let points = face.points();
        let edge_a: [i32; 3] =
            std::array::from_fn(|axis| i32::from(points[1][axis].wrapping_sub(points[0][axis])));
        let edge_b: [i32; 3] =
            std::array::from_fn(|axis| i32::from(points[2][axis].wrapping_sub(points[0][axis])));
        let normal = [
            edge_a[1]
                .wrapping_mul(edge_b[2])
                .wrapping_sub(edge_a[2].wrapping_mul(edge_b[1])),
            edge_a[2]
                .wrapping_mul(edge_b[0])
                .wrapping_sub(edge_a[0].wrapping_mul(edge_b[2])),
            edge_a[0]
                .wrapping_mul(edge_b[1])
                .wrapping_sub(edge_a[1].wrapping_mul(edge_b[0])),
        ]
        .map(|value| value / 4096);
        let mut denominator = dot(direction, normal);
        if denominator == 0 || denominator == i32::MIN {
            continue;
        }
        let plane = dot(points[0].map(i32::from), normal).wrapping_neg();
        let mut numerator = dot(origin, normal).wrapping_add(plane).wrapping_neg();
        if denominator.wrapping_abs() >= 4096 {
            numerator /= 4096;
            denominator /= 4096;
        }
        // The original raises a divide exception for this one overflow pair.
        if numerator == i32::MIN && denominator == -1 {
            return Err(CollisionFault::DivideOverflow);
        }
        let whole = numerator / denominator;
        let remainder = numerator % denominator;
        let integer = if whole != 0 {
            whole.wrapping_shl(16)
        } else {
            numerator.wrapping_mul(denominator) & i32::MIN
        };
        let fraction = remainder.wrapping_abs().wrapping_shl(16) / denominator.wrapping_abs();
        let parameter = integer | fraction;
        if parameter < 0 {
            continue;
        }
        let displacement = direction.map(|value| value.wrapping_mul(parameter) >> 16);
        let position: [i16; 3] =
            std::array::from_fn(|axis| origin[axis].wrapping_add(displacement[axis]) as i16);
        let distance = length(displacement, tables);
        if length(direction, tables) < distance {
            continue;
        }
        let minimum: [i16; 3] =
            std::array::from_fn(|axis| points.iter().map(|point| point[axis]).min().unwrap());
        let maximum: [i16; 3] =
            std::array::from_fn(|axis| points.iter().map(|point| point[axis]).max().unwrap());
        if (0..3).any(|axis| {
            i32::from(position[axis]) < i32::from(minimum[axis]) - 24
                || i32::from(position[axis]) > i32::from(maximum[axis]) + 24
        }) {
            continue;
        }
        if nearest.is_none_or(|hit| distance < hit.distance) {
            nearest = Some(SegmentHit {
                distance,
                position,
                face: face_index,
                height_from_face_minimum: i32::from(position[1]) - i32::from(minimum[1]),
            });
        }
    }
    Ok(nearest)
}
