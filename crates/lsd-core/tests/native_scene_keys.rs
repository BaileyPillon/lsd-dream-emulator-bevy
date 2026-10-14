use lsd_core::{SceneActorKeys, SceneGrid, SceneSlot, SceneSlots, SpawnLocation};
use serde_json::Value;
fn location(value: &Value) -> SpawnLocation {
    SpawnLocation {
        chunk: value[0].as_i64().unwrap() as i16,
        tile: value[1].as_i64().unwrap() as i16,
        position: std::array::from_fn(|i| value[i + 2].as_i64().unwrap() as i16),
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/scene-keys.json")).unwrap()
}
fn grid(value: &Value) -> SceneGrid {
    SceneGrid {
        columns: value[0].as_i64().unwrap() as i16,
        rows: value[1].as_i64().unwrap() as i16,
        mode: value[2].as_i64().unwrap() as i32,
    }
}
fn slots(value: &Value) -> SceneSlots {
    SceneSlots {
        grid: grid(&value["layout"]),
        slots: std::array::from_fn(|i| {
            let slot = &value["slots"][i];
            SceneSlot {
                label: slot["label"].as_i64().unwrap() as i16,
                map_index: slot["chunk"].as_i64().unwrap() as i16,
                origin: std::array::from_fn(|axis| slot["origin"][axis].as_i64().unwrap() as i32),
                ready: slot["ready"].as_bool().unwrap(),
            }
        }),
    }
}

#[test]
fn actor_cached_spawn_access_refresh_and_world_placement_match_native_methods() {
    let fixture = fixture();
    for case in fixture["cached"].as_array().unwrap() {
        let mut keys = SceneActorKeys {
            current: location(&case["initial"]),
            ..SceneActorKeys::default()
        };
        let graph = slots(&fixture["graphs"][case["graph"].as_u64().unwrap() as usize]);
        let actual = keys
            .current_spawn(
                case["refresh"].as_bool().unwrap(),
                &graph,
                std::array::from_fn(|i| case["position"][i].as_i64().unwrap() as i32),
            )
            .unwrap();
        assert_eq!(
            actual,
            case["result"].as_array().map(|_| location(&case["result"]))
        );
        assert_eq!(keys.current, location(&case["cached"]));
    }
    for case in fixture["placements"].as_array().unwrap() {
        let actual = grid(&case["layout"]).place_spawn(
            std::array::from_fn(|i| case["base"][i].as_i64().unwrap() as i32),
            location(&case["spawn"]),
        );
        assert_eq!(
            actual.position,
            std::array::from_fn(|i| case["position"][i].as_i64().unwrap() as i32)
        );
    }
    assert_eq!(fixture["cached"].as_array().unwrap().len(), 172);
    assert_eq!(fixture["placements"].as_array().unwrap().len(), 144);
}

#[test]
fn body_dispatch_stamps_keep_current_chunk_candidate_cell_and_existing_position_tail() {
    let fixture = fixture();
    let mut count = 0;
    for case in fixture["stamps"].as_array().unwrap() {
        let mut keys = SceneActorKeys {
            current: SpawnLocation {
                chunk: case["current_chunk"].as_i64().unwrap() as i16,
                ..SpawnLocation::default()
            },
            contact: location(&case["initial"]),
        };
        let cell: [i8; 2] = std::array::from_fn(|i| case["cell"][i].as_i64().unwrap() as i8);
        let observations = case["observations"].as_array().unwrap();
        for dz in 0..2 {
            for dx in 0..2 {
                keys.stamp_body_candidate([cell[0] + dx, cell[1] + dz]);
                assert_eq!(
                    keys.contact,
                    location(&observations[(dz * 2 + dx) as usize])
                );
                count += 1;
            }
        }
        assert_eq!(keys.contact, location(&case["final"]));
    }
    assert_eq!(count, 16);
}
