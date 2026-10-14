use lsd_assets::{
    StreamedStage, formats::Tmd, load_link_tables, load_math_tables, load_scene_grid,
};
use lsd_core::{
    ChunkPlayer, CollisionBounds, LinkedPlayer, MapLinkCommand, PadState, PlayerControls,
    SceneLinkTables, SpawnLocation,
};
use std::path::PathBuf;

#[test]
#[ignore = "Requires local original executable, models and streamed maps"]
fn original_teleport_triggers_reach_the_loaded_scene_link_pipeline() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let tables = load_link_tables(&executable).unwrap();
    let math = load_math_tables(&executable).unwrap();
    let body = Tmd::parse(&std::fs::read(files.join("CDI/ETC/DREAME5.TMD")).unwrap()).unwrap();
    let bounds = CollisionBounds::from_points(
        &body.objects[0]
            .vertices
            .iter()
            .map(|value| value.xyz)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut count = 0;
    for (stage, triggers) in tables.catalog.teleport.triggers.iter().enumerate() {
        let grid = load_scene_grid(&executable, stage as u8).unwrap();
        for trigger in triggers {
            let chunk = trigger.chunk.to_le_bytes();
            let map = u16::from(chunk[0]) + grid.columns as u16 * u16::from(chunk[1]);
            let streamed =
                StreamedStage::load(&files, &executable, stage as u8, map, [0; 3]).unwrap();
            let source = SpawnLocation {
                chunk: trigger.chunk,
                tile: if trigger.tile < 0 { 0 } else { trigger.tile },
                position: [
                    0,
                    if grid.mode != 0 {
                        -(map as i16) * 2048
                    } else {
                        0
                    },
                    0,
                ],
            };
            let mut player = ChunkPlayer {
                position: grid.place_spawn([0; 3], source).position,
                bounds,
                pad: PadState::default(),
                controls: PlayerControls {
                    move_direction: if stage == 0 { 2 } else { 3 },
                    previous_move_direction: if stage == 0 { 2 } else { 3 },
                    ..PlayerControls::default()
                },
            };
            let mut linked = LinkedPlayer::new(
                stage as u8,
                &player,
                &streamed.collision,
                SceneLinkTables {
                    catalog: tables.catalog.clone(),
                    motion: tables.motion.clone(),
                    auto_move: tables.auto_move.clone(),
                },
                1,
            )
            .unwrap();
            linked.context.teleports_enabled = 1;
            assert_eq!(
                (linked.keys.current.chunk, linked.keys.current.tile),
                (source.chunk, source.tile),
                "source key {stage}"
            );
            let ready = std::array::from_fn(|i| streamed.protocol.slots[i].ready);
            linked
                .step(&mut player, &streamed.collision, &math, ready, 0x1000)
                .unwrap();
            linked
                .refresh_key(&streamed.collision, player.position)
                .unwrap();
            linked
                .step(&mut player, &streamed.collision, &math, ready, 0x1000)
                .unwrap();
            linked
                .step(&mut player, &streamed.collision, &math, ready, 0x1000)
                .unwrap();
            assert!(
                linked.commands.contains(&MapLinkCommand::Notify(11)),
                "teleport notification {stage}"
            );
            assert_eq!(linked.links.execution.state, 0);
            let target = tables.catalog.teleport.spawns[trigger.target_stage as usize]
                [usize::from(trigger.target_spawn)];
            let expected = SpawnLocation {
                chunk: target.chunk,
                tile: target.tile,
                position: tables.catalog.adjustments[usize::from(target.adjustment)],
            };
            assert_eq!(linked.links.spawn, expected);
            assert_eq!(player.position, grid.place_spawn([0; 3], expected).position);
            assert_eq!(
                linked.links.execution.stage,
                i32::from(trigger.target_stage)
            );
            count += 1;
        }
    }
    assert_eq!(count, 17);
}
