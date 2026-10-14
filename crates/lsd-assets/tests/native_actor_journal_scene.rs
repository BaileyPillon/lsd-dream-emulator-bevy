use lsd_assets::{
    StageLinkSession, StreamedStage, formats::Tmd, load_day_tables, load_effect_palette,
    load_encounter_tables, load_entity_properties, load_generation_effect_tables,
    load_generation_object_tables, load_generation_paths, load_generation_tables, load_link_tables,
    load_math_tables,
};
use lsd_core::{
    ChunkPlayer, CollisionBounds, EntityTarget, LinkedPlayer, MapEffect, PadState, PlayerControls,
    SceneLinkTables,
};
#[test]
#[ignore = "Requires original stage/model/texture corpus and canonical executable"]
fn loaded_futon_trigger_immediately_updates_mood_score_and_recall_rng_once() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let original = load_link_tables(&executable).unwrap();
    let content = load_day_tables(&executable).unwrap();
    let model = Tmd::parse(&std::fs::read(files.join("CDI/ETC/DREAME5.TMD")).unwrap()).unwrap();
    let bounds = CollisionBounds::from_points(
        &model.objects[0]
            .vertices
            .iter()
            .map(|v| v.xyz)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut stage = StreamedStage::load(&files, &executable, 0, 0, [0; 3]).unwrap();
    let mut player = ChunkPlayer {
        position: stage
            .collision
            .slots
            .slots
            .iter()
            .find(|slot| slot.ready)
            .unwrap()
            .origin,
        bounds,
        pad: PadState::default(),
        controls: PlayerControls::default(),
    };
    let mut linked = LinkedPlayer::new(
        0,
        &player,
        &stage.collision,
        SceneLinkTables {
            catalog: original.catalog,
            motion: original.motion,
            auto_move: original.auto_move,
        },
        1,
    )
    .unwrap();
    let mut effect = MapEffect::new(load_effect_palette(&executable).unwrap());
    let mut session = StageLinkSession {
        player_collision: Some(lsd_core::CollisionModel {
            bounds: Some(bounds),
            faces: model.objects[0].collision_faces(),
        }),
        ..Default::default()
    };
    session.enable_generation(
        load_generation_tables(&executable).unwrap(),
        load_generation_paths(&executable).unwrap(),
    );
    session.enable_generated_objects(load_generation_object_tables(&executable).unwrap());
    session.enable_generation_layers(load_generation_effect_tables(&executable).unwrap());
    session.enable_encounters(
        load_encounter_tables(&executable).unwrap(),
        load_entity_properties(&executable).unwrap(),
    );
    session.begin_original_day(&mut linked, content);
    session
        .tick(&mut stage, &mut player, &mut linked, &mut effect)
        .unwrap();
    let math = load_math_tables(&executable).unwrap();
    session
        .tick_encounters(&stage, &mut linked, &mut player, &math, &mut effect)
        .unwrap();
    assert_eq!(session.day.as_ref().unwrap().instances.count, 0);
    let position = session
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .find(|actor| actor.entity == 5)
        .unwrap()
        .actor
        .pose
        .position;
    let offset = EntityTarget {
        position: [0; 3],
        local_y: 0,
        matrix: math.rotation(player.controls.angles),
        eye: player.controls.eye,
        target: player.controls.target,
    }
    .resolve(2048)
    .unwrap();
    player.position = std::array::from_fn(|axis| position[axis].wrapping_sub(offset[axis]));
    linked
        .refresh_key(&stage.collision, player.position)
        .unwrap();
    let source_key = linked.keys.current;
    let mut expected_rng = linked.rng;
    let recall = expected_rng.next_u15().is_multiple_of(3);
    session
        .tick_encounters(&stage, &mut linked, &mut player, &math, &mut effect)
        .unwrap();
    let day = session.day.as_ref().unwrap();
    assert_eq!(day.instances.count, 1);
    assert_eq!(day.instances.last.0, [-4, 4]);
    assert_eq!(day.instance_score, -50000);
    assert_eq!(linked.rng.state(), expected_rng.state());
    assert_eq!(day.flashback_count, i32::from(recall));
    if recall {
        assert_eq!(day.flashbacks[0].spawn, source_key);
        assert_eq!(day.flashbacks[0].seconds, 16);
    }
    assert!(
        !session
            .encounters
            .as_ref()
            .unwrap()
            .pending_notifications
            .iter()
            .any(|event| matches!(event.kind, lsd_assets::EncounterNotificationKind::Notify(9)))
    );
    let retained_rng = linked.rng.state();
    session
        .tick_encounters(&stage, &mut linked, &mut player, &math, &mut effect)
        .unwrap();
    assert_eq!(session.day.as_ref().unwrap().instances.count, 1);
    assert_eq!(linked.rng.state(), retained_rng);
    println!(
        "original_loaded_futon_journal_moods=1 instance_score=-50000 immediate_shared_rng_order=true supplied_player_placement=true whole_npc_encounter_acceptance=false"
    );
}
