use lsd_core::{BodyGeometryQuery, CollisionBounds, CollisionCloud, CollisionFace, PsxMathTables};
use serde_json::Value;
fn points<const N: usize>(value: &Value) -> [[i16; 3]; N] {
    std::array::from_fn(|i| std::array::from_fn(|axis| value[i][axis].as_i64().unwrap() as i16))
}
#[test]
fn body_transform_geometry_result_matches_complete_original_hit_notification_chain() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/native-hit-chain.json")).unwrap();
    let mut tables = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    tables.sin_cos[0] = 4096 << 16;
    tables.sin_cos[1024] = 4096;
    tables.sin_cos[2048] = u32::from((-4096i16) as u16) << 16;
    tables.sin_cos[3072] = u32::from((-4096i16) as u16);
    for (index, value) in tables.sqrt_window[320..].iter_mut().enumerate() {
        *value = (((index + 64) as f64).sqrt() * 512.0) as i16;
    }
    let mut hits = 0;
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let angles: Vec<[i16; 3]> = case["angles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|angle| std::array::from_fn(|axis| angle[axis].as_i64().unwrap() as i16))
            .collect();
        let face = points::<4>(&case["face"]);
        let query = BodyGeometryQuery {
            world_delta: std::array::from_fn(|axis| {
                case["actor_position"][axis].as_i64().unwrap() as i32
                    - case["tile_position"][axis].as_i64().unwrap() as i32
            }),
            inverse: tables.inverse_rotation_chain(angles[0], &angles[1..]),
            cloud: CollisionCloud(points(&case["cloud"])),
            ignore_low_midpoint: case["filter_low"].as_bool().unwrap(),
        };
        let hit = query
            .intersect(
                CollisionBounds::from_points(&face).unwrap(),
                &[CollisionFace::Quad(face)],
                &tables,
            )
            .unwrap();
        assert_eq!(
            hit.is_some(),
            case["hit"].as_bool().unwrap(),
            "case {index}"
        );
        hits += usize::from(hit.is_some());
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 120);
    assert_eq!(hits, 36);
}
