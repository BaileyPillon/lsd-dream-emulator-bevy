use lsd_core::{
    DreamLinks, LinkExecution, MapLinkPolicy, PlayerControls, SpawnLocation, reset_scene_motion,
};
use serde_json::{Value, json};

#[test]
fn original_link_scene_motion_reset_matches_fields_and_rotation_packet() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/scene-motion-reset.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let initial = &case["initial"];
        let mut controls = PlayerControls {
            paused: 2,
            input_locked: true,
            action_pressed: true,
            auto_move_locked: true,
            auto_move_active: true,
            look_state: 1,
            look_vertical: 1,
            turn_state: 1,
            turn_offset: 1,
            left_right: 1,
            is_forward: true,
            move_state: 1,
            move_direction: 4,
            previous_move_direction: 2,
            move_step: 1,
            angles: std::array::from_fn(|i| initial["angles"][i].as_i64().unwrap() as i16),
            ..PlayerControls::default()
        };
        let key = SpawnLocation {
            chunk: 257,
            tile: 513,
            position: [11, 22, 33],
        };
        let mut links = DreamLinks {
            execution: LinkExecution {
                state: 13,
                stage: 3,
                dream_timer: 12345,
                flashback: false,
            },
            spawn: key,
            source_transform: None,
            target_transform: None,
            auto_action: Some(2),
            auto_step: 37,
            auto_target: key,
            time_limit: -1,
            default_teleport_transform: false,
        };
        let mut map = MapLinkPolicy {
            state: 5,
            tick: 0,
            location: 3,
            move_callback: initial["move_callback"].as_i64().unwrap() as i32,
            reset_view_pending: true,
            view_active: true,
        };
        let reset = reset_scene_motion(
            &mut controls,
            &mut links,
            &mut map,
            case["direction"].as_i64().unwrap() as i32,
            case["interval"].as_u64().unwrap() as u32,
        )
        .unwrap();
        let output = &case["output"];
        for (name, actual) in [
            ("link_state", links.execution.state),
            ("paused", controls.paused),
            ("input_locked", i32::from(controls.input_locked)),
            ("action_pressed", i32::from(controls.action_pressed)),
            ("view_callback", reset.view_callback),
            ("look_state", controls.look_state),
            ("look_vertical", controls.look_vertical),
            ("turn_state", controls.turn_state),
            ("turn_offset", controls.turn_offset),
            ("move_callback", map.move_callback),
            ("move_state", controls.move_state),
            ("left_right", controls.left_right),
            ("is_forward", i32::from(controls.is_forward)),
            ("direction", controls.move_direction),
            ("previous_direction", controls.previous_move_direction),
            ("move_step", controls.move_step),
            ("floor_surface", reset.floor_surface),
            ("floor_effect", reset.floor_effect),
            ("reset_view_pending", i32::from(map.reset_view_pending)),
            ("view_active", i32::from(map.view_active)),
            ("interval", reset.interval as i32),
            ("can_dynamic", reset.can_dynamic),
            ("param0", reset.link_parameters[0]),
            ("param1", reset.link_parameters[1]),
            ("param2", reset.link_parameters[2]),
            ("reset_flag", reset.reset_flag),
            ("timer", links.execution.dream_timer as i32),
            ("auto_locked", i32::from(controls.auto_move_locked)),
            ("auto_active", i32::from(controls.auto_move_active)),
            ("auto_step", links.auto_step),
        ] {
            assert_eq!(actual, output[name].as_i64().unwrap() as i32, "{name}");
        }
        assert_eq!(json!(controls.angles), output["angles"]);
        assert_eq!(
            reset.cinematic_variant,
            output["cinematic"][1].as_i64().unwrap() as i16
        );
        assert!(links.auto_action.is_none() && output["auto_callback_cleared"].as_bool().unwrap());
        assert_eq!(
            json!([
                ["mood", [key.chunk, key.tile, 11, 22, 33]],
                ["rotate", true, reset.rotation_packet]
            ]),
            output["trace"]
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 160);
}
