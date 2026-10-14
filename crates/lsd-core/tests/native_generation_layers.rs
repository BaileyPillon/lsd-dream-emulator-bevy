#[path = "support/generation.rs"]
mod generation_support;
#[path = "support/generation_layers.rs"]
mod support;
use lsd_core::{GenerationLayers, generation_border_moves};
use serde_json::{Value, json};

#[test]
fn original_generation_layers_parent_setup_colors_offsets_cleanup_and_border_commands_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/generation-layers.json")).unwrap();
    let baseline: Value = serde_json::from_str(include_str!("fixtures/generation.json")).unwrap();
    let mut tables = generation_support::tables(&baseline["tables"]);
    tables.effect_palettes = support::arr(&fixture["palettes"], |v| {
        support::arr(v, |v| support::arr(v, |v| v.as_u64().unwrap() as u8))
    });
    let geometry = support::geometry(&fixture["geometry"]);
    for case in fixture["cases"].as_array().unwrap() {
        let flags = case["flags"].as_i64().unwrap() as i32;
        let palette = case["palette"].as_u64().unwrap() as usize;
        let environment = case["environment"]
            .as_bool()
            .unwrap()
            .then_some([33, 66, 99]);
        let mut layers = GenerationLayers::default();
        layers.create(&tables, &geometry, environment, flags, palette);
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
            assert_eq!(
                support::snapshot(&layers),
                update["expected"],
                "flags={flags} eye={}",
                update["eye"]
            );
        }
        layers.stop();
        assert!(layers.environment.is_none());
        assert!(layers.gradient.is_empty());
    }
    for case in fixture["borders"].as_array().unwrap() {
        let commands: Vec<_> =
            generation_border_moves(case["stage"].as_i64().unwrap() as i32, &geometry)
                .iter()
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
}
