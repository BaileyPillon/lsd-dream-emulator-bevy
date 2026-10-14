#[allow(dead_code)]
#[path = "support/link_state.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "support/links.rs"]
mod support;
use runtime::{Effects, array, check, initial, tables};
use serde_json::Value;
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/link-runtime.json")).unwrap()
}
#[test]
fn original_dream_handlers_match_native_state_and_real_angle_position_setters() {
    let fixture = fixture();
    let catalog = support::catalog(&fixture["catalog"]);
    let tables = tables(&fixture);
    for (index, case) in fixture["handlers"].as_array().unwrap().iter().enumerate() {
        let (mut state, mut controls, mut context, mut rng) = initial(&case["initial"]);
        let mut effects = Effects {
            cancel: case["cancel"].as_bool().unwrap(),
            position: array(&case["initial"]["position"], |v| v.as_i64().unwrap() as i32),
            trace: Vec::new(),
        };
        let key = support::location(&case["initial"]["key"]);
        let result = match case["function"].as_str().unwrap() {
            "tunnel" => state.tunnel_at(&mut controls, &catalog, &mut context, key, &mut effects),
            "teleport" => {
                state.teleport_at(&mut controls, &catalog, &mut context, key, &mut effects)
            }
            "stairs" => state.staircase_at(
                &mut controls,
                &catalog,
                &mut context,
                key,
                &tables,
                &mut effects,
            ),
            "dynamic" => state.dynamic_at(
                &mut controls,
                &catalog,
                &mut context,
                key,
                &mut rng,
                &mut effects,
            ),
            _ => unreachable!(),
        }
        .unwrap();
        assert_eq!(result, case["result"].as_bool().unwrap(), "handler {index}");
        check(&state, &controls, &context, &rng, &effects, case);
    }
    assert_eq!(fixture["handlers"].as_array().unwrap().len(), 512);
}

#[test]
fn all_four_original_stair_scripts_match_boundary_ticks_and_completion_sequences() {
    let fixture = fixture();
    let tables = tables(&fixture);
    let catalog = support::catalog(&fixture["catalog"]);
    for case in fixture["auto_ticks"].as_array().unwrap() {
        let (mut state, mut controls, context, rng) = initial(&case["initial"]);
        let mut effects = Effects {
            cancel: false,
            position: array(&case["initial"]["position"], |v| v.as_i64().unwrap() as i32),
            trace: Vec::new(),
        };
        assert_eq!(
            state
                .auto_move_tick(
                    case["action"].as_u64().unwrap() as u8,
                    &mut controls,
                    &tables,
                    &mut effects
                )
                .unwrap(),
            case["result"].as_bool().unwrap()
        );
        check(&state, &controls, &context, &rng, &effects, case);
    }
    let mut states = 0;
    for case in fixture["auto_sequences"].as_array().unwrap() {
        let (mut state, mut controls, mut context, rng) = initial(&case["initial"]);
        let mut effects = Effects {
            cancel: false,
            position: array(&case["initial"]["position"], |v| v.as_i64().unwrap() as i32),
            trace: Vec::new(),
        };
        for expected in case["steps"].as_array().unwrap() {
            effects.trace.clear();
            assert!(
                !state
                    .staircase_at(
                        &mut controls,
                        &catalog,
                        &mut context,
                        support::location(&case["initial"]["key"]),
                        &tables,
                        &mut effects
                    )
                    .unwrap()
            );
            check(&state, &controls, &context, &rng, &effects, expected);
            states += 1;
        }
    }
    assert_eq!(fixture["auto_ticks"].as_array().unwrap().len(), 400);
    assert_eq!(states, 582);
}
