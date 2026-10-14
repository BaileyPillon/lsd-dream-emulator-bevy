use lsd_core::{
    DreamLinks, LinkExecution, LinkPalette, MapLinkCommand, MapLinkPolicy, PlayerControls,
    SpawnLocation,
};
use serde_json::{Value, json};
#[test]
fn native_map_scene_notification_start_and_busy_cancellation_policy_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/map-link-policy.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut links = DreamLinks {
            execution: LinkExecution {
                state: case["event"].as_i64().unwrap() as i32,
                stage: case["stage"].as_i64().unwrap() as i32,
                dream_timer: 0,
                flashback: case["flashback"].as_u64().unwrap() != 0,
            },
            spawn: SpawnLocation::default(),
            source_transform: Some(std::array::from_fn(|i| {
                fixture["source_packet"][i].as_u64().unwrap() as u32
            })),
            target_transform: None,
            auto_action: None,
            auto_step: 0,
            auto_target: SpawnLocation::default(),
            time_limit: -1,
            default_teleport_transform: false,
        };
        let mut controls = PlayerControls {
            move_direction: 4,
            previous_move_direction: 2,
            angles: [0, 1024, 0],
            ..PlayerControls::default()
        };
        let mut policy = MapLinkPolicy {
            state: case["map_state"].as_i64().unwrap() as i32,
            tick: case["tick"].as_i64().unwrap() as i32,
            location: 0,
            move_callback: 1,
            reset_view_pending: false,
            view_active: false,
        };
        let commands = policy
            .notify(
                &mut links,
                &mut controls,
                case["event"].as_i64().unwrap() as i32,
                LinkPalette {
                    dream: 6,
                    flashback: 7,
                },
            )
            .unwrap();
        let mut trace = Vec::new();
        for command in commands {
            match command {
                MapLinkCommand::Fade {
                    color,
                    target,
                    ticks,
                    attach,
                } => {
                    trace.push(json!(["get_effect"]));
                    if ticks != 0 {
                        trace.push(json!(["duration", ticks]));
                    }
                    if attach {
                        trace.push(json!(["attach"]));
                    }
                    trace.push(json!(["start", color, target]));
                }
                MapLinkCommand::Notify(code) => trace.push(json!(["notify", code])),
                MapLinkCommand::MoveCallback(mode) => trace.push(json!(["move_callback", mode])),
            }
        }
        assert_eq!(trace, *case["trace"].as_array().unwrap());
        for (name, actual) in [
            ("state", policy.state),
            ("link_state", links.execution.state),
            ("input_locked", i32::from(controls.input_locked)),
            ("paused", controls.paused),
            ("direction", controls.move_direction),
            ("previous_direction", controls.previous_move_direction),
            ("move_callback", policy.move_callback),
            ("reset_view_pending", i32::from(policy.reset_view_pending)),
            ("view_active", i32::from(policy.view_active)),
        ] {
            assert_eq!(actual, case[name].as_i64().unwrap() as i32, "{name}");
        }
        assert_eq!(
            controls.angles,
            std::array::from_fn(|i| case["angles"][i].as_i64().unwrap() as i16)
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 864);
}
