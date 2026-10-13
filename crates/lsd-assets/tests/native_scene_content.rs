use lsd_assets::{load_layered_stage, load_scene_grid, stage_map_path};
use std::path::PathBuf;

#[test]
#[ignore = "Requires local original executable and stage maps"]
fn canonical_scene_descriptors_agree_with_disc_maps_and_layered_imports() {
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(root.join("SLPS_015.56")).unwrap();
    let mut total = 0;
    for stage in 0..14 {
        let grid = load_scene_grid(&executable, stage).unwrap();
        let mut pending = vec![root.join(format!("CDI/STG{stage:02}"))];
        let mut files = 0;
        while let Some(folder) = pending.pop() {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("LBD"))
                {
                    files += 1;
                }
            }
        }
        assert_eq!(
            files,
            (i32::from(grid.columns) * i32::from(grid.rows)) as usize,
            "stage {stage}"
        );
        total += files;
        for map in 0..files {
            assert!(
                root.join(stage_map_path(&executable, stage, map as u16).unwrap())
                    .is_file()
            );
        }
    }
    assert_eq!(total, 418);
    for stage in [0, 6] {
        let layered = load_layered_stage(&root, &executable, stage, [0; 3]).unwrap();
        assert_eq!(
            layered.maps.len(),
            layered.collision.slots.grid.rows as usize
        );
        assert_eq!(
            layered
                .collision
                .slots
                .slots
                .iter()
                .filter(|slot| slot.ready)
                .count(),
            layered.maps.len()
        );
        assert!(layered.maps.iter().all(|map| !map.tmd.objects.is_empty()));
    }
    assert!(load_layered_stage(&root, &executable, 2, [0; 3]).is_err());
    assert!(load_scene_grid(&executable, 14).is_err());
}
