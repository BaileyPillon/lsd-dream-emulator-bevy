use lsd_core::{
    AutoMoveTables, ChunkPlayer, CollisionBounds, FramePhase, LinkCatalog, LinkMotionRules,
    LinkSet, LinkedPlayer, PAD_MASKS, PadState, PlayerControls, PsxMathTables, SceneCollision,
    SceneGrid, SceneLinkTables, SceneSlot, SceneSlots,
};
use serde_json::{Value, json};

fn controls(value: &Value) -> PlayerControls {
    let number = |name: &str| value[name].as_i64().unwrap() as i32;
    let flag = |name: &str| value[name].as_bool().unwrap();
    PlayerControls {
        paused: number("paused"),
        input_locked: flag("input_locked"),
        auto_move_locked: flag("auto_move_locked"),
        auto_move_active: flag("auto_move_active"),
        action_pressed: flag("action_pressed"),
        look_state: number("look_state"),
        look_vertical: number("look_vertical"),
        turn_state: number("turn_state"),
        turn_offset: number("turn_offset"),
        left_right: number("left_right"),
        is_forward: flag("is_forward"),
        move_state: number("move_state"),
        move_direction: number("move_direction"),
        previous_move_direction: number("previous_move_direction"),
        move_step: number("move_step"),
        screen_shake: flag("screen_shake"),
        angles: std::array::from_fn(|i| value["angles"][i].as_i64().unwrap() as i16),
        eye: std::array::from_fn(|i| value["eye"][i].as_i64().unwrap() as i32),
        target: std::array::from_fn(|i| value["target"][i].as_i64().unwrap() as i32),
    }
}
fn tables() -> SceneLinkTables {
    let empty = || LinkSet {
        triggers: std::array::from_fn(|_| vec![]),
        spawns: std::array::from_fn(|_| vec![]),
        trigger_transforms: None,
        spawn_transforms: None,
    };
    SceneLinkTables {
        catalog: LinkCatalog {
            adjustments: vec![],
            random_spawns: std::array::from_fn(|_| vec![]),
            wall: empty(),
            tunnel: empty(),
            teleport: empty(),
            staircase: empty(),
            transforms: vec![],
            facing_angles: vec![],
            violence_key: 0,
        },
        motion: LinkMotionRules {
            distances: [0; 5],
            signs: [0; 5],
        },
        auto_move: AutoMoveTables {
            targets: [[0; 3]; 4],
            clockwise: [0; 3],
            counterclockwise: [0; 3],
        },
    }
}
fn check(
    player: &ChunkPlayer,
    linked: &LinkedPlayer,
    frame: FramePhase,
    expected: &Value,
    label: &str,
) {
    assert_eq!(player.controls, controls(&expected["state"]), "{label}");
    assert_eq!(
        json!([player.pad.held, player.pad.released, player.pad.pressed]),
        expected["pad"],
        "{label}"
    );
    assert_eq!(
        json!([
            linked.links.execution.dream_timer,
            i32::from(player.controls.action_pressed),
            i32::from(linked.can_dynamic),
            frame.phase,
            frame.waiting,
            frame.finished
        ]),
        json!([
            expected["timer"],
            expected["action_flag"],
            expected["can_dynamic"],
            expected["phase"],
            expected["waiting"],
            expected["finished"]
        ]),
        "{label}"
    );
}
#[test]
fn original_display_clock_before_pad_and_next_tick_view_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/frame-pad-order.json")).unwrap();
    let scene = SceneCollision {
        slots: SceneSlots {
            grid: SceneGrid {
                columns: 1,
                rows: 1,
                mode: 0,
            },
            slots: std::array::from_fn(|i| SceneSlot {
                label: i as i16,
                map_index: 0,
                origin: [0; 3],
                ready: i == 0,
            }),
        },
        chunks: std::array::from_fn(|_| None),
    };
    let math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    let mut observations = 0;
    for (case_number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let mut player = ChunkPlayer {
            controls: controls(&case["initial_controls"]),
            pad: PadState::default(),
            position: [0; 3],
            bounds: CollisionBounds {
                minimum: [0; 3],
                maximum: [0; 3],
            },
        };
        let mut linked = LinkedPlayer::new(0, &player, &scene, tables(), 1).unwrap();
        linked.map.move_callback = 0; // Same disabled geometry callback as the owned native graph.
        let mut frame = FramePhase::default();
        check(&player, &linked, frame, &case["initial"], "initial");
        for (step_number, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let label = format!("case {case_number} step {step_number}");
            frame.waiting = step["waiting"].as_i64().unwrap() as i32;
            frame.finished = step["finished"].as_i64().unwrap() as i32;
            if step["display_code"] == 2 {
                let signal = frame.advance();
                linked
                    .tick_clock(&mut player, &scene, &math, [1; 7], None, signal)
                    .unwrap();
                check(&player, &linked, frame, &step["before_pad"], &label);
                LinkedPlayer::poll_pad(&mut player, step["buttons"].as_u64().unwrap() as u32);
                assert_eq!(
                    json!(player.pad.notifications(&PAD_MASKS)),
                    step["expected"]["notifications"],
                    "{label}"
                );
                assert_eq!(
                    step["expected"]["order"],
                    json!([
                        "player_clock",
                        "pad_poll",
                        "firmware_pad_read",
                        "pad_dispatch"
                    ]),
                    "{label}"
                );
            } else {
                assert!(step["before_pad"].is_null());
                assert_eq!(step["expected"]["order"], json!([]));
            }
            check(&player, &linked, frame, &step["expected"], &label);
            observations += 1;
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 120);
    assert_eq!(observations, 3840);
}
