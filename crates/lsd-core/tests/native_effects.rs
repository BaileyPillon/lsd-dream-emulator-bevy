use lsd_core::{
    EffectEvent, EffectFault, EffectPalette, EffectState, MapCompletionCommand, MapLinkPolicy,
    PlayerControls,
};
use serde_json::{Value, json};

fn palette(fixture: &Value) -> EffectPalette {
    let colors = |name: &str| {
        std::array::from_fn(|i| {
            std::array::from_fn(|j| fixture["palette"][name][i][j].as_u64().unwrap() as u8)
        })
    };
    EffectPalette {
        start: colors("start"),
        attached: colors("attached"),
    }
}
fn state(value: &Value) -> EffectState {
    let number = |name: &str| value[name].as_i64().unwrap() as i32;
    EffectState {
        render_flags: value["render_flags"].as_u64().unwrap() as u32,
        color: std::array::from_fn(|i| value["color"][i].as_u64().unwrap() as u8),
        color_mask: number("color_mask"),
        state: number("state"),
        variant: number("variant"),
        color_step: number("color_step"),
        channels: number("channels"),
        mode: number("mode"),
        life: number("life"),
        mask_step: number("mask_step"),
        linked: number("linked"),
        linked_divisor: number("linked_divisor"),
    }
}
fn trace(events: Vec<EffectEvent>) -> Vec<Value> {
    events
        .into_iter()
        .map(|event| match event {
            EffectEvent::Attach => json!(["effect_attach"]),
            EffectEvent::Detach => json!(["effect_detach"]),
            EffectEvent::Notify(code) => json!(["effect_notify", code]),
        })
        .collect()
}
fn check(actual: &EffectState, events: Vec<EffectEvent>, expected: &Value) {
    assert_eq!(actual, &state(&expected["state"]));
    assert_eq!(trace(events), *expected["trace"].as_array().unwrap());
}

#[test]
fn original_effect_activation_and_all_ticks_match_owned_palette_sequences() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/effect-lifecycle.json")).unwrap();
    let palette = palette(&fixture);
    let mut observations = 0;
    for case in fixture["sequences"].as_array().unwrap() {
        let mut effect = state(&case["initial"]);
        let output = case["observations"].as_array().unwrap();
        let events = effect
            .start(
                &palette,
                case["attached"].as_bool().unwrap(),
                case["variant"].as_i64().unwrap() as i32,
                case["mode"].as_i64().unwrap() as i32,
            )
            .unwrap();
        check(&effect, events, &output[0]);
        for expected in &output[1..] {
            let events = effect.tick(&palette, 2);
            check(&effect, events, expected);
        }
        observations += output.len();
    }
    assert_eq!(fixture["sequences"].as_array().unwrap().len(), 128);
    assert_eq!(observations, 3600);
}

#[test]
fn original_effect_inactive_wrapping_reset_and_deactivation_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/effect-lifecycle.json")).unwrap();
    let palette = palette(&fixture);
    for case in fixture["ticks"].as_array().unwrap() {
        let mut effect = state(&case["initial"]);
        let events = effect.tick(&palette, case["code"].as_i64().unwrap() as i32);
        check(&effect, events, &case["output"]);
    }
    for case in fixture["deactivations"].as_array().unwrap() {
        let mut effect = state(&case["initial"]);
        let events = effect.deactivate(&palette);
        check(&effect, events, &case["output"]);
    }
    for case in fixture["resets"].as_array().unwrap() {
        let mut effect = state(&case["initial"]);
        effect.reset(case["variant"].as_i64().unwrap() as i32);
        check(&effect, Vec::new(), &case["output"]);
    }
}

#[test]
fn original_map_completion_order_and_view_restoration_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/effect-lifecycle.json")).unwrap();
    let palette = palette(&fixture);
    for case in fixture["completions"].as_array().unwrap() {
        let mut effect = state(&fixture["sequences"][0]["initial"]);
        effect.channels = case["channels"].as_i64().unwrap() as i32;
        let mut policy = MapLinkPolicy {
            state: case["map_state"].as_i64().unwrap() as i32,
            tick: 0,
            location: 0,
            move_callback: 1,
            reset_view_pending: true,
            view_active: false,
        };
        let mut controls = PlayerControls {
            paused: 2,
            move_direction: 4,
            previous_move_direction: 2,
            ..PlayerControls::default()
        };
        let commands = policy
            .complete_effect(
                &mut controls,
                &effect,
                &palette,
                case["code"].as_i64().unwrap() as i32,
            )
            .unwrap();
        let output = &case["output"];
        assert_eq!(policy.state, output["state"].as_i64().unwrap() as i32);
        assert_eq!(controls.paused, output["paused"].as_i64().unwrap() as i32);
        assert_eq!(
            controls.move_direction,
            output["direction"].as_i64().unwrap() as i32
        );
        assert_eq!(
            controls.previous_move_direction,
            output["previous_direction"].as_i64().unwrap() as i32
        );
        assert_eq!(
            policy.reset_view_pending,
            output["reset_view_pending"].as_i64().unwrap() != 0
        );
        assert_eq!(
            policy.view_active,
            output["view_active"].as_i64().unwrap() != 0
        );
        let actual: Vec<_> = commands
            .into_iter()
            .map(|command| match command {
                MapCompletionCommand::DetachEffect => json!(["scene_detach"]),
                MapCompletionCommand::SetStageColor(color) => json!(["stage_color", color]),
                MapCompletionCommand::StopEntitySound => json!(["sound_stop"]),
                MapCompletionCommand::Notify(code) => json!(["scene_notify", code]),
            })
            .collect();
        assert_eq!(actual, *output["trace"].as_array().unwrap());
    }
    assert_eq!(fixture["completions"].as_array().unwrap().len(), 84);
}

#[test]
fn unsupported_effect_divisions_and_palette_indices_are_reported() {
    let palette = EffectPalette {
        start: [[0; 3]; 8],
        attached: [[0; 3]; 8],
    };
    for (step, linked, divisor) in [(0, 0, 0), (300, 0, 0), (10, 1, 0), (10, 1, 1)] {
        let mut effect = EffectState::new([0; 3], 31, 0);
        effect.color_step = step;
        effect.linked = linked;
        effect.linked_divisor = divisor;
        assert_eq!(
            effect.start(&palette, true, 0, 0),
            Err(EffectFault::InvalidDivision)
        );
    }
    let mut effect = EffectState::new([0; 3], 31, 0);
    assert_eq!(
        effect.start(&palette, true, 8, 0),
        Err(EffectFault::InvalidPaletteIndex)
    );
}
