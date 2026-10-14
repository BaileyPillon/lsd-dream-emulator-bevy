use lsd_core::{FixedMatrix, SceneScale, SceneScaleTables};
use serde_json::{Value, json};

fn compare(value: &Value, tables: SceneScaleTables) {
    for (case_index, case) in value["cases"].as_array().unwrap().iter().enumerate() {
        let mut scale = SceneScale::new(tables);
        let initial = &case["initial"];
        scale.remaining = initial["remaining"].as_i64().unwrap() as i32;
        scale.selected = initial["selected"].as_u64().map(|v| v as usize);
        scale.scale = std::array::from_fn(|i| initial["scale"][i].as_i64().unwrap() as i32);
        for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let operation = &step["operation"];
            let changed = match operation[0].as_str().unwrap() {
                "request" => {
                    scale.request(
                        operation[1].as_i64().unwrap() as i32,
                        operation[2].as_i64().unwrap() as i32,
                    );
                    false
                }
                "step" => scale.step().unwrap(),
                "reset" => scale.reset().unwrap(),
                _ => unreachable!(),
            };
            assert_eq!(
                json!({"scale":scale.scale,"remaining":scale.remaining,"selected":scale.selected,
                "visits": if changed {2870} else {0}}),
                step["state"],
                "case {case_index} step {index}"
            );
        }
    }
    // The original collision backend asks for rotation alone, even at zero or
    // negative visual scale. Do not invert the visual scale for these queries.
    for row in value["inverse_rotation"].as_array().unwrap() {
        assert_eq!(
            json!(
                FixedMatrix::IDENTITY
                    .0
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
            ),
            row["matrix"]
        );
    }
}

#[test]
fn persistent_map_scale_matches_original_traversal_count_reset_and_overflow() {
    let value: Value =
        serde_json::from_str(include_str!("fixtures/scene-scale-runtime.json")).unwrap();
    let tables = SceneScaleTables {
        packets: std::array::from_fn(|row| {
            std::array::from_fn(|axis| {
                value["owned_scale_packets"][row][axis].as_u64().unwrap() as u32
            })
        }),
    };
    compare(&value, tables);
}

#[test]
#[ignore = "Requires original executable and private original scene traversal observations"]
fn canonical_map_scale_matches_original_cpu() {
    let executable = std::fs::read(
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap()).join("SLPS_015.56"),
    )
    .unwrap();
    let value: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_NATIVE_SCENE_SCALE_RUNTIME").unwrap()).unwrap(),
    )
    .unwrap();
    compare(
        &value,
        lsd_assets::load_scene_scale_tables(&executable).unwrap(),
    );
}
