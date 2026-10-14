#[path = "../../lsd-core/tests/support/generation.rs"]
mod support;
use lsd_assets::{load_generation_paths, load_generation_tables};
use lsd_core::{DreamGeneration, PsxRng, choose_music, choose_texture};
use serde_json::{Value, json};
use std::path::PathBuf;
use support::*;

#[test]
#[ignore = "Requires a canonical local executable and private original CPU observations"]
fn original_generation_content_and_all_calendar_selections_match() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_GENERATION_CASES")
                .expect("Set LSD_NATIVE_GENERATION_CASES"),
        )
        .unwrap(),
    )
    .unwrap();
    let loaded = load_generation_tables(&executable).unwrap();
    assert_eq!(loaded, tables(&fixture["tables"]));
    let paths = load_generation_paths(&executable).unwrap();
    assert_eq!(
        json!({"textures":paths.textures,"music":paths.music}),
        fixture["path_tables"]
    );
    for case in fixture["cases"].as_array().unwrap() {
        let mut generation = DreamGeneration {
            index: 77,
            effect_data_color: loaded.palette[0],
            ..Default::default()
        };
        generation
            .start(&loaded, num(&case["stage"]), num(&case["day"]), 0)
            .unwrap();
        let style = generation.style.unwrap();
        let actual = json!({"owner":generation.owner_active,"location":generation.location,"day":generation.day,
            "submode":generation.submode,"timer":generation.timer,"kind":generation.generation_type,
            "index":generation.index,"flags":generation.flags,"palette":generation.effect_palette,
            "data_color":generation.effect_data_color,"environment":generation.environment_effect,
            "style":{"kind":style.kind,"height":style.height,"scene_color":style.scene_color,"environment_color":style.environment_color}});
        assert_eq!(
            actual, case["expected"],
            "stage={} day={}",
            case["stage"], case["day"]
        );
    }
    let mut selected_paths = std::collections::BTreeSet::new();
    for case in fixture["paths"].as_array().unwrap() {
        let mut rng = PsxRng::new(num(&case["seed"]) as u32);
        let mut override_track = 0;
        let music = choose_music(num(&case["stage"]), &mut override_track, &mut rng);
        let texture = choose_texture(num(&case["day"]), &mut rng).unwrap();
        assert_eq!(music, num(&case["music"]) as usize);
        assert_eq!(texture, num(&case["texture"]) as usize);
        assert_eq!(rng.state(), num(&case["rng"]) as u32);
        let stage = num(&case["stage"]) as usize;
        selected_paths.insert(&paths.textures[stage][texture]);
        selected_paths.insert(&paths.music[stage][music]);
    }
    for path in selected_paths {
        assert!(files.join("CDI").join(path).is_file());
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 5110);
    assert_eq!(fixture["paths"].as_array().unwrap().len(), 10220);
}
