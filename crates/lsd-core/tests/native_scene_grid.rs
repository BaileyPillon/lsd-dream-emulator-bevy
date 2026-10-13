use lsd_core::{SceneGrid, SceneLocation, SceneSlot, SceneSlots};
use serde_json::Value;

fn array_i32<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
fn grid(value: &Value) -> SceneGrid {
    SceneGrid {
        columns: value[0].as_i64().unwrap() as i16,
        rows: value[1].as_i64().unwrap() as i16,
        mode: value[2].as_i64().unwrap() as i32,
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/scene-lookup.json")).unwrap()
}
fn graph(value: &Value) -> SceneSlots {
    SceneSlots {
        grid: grid(&value["layout"]),
        slots: std::array::from_fn(|i| {
            let slot = &value["slots"][i];
            SceneSlot {
                label: slot["label"].as_i64().unwrap() as i16,
                map_index: slot["chunk"].as_i64().unwrap() as i16,
                origin: array_i32(&slot["origin"]),
                ready: slot["ready"].as_bool().unwrap(),
            }
        }),
    }
}
fn location(value: &Value) -> SceneLocation {
    SceneLocation {
        slot: value["slot"].as_u64().unwrap() as usize,
        map_index: value["chunk_index"].as_i64().unwrap() as i16,
        grid_index: array_i32(&value["grid_index"]).map(|v| v as i8),
        cell: array_i32(&value["cell"]).map(|v| v as i8),
        cell_offset: array_i32(&value["cell_offset"]).map(|v| v as i16),
        reference_origin: array_i32(&value["reference_origin"]),
        relative_position: array_i32(&value["relative_position"]),
    }
}

#[test]
fn world_locations_and_query_order_match_original_cpu() {
    let fixture = fixture();
    let graphs: Vec<_> = fixture["graphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(graph)
        .collect();
    for (index, case) in fixture["locations"].as_array().unwrap().iter().enumerate() {
        let graph = &graphs[case["graph"].as_u64().unwrap() as usize];
        let expected = if case["expected"].is_null() {
            None
        } else {
            Some(location(&case["expected"]))
        };
        assert_eq!(
            graph.locate(array_i32(&case["position"])).unwrap(),
            expected,
            "location {index}"
        );
    }
    for (index, case) in fixture["queries"].as_array().unwrap().iter().enumerate() {
        let graph = &graphs[case["graph"].as_u64().unwrap() as usize];
        let result = graph
            .queries(
                location(&case["location"]),
                case["width"].as_i64().unwrap() as i32,
            )
            .unwrap();
        let expected = case["selection"].as_array().unwrap();
        assert_eq!(result.len(), expected.len(), "query {index}");
        for (actual, expected) in result.iter().zip(expected) {
            assert_eq!(
                actual.slot,
                expected["slot"].as_u64().unwrap() as usize,
                "query {index}"
            );
            assert_eq!(
                [
                    i32::from(actual.cells.x),
                    i32::from(actual.cells.z),
                    actual.cells.columns,
                    actual.cells.rows
                ],
                array_i32(&expected["rectangle"]),
                "query {index}"
            );
        }
    }
    assert_eq!(fixture["locations"].as_array().unwrap().len(), 1108);
    assert_eq!(fixture["queries"].as_array().unwrap().len(), 3690);
}

#[test]
fn chunk_placement_matches_original_cpu() {
    let fixture = fixture();
    for (index, case) in fixture["placements"].as_array().unwrap().iter().enumerate() {
        let result = grid(&case["layout"]).place(
            array_i32(&case["base"]),
            array_i32(&case["grid_index"]).map(|v| v as i8),
            array_i32(&case["cell"]).map(|v| v as i8),
            array_i32(&case["cell_offset"]).map(|v| v as i16),
        );
        assert_eq!(
            result.index,
            case["index"].as_i64().unwrap() as i32,
            "placement {index}"
        );
        assert_eq!(
            result.position,
            array_i32(&case["position"]),
            "placement {index}"
        );
        assert_eq!(
            result.center,
            array_i32(&case["center"]),
            "placement {index}"
        );
    }
    assert_eq!(fixture["placements"].as_array().unwrap().len(), 144);
}
