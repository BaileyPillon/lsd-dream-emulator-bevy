use lsd_core::{
    CollisionFace, FloorEffectStep, MotionAxis, PAD_MASKS, PadState, PlayerControls, PsxMathTables,
    intersect_model,
};
use serde_json::{Value, json};

fn array<const N: usize, T>(value: &Value, convert: impl Fn(i64) -> T) -> [T; N] {
    std::array::from_fn(|i| convert(value[i].as_i64().unwrap()))
}

fn controls(value: &Value) -> PlayerControls {
    PlayerControls {
        paused: value["paused"].as_i64().unwrap() as i32,
        input_locked: value["input_locked"].as_bool().unwrap(),
        auto_move_locked: value["auto_move_locked"].as_bool().unwrap(),
        auto_move_active: value["auto_move_active"].as_bool().unwrap(),
        action_pressed: value["action_pressed"].as_bool().unwrap(),
        look_state: value["look_state"].as_i64().unwrap() as i32,
        look_vertical: value["look_vertical"].as_i64().unwrap() as i32,
        turn_state: value["turn_state"].as_i64().unwrap() as i32,
        turn_offset: value["turn_offset"].as_i64().unwrap() as i32,
        left_right: value["left_right"].as_i64().unwrap() as i32,
        is_forward: value["is_forward"].as_bool().unwrap(),
        move_state: value["move_state"].as_i64().unwrap() as i32,
        move_direction: value["move_direction"].as_i64().unwrap() as i32,
        previous_move_direction: value["previous_move_direction"].as_i64().unwrap() as i32,
        move_step: value["move_step"].as_i64().unwrap() as i32,
        screen_shake: value["screen_shake"].as_bool().unwrap(),
        angles: array(&value["angles"], |v| v as i16),
        eye: array(&value["eye"], |v| v as i32),
        target: array(&value["target"], |v| v as i32),
    }
}

fn snapshot(c: &PlayerControls) -> Value {
    json!({"paused":c.paused,"input_locked":c.input_locked,"auto_move_locked":c.auto_move_locked,
        "auto_move_active":c.auto_move_active,"action_pressed":c.action_pressed,"look_state":c.look_state,
        "look_vertical":c.look_vertical,"turn_state":c.turn_state,"turn_offset":c.turn_offset,
        "left_right":c.left_right,"is_forward":c.is_forward,"move_state":c.move_state,"move_direction":c.move_direction,
        "previous_move_direction":c.previous_move_direction,"move_step":c.move_step,"screen_shake":c.screen_shake,
        "angles":c.angles,"eye":c.eye,"target":c.target})
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/player-collision.json")).unwrap()
}

#[test]
fn original_pad_notification_view_and_movement_sequences_match() {
    let fixture = fixture();
    let mut observations = 0;
    for (index, case) in fixture["controls"].as_array().unwrap().iter().enumerate() {
        let mut controls = controls(&case["initial"]);
        let mut pad = PadState::default();
        for (step, data) in case["steps"].as_array().unwrap().iter().enumerate() {
            pad.poll(data["buttons"].as_u64().unwrap() as u32);
            assert_eq!(json!([pad.held, pad.released, pad.pressed]), data["pad"]);
            let notifications = pad.notifications(&PAD_MASKS);
            assert_eq!(json!(notifications), data["notifications"]);
            for notification in notifications {
                controls.notify_pad(notification);
            }
            controls.update_view();
            let movement = controls.update_movement();
            let mut floor = Vec::new();
            let mut motion = Vec::new();
            if let Some(movement) = movement {
                floor.push(match movement.floor_effect {
                    FloorEffectStep::Apply => "apply",
                    FloorEffectStep::Release => "release",
                });
                if let Some(request) = movement.motion {
                    motion.push(json!({"axis":match request.axis { MotionAxis::Forward => "forward",MotionAxis::Strafe => "strafe" },
                        "amount":request.amount,"collision_and_links":request.collision_and_links}));
                }
            }
            assert_eq!(json!(floor), data["floor"], "case {index} step {step}");
            assert_eq!(json!(motion), data["motion"], "case {index} step {step}");
            assert_eq!(
                snapshot(&controls),
                data["state"],
                "case {index} step {step}"
            );
            observations += 1;
        }
    }
    assert_eq!(observations, 960);
}

fn sparse_tables(fixture: &Value) -> PsxMathTables {
    let mut table = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    for case in fixture["rotations"].as_array().unwrap() {
        for input in case["table_inputs"].as_array().unwrap() {
            table.sin_cos[input[0].as_u64().unwrap() as usize] = input[1].as_u64().unwrap() as u32;
        }
    }
    for (index, value) in table.sqrt_window[320..].iter_mut().enumerate() {
        *value = (((index + 64) as f64).sqrt() * 512.0) as i16;
    }
    for case in fixture["square_roots"].as_array().unwrap() {
        table.sqrt_window[case["lookup_index"].as_u64().unwrap() as usize] =
            case["lookup_entry"].as_i64().unwrap() as i16;
    }
    table
}

#[test]
fn integer_rotation_and_sdk_square_root_match_original_cpu_observations() {
    let fixture = fixture();
    let tables = sparse_tables(&fixture);
    for case in fixture["rotations"].as_array().unwrap() {
        let angles = array(&case["angles"], |v| v as i16);
        let matrix: Vec<_> = tables.rotation(angles).0.into_iter().flatten().collect();
        assert_eq!(json!(matrix), case["matrix"], "{angles:?}");
    }
    for case in fixture["square_roots"].as_array().unwrap() {
        assert_eq!(
            tables.square_root(case["value"].as_i64().unwrap() as i32),
            case["result"].as_u64().unwrap() as u32,
            "{case}"
        );
    }
}

#[test]
fn face_queries_match_original_cpu_with_semantic_gte_observations() {
    let fixture = fixture();
    let tables = sparse_tables(&fixture);
    for (index, case) in fixture["collisions"].as_array().unwrap().iter().enumerate() {
        let faces: Vec<_> = case["faces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|face| {
                let points: Vec<_> = face
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|point| array(point, |v| v as i16))
                    .collect();
                match points.len() {
                    3 => CollisionFace::Triangle(points.try_into().unwrap()),
                    4 => CollisionFace::Quad(points.try_into().unwrap()),
                    _ => panic!("bad face"),
                }
            })
            .collect();
        let hit = intersect_model(
            &faces,
            array(&case["start"], |v| v as i16),
            array(&case["end"], |v| v as i16),
            &tables,
        )
        .unwrap();
        assert_eq!(
            hit.is_some(),
            case["hit"].as_bool().unwrap(),
            "case {index}"
        );
        if let Some(hit) = hit {
            assert_eq!(
                hit.distance,
                case["distance"].as_i64().unwrap() as i32,
                "case {index}"
            );
            assert_eq!(json!(hit.position), case["position"], "case {index}");
            assert_eq!(
                hit.height_from_face_minimum,
                case["height"].as_i64().unwrap() as i32,
                "case {index}"
            );
        }
    }
}
