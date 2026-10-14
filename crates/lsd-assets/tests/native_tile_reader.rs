use lsd_assets::formats::Tile;
use serde_json::Value;

#[test]
fn tile_pose_attributes_and_chain_selection_match_original_reader() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/tile-reader.json")).unwrap();
    let mut calls = 0;
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let fields = &case["tile"];
        let extra = case["next_offset"].as_u64().unwrap() != 0;
        let tile = Tile {
            tag: fields[0].as_u64().unwrap() as u8,
            category: fields[1].as_u64().unwrap() as u8,
            model: if extra {
                124
            } else {
                fields[2].as_u64().unwrap() as u16
            },
            flags: fields[3].as_u64().unwrap() as u8,
            quarter_turn: fields[4].as_u64().unwrap() as u8,
            height: fields[5].as_i64().unwrap() as i16,
            next_offset: if extra {
                0
            } else {
                fields[6].as_u64().unwrap() as u32
            },
        };
        let cell = case["cell"].as_u64().unwrap() as usize;
        let expected = &case["expected"];
        if cell >= 400 {
            assert_eq!(expected["returned"].as_i64().unwrap(), 0, "case {index}");
            assert!(!case["changed"].as_bool().unwrap());
        } else {
            assert_eq!(
                expected["extra"].as_i64().unwrap(),
                i64::from(extra),
                "case {index}"
            );
            assert_eq!(
                expected["next_offset"].as_u64().unwrap(),
                u64::from(tile.next_offset),
                "case {index}"
            );
            if tile.present() {
                assert_eq!(
                    tile.native_position(cell),
                    std::array::from_fn(|i| expected["position"][i].as_i64().unwrap() as i32),
                    "case {index}"
                );
                assert_eq!(
                    tile.native_heading(),
                    expected["heading"].as_i64().unwrap() as i16,
                    "case {index}"
                );
                assert_eq!(
                    u64::from(tile.category),
                    expected["category"].as_u64().unwrap()
                );
                assert_eq!(u64::from(tile.flags), expected["flags"].as_u64().unwrap());
                assert_eq!(u64::from(tile.model), expected["model"].as_u64().unwrap());
                assert_eq!(expected["lookup"][0][1].as_u64().unwrap(), cell as u64);
                calls += 1;
            } else {
                assert_eq!(expected["returned"].as_i64().unwrap(), -1);
                assert!(expected["lookup"].as_array().unwrap().is_empty());
            }
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 168);
    assert!(calls > 0);
}
