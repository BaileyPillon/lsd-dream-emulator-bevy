use lsd_assets::{formats::Lbd, load_encounter_tables, stage_map_path};
#[test]
#[ignore = "Requires canonical executable and original map animation libraries"]
fn original_encounter_tables_bind_declared_actor_models_and_variants() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let tables = load_encounter_tables(&executable).unwrap();
    let mut contexts = 0;
    let mut model_bindings = 0;
    for stage in 0..14 {
        for chunk in &tables.chunks[stage] {
            let coords = chunk.chunk.to_le_bytes().map(|v| i32::from(v as i8));
            let grid = lsd_assets::load_scene_grid(&executable, stage as u8).unwrap();
            let map = coords[1] * i32::from(grid.columns) + coords[0];
            let path = stage_map_path(&executable, stage as u8, map as u16).unwrap();
            let data = std::fs::read(files.join(path)).unwrap();
            let map = Lbd::parse(&data).unwrap();
            contexts += 1;
            for index in chunk.objects.into_iter().take_while(|&index| index != -1) {
                let object = &tables.objects[stage][index as usize];
                if object.model >= 0
                    && let Some(model) = map
                        .model_library
                        .as_ref()
                        .and_then(|v| v.models.get(object.model as usize))
                {
                    let actor = model.construct_actor().unwrap();
                    assert!(actor.root_part < actor.animation.parts.len());
                    model_bindings += 1;
                }
                for index in object.variants.into_iter().take_while(|&index| index != -1) {
                    let variant = tables.variants[index as u8 as usize];
                    assert!((variant.rotation as usize) < tables.rotations.len());
                    assert!((variant.position as usize) < tables.positions.len());
                }
            }
        }
    }
    assert!(contexts > 90 && model_bindings > 100);
    println!(
        "original_encounter_chunk_contexts={contexts} available_actor_model_bindings={model_bindings} whole_npc_encounter_acceptance=false"
    );
}

#[test]
#[ignore = "Requires canonical executable and the original streamed stage corpus"]
fn original_loaded_encounters_bind_once_retire_slots_and_rebuild_without_stale_actors() {
    use lsd_assets::{EncounterScene, StreamedStage, load_entity_properties};
    use lsd_core::PsxRng;
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let tables = load_encounter_tables(&executable).unwrap();
    let properties = load_entity_properties(&executable).unwrap();
    assert_eq!(properties.len(), 130);
    let mut contexts = 0;
    let mut actors = 0;
    for stage in 0..14 {
        let grid = lsd_assets::load_scene_grid(&executable, stage as u8).unwrap();
        let maps = tables.chunks[stage]
            .iter()
            .map(|chunk| {
                let [column, row] = chunk.chunk.to_le_bytes().map(u16::from);
                row * grid.columns as u16 + column
            })
            .collect::<std::collections::BTreeSet<_>>();
        for map in maps {
            let mut loaded =
                StreamedStage::load(&files, &executable, stage as u8, map, [0; 3]).unwrap();
            let mut scene = EncounterScene::new(tables.clone(), properties.clone());
            let mut rng = PsxRng::new(1);
            scene.update(&loaded, 0, 8, 1, 2, &mut rng).unwrap();
            let identity = scene
                .actors
                .iter()
                .map(|a| (a.serial, a.slot, a.map, a.entity))
                .collect::<Vec<_>>();
            let retained_rng = rng.state();
            scene.update(&loaded, 0, 8, 1, 2, &mut rng).unwrap();
            assert_eq!(
                identity,
                scene
                    .actors
                    .iter()
                    .map(|a| (a.serial, a.slot, a.map, a.entity))
                    .collect::<Vec<_>>()
            );
            assert_eq!(retained_rng, rng.state(), "duplicate binding consumed RNG");
            for actor in &scene.actors {
                assert!(loaded.collision.slots.slots[actor.slot].ready);
                assert_eq!(loaded.protocol.slots[actor.slot].map_index, actor.map);
                assert_eq!(properties[actor.entity as usize], actor.properties);
                assert_eq!(actor.properties.callback.is_some(), actor.behavior_pending);
                assert!(actor.actor.root_part < actor.actor.animation.parts.len());
            }
            actors += scene.actors.len();
            contexts += 1;
            if let Some(retired_slot) = scene.actors.first().map(|actor| actor.slot) {
                loaded.collision.slots.slots[retired_slot].ready = false;
                scene.update(&loaded, 0, 8, 1, 2, &mut rng).unwrap();
                assert!(scene.actors.iter().all(|actor| actor.slot != retired_slot));
                loaded.collision.slots.slots[retired_slot].ready = true;
                scene.update(&loaded, 0, 8, 1, 2, &mut rng).unwrap();
                assert!(
                    scene
                        .actors
                        .iter()
                        .filter(|actor| actor.slot == retired_slot)
                        .all(|actor| !identity.iter().any(|old| old.0 == actor.serial))
                );
            }
            let old_serials = scene
                .actors
                .iter()
                .map(|actor| actor.serial)
                .collect::<Vec<_>>();
            scene.update(&loaded, 1, 8, 1, 2, &mut rng).unwrap();
            assert!(
                scene
                    .actors
                    .iter()
                    .all(|actor| !old_serials.contains(&actor.serial))
            );
            scene.stop();
            assert!(scene.actors.is_empty());
            assert!(!scene.teleports_requested);
        }
    }
    assert!(contexts > 90 && actors > 50);
    println!(
        "original_loaded_encounter_contexts={contexts} constructed_actor_instances={actors} whole_npc_encounter_acceptance=false"
    );
}
