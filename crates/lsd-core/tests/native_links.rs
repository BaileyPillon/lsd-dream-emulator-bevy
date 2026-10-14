#[path = "support/links.rs"]
mod support;
use lsd_core::{
    LinkActionHost, LinkContext, LinkExecution, LinkKind, LinkMotionRules, MotionAxis,
    MotionRequest, SpawnLocation, is_facing_link_angle, trigger_link_action,
};
use serde_json::{Value, json};
use std::cell::RefCell;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/links.json")).unwrap()
}

#[test]
fn static_link_matching_and_wrapper_gates_match_original_cpu() {
    let fixture = fixture();
    support::static_cases(
        &support::catalog(&fixture["catalog"]),
        fixture["static"].as_array().unwrap(),
    );
    assert_eq!(fixture["static"].as_array().unwrap().len(), 350);
}

#[test]
fn random_and_dynamic_spawns_preserve_original_rng_and_side_effects() {
    let fixture = fixture();
    let catalog = support::catalog(&fixture["catalog"]);
    support::spawn_cases(&catalog, fixture["random"].as_array().unwrap(), false);
    support::spawn_cases(&catalog, fixture["dynamic"].as_array().unwrap(), true);
    assert_eq!(fixture["random"].as_array().unwrap().len(), 102);
    assert_eq!(fixture["dynamic"].as_array().unwrap().len(), 336);
}

#[test]
fn facing_limits_transform_selection_and_stair_action_match_original_cpu() {
    let fixture = fixture();
    let catalog = support::catalog(&fixture["catalog"]);
    for case in fixture["facing"].as_array().unwrap() {
        assert_eq!(
            is_facing_link_angle(
                case["yaw"].as_i64().unwrap() as i16,
                case["angle"].as_u64().unwrap() as u16
            ),
            case["result"].as_bool().unwrap()
        );
    }
    for case in fixture["transforms"].as_array().unwrap() {
        let kind = support::kind(&case["kind"]);
        let context = LinkContext {
            source_stage: case["globals"][0].as_i64().unwrap() as i32,
            source_index: case["globals"][1].as_i64().unwrap() as i32,
            target_stage: case["globals"][2].as_i64().unwrap() as i32,
            target_index: case["globals"][3].as_i64().unwrap() as i32,
            ..LinkContext::default()
        };
        let actual = catalog
            .facing_link(kind, &context, case["yaw"].as_i64().unwrap() as i16)
            .unwrap();
        if case["result"].is_null() {
            assert_eq!(actual, None);
        } else {
            let actual = actual.unwrap();
            assert_eq!(
                actual.target,
                std::array::from_fn(|i| case["result"][0][i].as_u64().unwrap() as u32)
            );
            assert_eq!(
                actual.source,
                std::array::from_fn(|i| case["result"][1][i].as_u64().unwrap() as u32)
            );
        }
        if kind == LinkKind::Staircase {
            assert_eq!(
                catalog.staircase_action(&context).unwrap(),
                case["auto_action"].as_i64().unwrap() as i8
            );
        }
    }
}

#[test]
fn transition_notification_cancellation_timer_and_sound_order_match_original_cpu() {
    let fixture = fixture();
    for case in fixture["execute"].as_array().unwrap() {
        let mut state = LinkExecution {
            state: 7,
            stage: 3,
            dream_timer: 0xfedcba98,
            flashback: case["flashback"].as_bool().unwrap(),
        };
        let trace = RefCell::new(Vec::<Value>::new());
        let result = state.execute(
            12,
            17,
            case["play_sound"].as_bool().unwrap(),
            |state, event| {
                assert_eq!(state.state, 17);
                trace.borrow_mut().push(json!(["notify", event]));
                state.state = case["notify_state"].as_i64().unwrap() as i32;
                state.flashback = case["after_flashback"].as_bool().unwrap();
            },
            |state, parameters| {
                assert_eq!(state.stage, 12);
                assert_eq!(
                    state.dream_timer,
                    if state.flashback { 0 } else { 0xfedcba98 }
                );
                trace.borrow_mut().push(json!([
                    "sound",
                    parameters[0],
                    parameters[1],
                    parameters[2]
                ]));
            },
        );
        assert_eq!(result, case["result"].as_bool().unwrap());
        assert_eq!(trace.into_inner(), *case["trace"].as_array().unwrap());
        assert_eq!(state.state, case["state"].as_i64().unwrap() as i32);
        assert_eq!(state.stage, case["stage"].as_i64().unwrap() as i32);
        assert_eq!(state.dream_timer, case["timer"].as_u64().unwrap() as u32);
    }
}

