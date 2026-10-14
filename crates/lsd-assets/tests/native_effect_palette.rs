use lsd_assets::load_effect_palette;
use lsd_core::{EffectEvent, EffectState};
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
#[ignore = "Requires the local original executable and private native effect observations"]
fn imported_original_effect_colors_reproduce_private_native_states() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let input = PathBuf::from(
        std::env::var_os("LSD_NATIVE_EFFECT_CASES").expect("Set LSD_NATIVE_EFFECT_CASES"),
    );
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let palette = load_effect_palette(&executable).unwrap();
    let fixture: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    assert_eq!(json!(palette.start), fixture["palette"]["start"]);
    assert_eq!(json!(palette.attached), fixture["palette"]["attached"]);
    let mut count = 0;
    for case in fixture["sequences"].as_array().unwrap() {
        let initial = &case["initial"];
        let mut state = EffectState::new(
            palette.attached[0],
            initial["color_mask"].as_i64().unwrap() as i32,
            initial["variant"].as_i64().unwrap() as i32,
        );
        state.render_flags = initial["render_flags"].as_u64().unwrap() as u32;
        state.color_step = case["step"].as_i64().unwrap() as i32;
        let mut events = state
            .start(&palette, true, case["variant"].as_i64().unwrap() as i32, 0)
            .unwrap();
        for expected in case["observations"].as_array().unwrap() {
            let output = &expected["state"];
            for (name, actual) in [
                ("state", state.state),
                ("variant", state.variant),
                ("color_step", state.color_step),
                ("channels", state.channels),
                ("mode", state.mode),
                ("life", state.life),
                ("mask_step", state.mask_step),
            ] {
                assert_eq!(actual, output[name].as_i64().unwrap() as i32, "{name}");
            }
            assert_eq!(
                state.render_flags,
                output["render_flags"].as_u64().unwrap() as u32
            );
            assert_eq!(json!(state.color), output["color"]);
            let trace: Vec<_> = events
                .into_iter()
                .map(|event| match event {
                    EffectEvent::Attach => json!(["effect_attach"]),
                    EffectEvent::Detach => json!(["effect_detach"]),
                    EffectEvent::Notify(code) => json!(["effect_notify", code]),
                })
                .collect();
            assert_eq!(trace, *expected["trace"].as_array().unwrap());
            count += 1;
            events = state.tick(&palette, 2);
        }
    }
    assert_eq!(fixture["sequences"].as_array().unwrap().len(), 16);
    assert_eq!(count, 536);
}
