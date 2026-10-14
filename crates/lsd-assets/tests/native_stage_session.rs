use lsd_assets::{
    StageLinkSession, StageSessionEvent, StreamedStage, formats::Tmd, load_effect_palette,
    load_link_tables, load_math_tables, load_scene_grid,
};
use lsd_core::{
    ChunkPlayer, CollisionBounds, LinkedPlayer, MapCompletionCommand, MapEffect, PadState,
    PlayerControls, SceneLinkTables, SpawnLocation,
};
use std::path::PathBuf;

#[test]
#[ignore = "Requires local original maps, model, executable and effect colors"]
fn supplied_original_permanent_link_destinations_rebuild_after_fade_completion() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let tables = load_link_tables(&executable).unwrap();
    let body = Tmd::parse(&std::fs::read(files.join("CDI/ETC/DREAME5.TMD")).unwrap()).unwrap();
    let bounds = CollisionBounds::from_points(
        &body.objects[0]
            .vertices
            .iter()
            .map(|vertex| vertex.xyz)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut count = 0;
    let math = load_math_tables(&executable).unwrap();
    for (source_stage, triggers) in tables.catalog.wall.triggers.iter().enumerate() {
        for trigger in triggers {
            let grid = load_scene_grid(&executable, source_stage as u8).unwrap();
            let source = SpawnLocation {
                chunk: trigger.chunk,
                tile: trigger.tile.max(0),
                position: [0; 3],
            };
            let placement = grid.place_spawn([0; 3], source);
            let mut stage = StreamedStage::load(
                &files,
                &executable,
                source_stage as u8,
                placement.index as u16,
                [0; 3],
            )
            .unwrap();
            let mut player = ChunkPlayer {
                position: placement.position,
                bounds,
                pad: PadState::default(),
                controls: PlayerControls::default(),
            };
            let mut linked = LinkedPlayer::new(
                source_stage as u8,
                &player,
                &stage.collision,
                SceneLinkTables {
                    catalog: tables.catalog.clone(),
                    motion: tables.motion.clone(),
                    auto_move: tables.auto_move.clone(),
                },
                0x12345678,
            )
            .unwrap();
            linked.links.execution.dream_timer = 12345;
            let rng = linked.rng.state();
            linked.keys.contact = source;
            linked
                .wall_contact(&mut player, &stage.collision, 4)
                .unwrap();
            let destination = i32::from(trigger.target_stage);
            assert_eq!(linked.links.execution.stage, destination);
            assert_eq!(linked.sounds, [[0x90, 0x6e, 0x6e]]);
            let commands = linked.commands.clone();
            let mut effect = MapEffect::new(load_effect_palette(&executable).unwrap());
            for command in commands {
                effect.consume(command).unwrap();
            }
            let mut session = StageLinkSession::default();
            for index in 0..9 {
                let output = effect.tick(&mut linked.map, &mut player.controls).unwrap();
                if index < 8 {
                    assert!(output.is_empty());
                } else {
                    for event in output {
                        if let MapCompletionCommand::Notify(code) = event {
                            session.receive(code);
                        }
                    }
                }
                assert_eq!(stage.stage_id(), source_stage as u8);
                assert_eq!(session.rebuilds, 0);
            }
            assert_eq!(session.policy.state, 3);
            assert!(
                session
                    .tick(&mut stage, &mut player, &mut linked, &mut effect)
                    .unwrap()
            );
            assert_eq!(stage.stage_id(), destination as u8);
            assert_eq!(session.rebuilds, 1);
            assert_eq!(session.policy.state, 2);
            assert_eq!(
                session.events[0..5],
                [
                    StageSessionEvent::StopMap,
                    StageSessionEvent::DestroyMap,
                    StageSessionEvent::CreateMap(destination),
                    StageSessionEvent::AttachMap,
                    StageSessionEvent::RunMap
                ]
            );
            let expected = load_scene_grid(&executable, destination as u8)
                .unwrap()
                .place_spawn([0; 3], linked.links.spawn);
            assert_eq!(player.position, expected.position);
            assert!(
                stage
                    .collision
                    .slots
                    .locate(player.position)
                    .unwrap()
                    .is_some()
            );
            assert_eq!(linked.links.execution.dream_timer, 12345);
            assert_eq!(linked.rng.state(), rng);
            assert_eq!(linked.map.state, 5);
            assert_eq!(effect.effect.state, 1);
            assert_eq!(linked.map.move_callback, 1);
            assert!(!player.controls.input_locked);
            while effect.render_attached {
                assert!(
                    effect
                        .tick(&mut linked.map, &mut player.controls)
                        .unwrap()
                        .iter()
                        .all(|command| !matches!(command, MapCompletionCommand::Notify(_)))
                );
            }
            assert_eq!(linked.map.state, 0);
            assert_eq!(player.controls.paused, 0);
            let ready = std::array::from_fn(|i| stage.protocol.slots[i].ready);
            linked
                .step(&mut player, &stage.collision, &math, ready, 0)
                .unwrap();
            count += 1;
        }
    }
    assert_eq!(count, 17);
}
