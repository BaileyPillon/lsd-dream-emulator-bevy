#[path = "support/day_runtime.rs"]
mod day_runtime;
#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use lsd_assets::EncounterEventHost;
use lsd_core::{PlayerControls, PsxMathTables, SceneGrid};
use serde_json::{Value, json};
fn array<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
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
        angles: array::<3>(&value["angles"]).map(|v| v as i16),
        eye: array(&value["eye"]),
        target: array(&value["target"]),
        screen_shake: flag("screen_shake"),
    }
}
#[test]
fn restored_callbacks_match_original_cleanup_and_subsequent_player_clock() {
    use lsd_core::{
        AutoMoveTables, ChunkPlayer, CollisionBounds, FrameSignal, LinkMotionRules, LinkedPlayer,
        PadState, SceneCollision, SceneLinkTables, SceneSlot, SceneSlots,
    };
    let value: Value =
        serde_json::from_str(include_str!("fixtures/restored-player-callbacks.json")).unwrap();
    let catalog_fixture: Value =
        serde_json::from_str(include_str!("../../lsd-core/tests/fixtures/links.json")).unwrap();
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
        chunks: std::array::from_fn(|i| {
            (i == 0).then(|| lsd_core::CollisionChunk {
                policy: Default::default(),
                models: vec![],
                cells: vec![vec![]; 400],
            })
        }),
    };
    let math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    for case in value["cases"].as_array().unwrap() {
        let mut player = ChunkPlayer {
            controls: controls(&case["initial"]["state"]),
            pad: PadState::default(),
            position: [0; 3],
            bounds: CollisionBounds {
                minimum: [0; 3],
                maximum: [0; 3],
            },
        };
        let mut linked = LinkedPlayer::new(
            0,
            &player,
            &scene,
            SceneLinkTables {
                catalog: support::catalog(&catalog_fixture["catalog"]),
                motion: LinkMotionRules {
                    distances: [0; 5],
                    signs: [0; 5],
                },
                auto_move: AutoMoveTables {
                    targets: [[0; 3]; 4],
                    clockwise: [0; 3],
                    counterclockwise: [0; 3],
                },
            },
            1,
        )
        .unwrap();
        let mut runtime = day_runtime::DayRuntime::new([0; 9]);
        let before = &case["before_callbacks"];
        runtime.move_callback = before[0].as_i64().unwrap() as i32;
        runtime.view_callback = before[1].as_i64().unwrap() as i32;
        runtime.reset_view_pending = before[4].as_bool().unwrap();
        runtime.view_active = before[5].as_bool().unwrap();
        runtime
            .dispatch(&math, |dispatcher| dispatcher.restore_player_callbacks())
            .unwrap();
        assert_eq!(
            json!([
                runtime.move_callback,
                runtime.view_callback,
                runtime.move_callback != 0,
                runtime.view_callback != 0,
                runtime.reset_view_pending,
                runtime.view_active
            ]),
            case["callbacks"]
        );
        linked.map.move_callback = runtime.move_callback;
        linked.view_callback = runtime.view_callback;
        linked.map.reset_view_pending = runtime.reset_view_pending;
        linked.map.view_active = runtime.view_active;
        linked.links.execution.dream_timer = case["initial"]["timer"].as_u64().unwrap() as u32;
        linked.can_dynamic = case["initial"]["can_dynamic"].as_u64().unwrap() != 0;
        for expected in case["steps"].as_array().unwrap() {
            linked
                .tick_clock(&mut player, &scene, &math, [1; 7], None, FrameSignal::Tick)
                .unwrap();
            assert_eq!(player.controls, controls(&expected["state"]), "{case}");
            assert_eq!(
                u64::from(linked.links.execution.dream_timer),
                expected["timer"].as_u64().unwrap()
            );
            assert_eq!(
                u64::from(linked.can_dynamic),
                expected["can_dynamic"].as_u64().unwrap()
            );
        }
    }
}
