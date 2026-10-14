use lsd_core::{CollisionBounds, CollisionModel, FixedMatrix};
use serde_json::Value;
#[test]
fn original_bounds_cube_extension_and_cached_world_matrix_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/prepared-cloud.json")).unwrap();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let bounds = CollisionBounds {
            minimum: std::array::from_fn(|i| input["minimum"][i].as_i64().unwrap() as i16),
            maximum: std::array::from_fn(|i| input["maximum"][i].as_i64().unwrap() as i16),
        };
        let matrix = FixedMatrix(std::array::from_fn(|r| {
            std::array::from_fn(|c| input["matrix"][r][c].as_i64().unwrap() as i16)
        }));
        let extension = if input["extension"].is_null() {
            None
        } else {
            Some((
                input["extension"][0].as_bool().unwrap(),
                input["extension"][1].as_i64().unwrap() as i16,
            ))
        };
        let mut local = bounds.cloud();
        if let Some((strafe, amount)) = extension {
            local.extend(strafe, amount);
        }
        assert_eq!(
            serde_json::to_value(local.0).unwrap(),
            case["local_cloud"],
            "local case {number}"
        );
        let model = CollisionModel {
            bounds: Some(bounds),
            faces: vec![],
        };
        assert_eq!(
            serde_json::to_value(model.prepared_cloud(matrix, extension).unwrap().0).unwrap(),
            case["world_cloud"],
            "world case {number}"
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 504);
}
