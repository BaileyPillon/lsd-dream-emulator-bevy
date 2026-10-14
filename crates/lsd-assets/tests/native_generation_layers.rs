#[path = "../../lsd-core/tests/support/generation_layers.rs"]
mod support;
use lsd_assets::{load_generation_effect_tables, load_generation_tables};
use lsd_core::{GenerationLayers, generation_border_moves};
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
#[ignore = "Requires original executable and private original-content layer CPU observations"]
fn original_generation_layer_content_loads_and_matches_native_updates() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_GENERATION_LAYER_CASES")
                .expect("Set LSD_NATIVE_GENERATION_LAYER_CASES"),
        )
        .unwrap(),
    )
    .unwrap();
    let tables = load_generation_tables(&executable).unwrap();
    let geometry = load_generation_effect_tables(&executable).unwrap();
    assert_eq!(geometry, support::geometry(&fixture["geometry"]));
    assert_eq!(json!(tables.effect_palettes), fixture["palettes"]);
    for case in fixture["cases"].as_array().unwrap() {
        let flags = case["flags"].as_i64().unwrap() as i32;
        let palette = case["palette"].as_u64().unwrap() as usize;
        let mut layers = GenerationLayers::default();
        layers.create(
            &tables,
            &geometry,
            case["environment"]
                .as_bool()
                .unwrap()
                .then_some([33, 66, 99]),
            flags,
            palette,
        );
        assert_eq!(support::snapshot(&layers), case["expected"]);
        for update in case["updates"].as_array().unwrap() {
            layers.update(
                &tables,
                &geometry,
                flags,
                palette,
                [249, 3, 254],
                [
                    update["eye"].as_i64().unwrap() as i32,
                    update["target"].as_i64().unwrap() as i32,
                ],
            );
            assert_eq!(support::snapshot(&layers), update["expected"]);
        }
    }
    for case in fixture["borders"].as_array().unwrap() {
        let commands: Vec<_> =
            generation_border_moves(case["stage"].as_i64().unwrap() as i32, &geometry)
                .into_iter()
                .map(|v| {
                    json!([
                        "move",
                        [
                            i32::from(v.source.x),
                            i32::from(v.source.y),
                            v.source.width,
                            v.source.height
                        ],
                        v.destination
                    ])
                })
                .collect();
        assert_eq!(json!(commands), case["expected"]);
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 12);
}
