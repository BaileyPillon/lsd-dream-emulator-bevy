use lsd_assets::StreamedStage;
use std::path::PathBuf;

#[test]
#[ignore = "Requires original local executable and stage maps"]
fn spatial_map_handoffs_preserve_resident_neighbors_and_complete_pending_requests() {
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(root.join("SLPS_015.56")).unwrap();
    for index in 0..14 {
        let corner = StreamedStage::load(&root, &executable, index, 0, [0; 3]).unwrap();
        assert_eq!(
            (corner.protocol.activity, corner.protocol.pending),
            (0, 0),
            "stage {index}"
        );
        assert!(
            (1..=7).contains(&corner.maps.iter().flatten().count()),
            "stage {index}"
        );
    }
    let mut stage = StreamedStage::load(&root, &executable, 3, 136, [0; 3]).unwrap();
    assert_eq!(stage.maps.iter().flatten().count(), 7);
    assert_eq!((stage.protocol.activity, stage.protocol.pending), (0, 0));
    for _ in 0..4 {
        let right = stage
            .collision
            .slots
            .slots
            .iter()
            .position(|slot| slot.label == 4)
            .unwrap();
        let entry = stage.collision.slots.slots[right];
        let position = [
            entry.origin[0] + 20480,
            entry.origin[1],
            entry.origin[2] + 20480,
        ];
        let current = stage.collision.slots.locate(position).unwrap().unwrap();
        let wanted = current.map_index;
        assert!(stage.update_for_position(position).unwrap());
        assert!(stage.protocol.activity != 0);
        // The current map is retained through a direction-label change, before new reads bind.
        let retained = stage
            .collision
            .slots
            .slots
            .iter()
            .position(|slot| slot.label == 3)
            .unwrap();
        assert_eq!(stage.collision.slots.slots[retained].map_index, wanted);
        assert!(stage.maps[retained].is_some());
        stage.complete_frame().unwrap();
        assert_eq!((stage.protocol.activity, stage.protocol.pending), (0, 0));
        assert_eq!(
            stage
                .collision
                .slots
                .locate(position)
                .unwrap()
                .unwrap()
                .map_index,
            wanted
        );
        for (slot, data) in stage.collision.slots.slots.iter().zip(&stage.maps) {
            assert_eq!(slot.ready, data.is_some());
        }
    }
    let cottage = StreamedStage::load(&root, &executable, 0, 0, [0; 3]).unwrap();
    assert_eq!(cottage.maps.iter().flatten().count(), 5);
}
