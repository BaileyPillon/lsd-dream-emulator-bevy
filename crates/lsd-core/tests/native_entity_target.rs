use lsd_core::{EntityTarget, FixedMatrix};
use serde_json::{Value, json};
#[test]
fn original_target_projection_long_matrix_height_and_cube_bounds_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/entity-target.json")).unwrap();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let target = EntityTarget {
            position: std::array::from_fn(|i| case["origin"][i].as_i64().unwrap() as i32),
            local_y: case["origin"][1].as_i64().unwrap() as i32,
            matrix: FixedMatrix(std::array::from_fn(|r| {
                std::array::from_fn(|c| case["matrix"][r][c].as_i64().unwrap() as i16)
            })),
            eye: std::array::from_fn(|i| case["view"][0][i].as_i64().unwrap() as i32),
            target: std::array::from_fn(|i| case["view"][1][i].as_i64().unwrap() as i32),
        };
        let forward = case["forward"].as_i64().unwrap() as i32;
        let point = std::array::from_fn(|i| case["point"][i].as_i64().unwrap() as i32);
        let radius = case["radius"].as_i64().unwrap() as i32;
        assert_eq!(
            json!({"resolved":target.resolve(forward).unwrap(),"inside":target.proximity(point,forward,radius).unwrap()}),
            case["expected"],
            "target case {number}"
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 720);
}
