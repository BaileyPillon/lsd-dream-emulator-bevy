use lsd_core::{
    BodyContactState, CollisionChunk, CollisionModel, CollisionPolicy, CollisionTile,
    SceneCollision, SceneGrid, SceneLocation, SceneSlot, SceneSlots,
};
use serde_json::Value;

#[test]
fn body_rectangles_and_attribute_filtered_chain_order_match_native_cpu() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/body-dispatch.json")).unwrap();
    let mut visits = 0;
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let graph = &fixture["graphs"][case["graph"].as_u64().unwrap() as usize];
        let slots = SceneSlots {
            grid: SceneGrid {
                columns: 3,
                rows: 7,
                mode: graph["mode"].as_i64().unwrap() as i32,
            },
            slots: std::array::from_fn(|i| SceneSlot {
                label: graph["labels"][i].as_i64().unwrap() as i16,
                map_index: i as i16 + 100,
                ready: graph["ready"][i].as_bool().unwrap(),
                origin: [0; 3],
            }),
        };
        let location = SceneLocation {
            slot: 0,
            map_index: 100,
            grid_index: [0; 2],
            cell: std::array::from_fn(|i| case["cell"][i].as_i64().unwrap() as i8),
            cell_offset: [0; 3],
            reference_origin: [0; 3],
            relative_position: [0; 3],
        };
        let queries = slots.body_queries(location).unwrap();
        let expected = case["rectangles"].as_array().unwrap();
        assert_eq!(queries.len(), expected.len(), "case {index}");
        for (query, expected) in queries.iter().zip(expected) {
            assert_eq!(
                query.slot,
                expected["slot"].as_u64().unwrap() as usize,
                "case {index}"
            );
            let values = [
                i32::from(query.cells.x),
                i32::from(query.cells.z),
                query.cells.columns,
                query.cells.rows,
            ];
            assert_eq!(
                values,
                std::array::from_fn(|i| expected["rectangle"][i].as_i64().unwrap() as i32),
                "case {index}"
            );
        }
        let scene = SceneCollision {
            slots: slots.clone(),
            chunks: std::array::from_fn(|physical| {
                Some(CollisionChunk {
                    policy: CollisionPolicy::default(),
                    models: vec![CollisionModel {
                        bounds: None,
                        faces: Vec::new(),
                    }],
                    cells: graph["cells"][physical]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|cell| {
                            cell.as_array()
                                .unwrap()
                                .iter()
                                .map(|flags| CollisionTile {
                                    model: 0,
                                    translation: [0; 3],
                                    heading: 0,
                                    category: 0,
                                    flags: flags.as_u64().unwrap() as u8,
                                })
                                .collect()
                        })
                        .collect(),
                })
            }),
        };
        let actual_visits: Vec<_> = scene
            .body_candidates(location)
            .unwrap()
            .iter()
            .map(|candidate| [candidate.slot, candidate.cell, candidate.tile])
            .collect();
        let expected_visits: Vec<[usize; 3]> = case["visits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|visit| std::array::from_fn(|i| visit["node"][i].as_u64().unwrap() as usize))
            .collect();
        assert_eq!(actual_visits, expected_visits, "dispatch case {index}");
        visits += actual_visits.len();
    }
    assert_eq!(visits, 740);
}

#[test]
fn body_notification_contact_assignment_matches_original_scalar_handler() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/body-dispatch.json")).unwrap();
    for sequence in fixture["contact_sequences"].as_array().unwrap() {
        let mut state = BodyContactState {
            current: Some(123u32),
        };
        for step in sequence.as_array().unwrap() {
            state.notify(
                step["value"].as_u64().unwrap() as u32,
                step["code"].as_i64().unwrap() as i32,
            );
            assert_eq!(
                state.current,
                Some(step["contact"].as_u64().unwrap() as u32)
            );
        }
    }
}