struct Host<'a> {
    case: &'a Value,
    trace: Vec<Value>,
    direction: i32,
    active: bool,
    stage: i32,
    position: [i32; 3],
    marker: i32,
}
impl LinkActionHost for Host<'_> {
    fn move_direction(&self) -> i32 {
        self.direction
    }
    fn auto_move_active(&self) -> bool {
        self.active
    }
    fn stage(&self) -> i32 {
        self.stage
    }
    fn position(&self) -> [i32; 3] {
        self.position
    }
    fn hook(&mut self) {
        self.trace.push(json!(["hook"]));
        self.direction = self.case["after_direction"].as_i64().unwrap() as i32;
    }
    fn spawn_key(&mut self) -> SpawnLocation {
        self.trace.push(json!(["key"]));
        SpawnLocation {
            chunk: 257,
            tile: 513,
            position: [1, 2, 3],
        }
    }
    fn staircase(&mut self, key: SpawnLocation) -> i32 {
        assert_eq!(
            key,
            SpawnLocation {
                chunk: 257,
                tile: 513,
                position: [1, 2, 3]
            }
        );
        self.trace.push(json!(["staircase"]));
        self.active = self.case["stairs_auto"].as_i64().unwrap() != 0;
        self.case["staircase"].as_i64().unwrap() as i32
    }
    fn teleport(&mut self, _key: SpawnLocation) -> i32 {
        self.trace.push(json!(["teleport"]));
        self.case["teleport"].as_i64().unwrap() as i32
    }
    fn tunnel(&mut self, _key: SpawnLocation) -> i32 {
        self.trace.push(json!(["tunnel"]));
        self.case["tunnel"].as_i64().unwrap() as i32
    }
    fn save_link_state(&mut self) {
        self.trace.push(json!(["save"]));
    }
    fn apply_motion(&mut self, request: MotionRequest) {
        self.trace.push(json!([
            if request.axis == MotionAxis::Forward {
                "forward"
            } else {
                "strafe"
            },
            request.amount,
            i32::from(request.collision_and_links)
        ]));
        self.stage = self.case["after_stage"].as_i64().unwrap() as i32;
        self.position = std::array::from_fn(|i| self.case["position"][i].as_i64().unwrap() as i32);
    }
    fn wall_link(&mut self, code: i32) {
        self.trace.push(json!(["wall", code]));
    }
    fn clear_transform_marker(&mut self) {
        self.marker = 0;
    }
}

#[test]
fn trigger_precedence_and_live_callback_mutations_match_original_cpu_boundaries() {
    let fixture = fixture();
    let rules = LinkMotionRules {
        distances: std::array::from_fn(|i| {
            fixture["motion_rules"]["distances"][i].as_i64().unwrap() as i32
        }),
        signs: std::array::from_fn(|i| fixture["motion_rules"]["signs"][i].as_i64().unwrap() as i8),
    };
    for (index, case) in fixture["trigger"].as_array().unwrap().iter().enumerate() {
        let mut host = Host {
            case,
            trace: Vec::new(),
            direction: case["direction"].as_i64().unwrap() as i32,
            active: false,
            stage: 3,
            position: [0; 3],
            marker: 99,
        };
        trigger_link_action(&mut host, &rules, case["action"].as_i64().unwrap() as i32).unwrap();
        assert_eq!(
            host.trace,
            *case["trace"].as_array().unwrap(),
            "trigger trace {index}"
        );
        assert_eq!(
            host.marker,
            case["marker"].as_i64().unwrap() as i32,
            "marker {index}"
        );
    }
    assert_eq!(fixture["trigger"].as_array().unwrap().len(), 400);
}
