use lsd_core::{MapRequest, SceneGrid, StreamEvent, StreamProtocol, StreamSlotState};
use serde_json::Value;

fn protocol(value: &Value) -> StreamProtocol {
    StreamProtocol {
        activity: value["activity"].as_i64().unwrap() as i32,
        pending: value["pending"].as_u64().unwrap() as u16,
        finalized: value["finalized"].as_i64().unwrap() as i32,
        slots: std::array::from_fn(|index| {
            let slot = &value["slots"][index];
            StreamSlotState {
                label: slot["label"].as_u64().unwrap() as u16,
                flag: slot["flag"].as_u64().unwrap() as u16,
                busy: slot["busy"].as_u64().unwrap() as u16,
                ready: slot["ready"].as_i64().unwrap() as i16,
                completed: slot["completed"].as_u64().unwrap() as u16,
                map_index: slot["map"].as_u64().unwrap() as u16,
            }
        }),
    }
}
fn event(value: &Value) -> StreamEvent {
    let slot = value["slot"].as_u64().unwrap() as usize;
    match value["kind"].as_str().unwrap() {
        "notify" => StreamEvent::Notify {
            slot,
            code: value["code"].as_u64().unwrap() as u32,
            index: value["index"].as_u64().unwrap() as usize,
        },
        "release" => StreamEvent::Release { slot },
        "load" => StreamEvent::Load {
            slot,
            map: value["map"].as_i64().unwrap() as i32,
        },
        "cancel" => StreamEvent::Cancel { slot },
        "bind" => StreamEvent::Bind { slot },
        _ => panic!("invalid event"),
    }
}
#[test]
fn boundary_masks_match_original_cpu() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/stream-masks.json")).unwrap();
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let grid = SceneGrid {
            columns: case["layout"][0].as_i64().unwrap() as i16,
            rows: case["layout"][1].as_i64().unwrap() as i16,
            mode: case["layout"][2].as_i64().unwrap() as i32,
        };
        assert_eq!(
            grid.boundary_mask(
                case["index"].as_i64().unwrap() as i32,
                case["parity"].as_i64().unwrap() as i32
            )
            .unwrap(),
            case["mask"].as_u64().unwrap() as u32,
            "mask {index}"
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 309);
}
#[test]
fn request_and_completion_states_match_native_with_captured_callbacks() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/stream-protocol.json")).unwrap();
    let mut operations = 0;
    for (case_index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let mut state = protocol(case);
        for (step, operation) in case["operations"].as_array().unwrap().iter().enumerate() {
            let events = if operation["kind"] == "request" {
                let requests: Vec<_> = operation["requests"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|request| MapRequest {
                        label: request["label"].as_u64().unwrap() as u8,
                        map_index: request["map"].as_i64().map(|v| v as i32),
                    })
                    .collect();
                state.request(&requests).unwrap()
            } else {
                for update in operation["updates"].as_array().unwrap() {
                    let slot = &mut state.slots[update["slot"].as_u64().unwrap() as usize];
                    slot.busy = update["busy"].as_u64().unwrap() as u16;
                    slot.ready = update["ready"].as_i64().unwrap() as i16;
                    slot.completed = update["completed"].as_u64().unwrap() as u16;
                }
                state.tick(operation["code"].as_i64().unwrap() as i32)
            };
            let expected: Vec<_> = operation["events"]
                .as_array()
                .unwrap()
                .iter()
                .map(event)
                .collect();
            assert_eq!(events, expected, "case {case_index} step {step}");
            assert_eq!(
                state,
                protocol(&operation["expected"]),
                "case {case_index} step {step}"
            );
            operations += 1;
        }
    }
    assert_eq!(operations, 128);
}
