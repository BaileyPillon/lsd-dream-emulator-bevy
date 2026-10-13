use lsd_assets::load_stream_grid_tables;
use lsd_core::{SceneGrid, SceneSlot, SceneSlots};
use serde_json::Value;
use std::path::PathBuf;

fn vector<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
fn grid(value: &Value) -> SceneGrid {
    let [columns, rows, mode] = vector(value);
    SceneGrid {
        columns: columns as i16,
        rows: rows as i16,
        mode,
    }
}

#[test]
#[ignore = "Requires original local executable and private native stream observations"]
fn original_direction_tables_reproduce_neighbor_and_rebinding_observations() {
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let observations = PathBuf::from(
        std::env::var_os("LSD_NATIVE_STREAM_CASES").expect("Set LSD_NATIVE_STREAM_CASES"),
    );
    let tables =
        load_stream_grid_tables(&std::fs::read(root.join("SLPS_015.56")).unwrap()).unwrap();
    let fixture: Value = serde_json::from_slice(&std::fs::read(observations).unwrap()).unwrap();
    for (i, case) in fixture["neighbors"].as_array().unwrap().iter().enumerate() {
        let actual = tables
            .neighbor(
                grid(&case["layout"]),
                case["index"].as_i64().unwrap() as i32,
                case["parity"].as_i64().unwrap() as i32,
                case["mask"].as_u64().unwrap() as u32,
                case["direction"].as_u64().unwrap() as u8,
            )
            .unwrap();
        assert_eq!(
            actual,
            case["neighbor"].as_i64().map(|value| value as i32),
            "neighbor {i}"
        );
    }
    for (i, case) in fixture["rebindings"].as_array().unwrap().iter().enumerate() {
        let slots = SceneSlots {
            grid: grid(&case["layout"]),
            slots: std::array::from_fn(|physical| SceneSlot {
                label: case["old_labels"][physical].as_i64().unwrap() as i16,
                map_index: physical as i16,
                ready: false,
                origin: [
                    physical as i32 * 77,
                    -(physical as i32) * 11,
                    physical as i32 * 33,
                ],
            }),
        };
        let address = case["selection_address"].as_u64().unwrap() as u32;
        let selection = if address == 0x80086904 {
            tables.initial
        } else {
            tables.reuse[(address - 0x80086914) as usize / 16 + 1].unwrap()
        };
        let actual = tables
            .rebind(
                &slots,
                case["index"].as_i64().unwrap() as i32,
                vector(&case["center"]),
                Some(selection),
            )
            .unwrap()
            .unwrap();
        for physical in 0..7 {
            let expected = &case["slots"][physical];
            assert_eq!(
                actual.slots.slots[physical].label,
                expected["label"].as_i64().unwrap() as i16,
                "rebind {i} slot {physical}"
            );
            assert_eq!(
                actual.slots.slots[physical].origin,
                vector(&expected["origin"]),
                "rebind {i} slot {physical}"
            );
            assert_eq!(
                actual.coordinate_flags_cleared[physical],
                expected["coordinate_flag"].as_u64().unwrap() == 0,
                "rebind {i} slot {physical}"
            );
        }
        assert_eq!(
            actual.requests.len(),
            case["requests"].as_array().unwrap().len(),
            "rebind {i}"
        );
        for (actual, expected) in actual
            .requests
            .iter()
            .zip(case["requests"].as_array().unwrap())
        {
            assert_eq!(
                u64::from(actual.label),
                expected[2].as_u64().unwrap(),
                "rebind {i}"
            );
            let expected = if expected[0].as_u64().unwrap() == 0 {
                None
            } else {
                Some(expected[1].as_u64().unwrap() as u32 as i32)
            };
            assert_eq!(actual.map_index, expected, "rebind {i}");
        }
    }
    assert_eq!(fixture["neighbors"].as_array().unwrap().len(), 2163);
    assert_eq!(fixture["rebindings"].as_array().unwrap().len(), 308);
}
