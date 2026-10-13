use lsd_assets::{formats::Tmd, is_canonical_reference, load_math_tables, sha256_hex};
use lsd_core::intersect_model;
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[test]
#[ignore = "Requires original local assets and private semantic-GTE observations"]
fn decoded_static_models_match_original_semantic_geometry_queries() {
    let root = PathBuf::from(std::env::var("LSD_GAME_FILES").unwrap());
    let exe = std::fs::read(root.join("SLPS_015.56")).unwrap();
    assert!(is_canonical_reference(&exe));
    let tables = load_math_tables(&exe).unwrap();
    let path = std::env::var("LSD_NATIVE_GEOMETRY_CASES").unwrap();
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut models = BTreeMap::new();
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let relative = case["path"].as_str().unwrap();
        if !models.contains_key(relative) {
            let data = std::fs::read(root.join(relative)).unwrap();
            assert_eq!(sha256_hex(&data), case["source_sha256"].as_str().unwrap());
            let offset = case["tmd_offset"].as_u64().unwrap() as usize;
            models.insert(relative.to_string(), Tmd::parse(&data[offset..]).unwrap());
        }
        let object = &models[relative].objects[case["object"].as_u64().unwrap() as usize];
        let point =
            |name: &str| std::array::from_fn(|axis| case[name][axis].as_i64().unwrap() as i16);
        let hit = intersect_model(
            &object.collision_faces(),
            point("start"),
            point("end"),
            &tables,
        )
        .unwrap();
        assert_eq!(
            hit.is_some(),
            case["hit"].as_bool().unwrap(),
            "query {index} {relative}"
        );
        if let Some(hit) = hit {
            assert_eq!(
                hit.distance,
                case["distance"].as_i64().unwrap() as i32,
                "query {index}"
            );
            assert_eq!(
                serde_json::json!(hit.position),
                case["position"],
                "query {index}"
            );
            assert_eq!(
                hit.height_from_face_minimum,
                case["height"].as_i64().unwrap() as i32,
                "query {index}"
            );
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 13037);
}
