use lsd_assets::{
    StageLinkSession, StreamedStage, formats::Tmd, load_day_tables, load_effect_palette,
    load_encounter_tables, load_entity_properties, load_generation_effect_tables,
    load_generation_object_tables, load_generation_paths, load_generation_tables, load_link_tables,
    load_math_tables,
};
use lsd_core::{
    ChunkPlayer, CollisionBounds, LinkedPlayer, MapEffect, PadState, PlayerControls,
    SceneLinkTables,
};
#[test]
#[ignore = "Requires the original stage, executable, player and encounter assets"]
fn loaded_npc_request_runs_after_draw_and_resets_on_rebuild() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let exe = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let original = load_link_tables(&exe).unwrap();
    let body = Tmd::parse(&std::fs::read(files.join("CDI/ETC/DREAME5.TMD")).unwrap()).unwrap();
    let bounds = CollisionBounds::from_points(
        &body.objects[0]
            .vertices
            .iter()
            .map(|v| v.xyz)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut stage = StreamedStage::load(&files, &exe, 0, 0, [0; 3]).unwrap();
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
    let math = load_math_tables(&exe).unwrap();
    let mut effect = MapEffect::new(load_effect_palette(&exe).unwrap());
    let mut session = StageLinkSession {
        player_collision: Some(lsd_core::CollisionModel {
            bounds: Some(bounds),
            faces: body.objects[0].collision_faces(),
        }),
        ..Default::default()
    };
    session.enable_generation(
        load_generation_tables(&exe).unwrap(),
        load_generation_paths(&exe).unwrap(),
    );
    session.enable_generated_objects(load_generation_object_tables(&exe).unwrap());
    session.enable_generation_layers(load_generation_effect_tables(&exe).unwrap());
    session.enable_encounters(
        load_encounter_tables(&exe).unwrap(),
        load_entity_properties(&exe).unwrap(),
    );
    session
        .encounters
        .as_mut()
        .unwrap()
        .enable_special_encounter(lsd_assets::load_special_encounter_assets(&exe, &files).unwrap());
    session
        .encounters
        .as_mut()
        .unwrap()
        .enable_entity_effects(lsd_assets::load_entity_effect_definition(&exe).unwrap());
    session.begin_original_day(&mut linked, load_day_tables(&exe).unwrap());
    session
        .tick(&mut stage, &mut player, &mut linked, &mut effect)
        .unwrap();

    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/scene-scale-npc-runtime.json")).unwrap();
    let text = fixture["owned_model_hex"].as_str().unwrap();
    let bytes: Vec<u8> = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect();
    let model = std::sync::Arc::new(lsd_assets::formats::Mom::parse(&bytes).unwrap());
    let fields = [0, 0, 2, 0, 0, 0, 0, 0, 0, 1, 10, 0];
    let mut actor = model.construct_actor().unwrap();
    let mut behavior = lsd_core::EntityBehavior::construct(
        17,
        lsd_core::NpcScript::OldWoman,
        fields,
        &mut actor,
        &mut session.encounter_audio,
    );
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    behavior.state = 11;
    behavior.ticks = 1;
    actor.pose.position = [player.position[0], 2000, player.position[2]];
    let encounters = session.encounters.as_mut().unwrap();
    let serial = encounters.add_actor(lsd_assets::SceneActor {
        serial: 0,
        slot: 0,
        map: 0,
        entity: 17,
        model: 0,
        resource: model,
        actor,
        collision: None,
        properties: lsd_assets::EntityProperties {
            fields,
            callback: Some(0x8005_ea94),
        },
        behavior_active: true,
        behavior_pending: false,
        behavior,
    });
    let before = stage.collision.floor(player.position, &math).unwrap();
    for (remaining, visible, live) in [
        (3, 4096, 5120),
        (2, 5120, 6144),
        (1, 6144, 7168),
        (-1, 7168, 8192),
        (-1, 8192, 8192),
    ] {
        session
            .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
            .unwrap();
        let scale = session.scene_scale.as_ref().unwrap();
        assert_eq!(
            (scale.remaining, scale.scale),
            (remaining, [4096, live, 4096])
        );
        assert_eq!(session.rendered_scene_scale, [4096, visible, 4096]);
    }
    assert_eq!(
        session
            .encounters
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .find(|actor| actor.serial == serial)
            .unwrap()
            .behavior
            .state,
        12
    );
    assert_eq!(
        before,
        stage.collision.floor(player.position, &math).unwrap()
    );
    let epoch = session.rebuilds;
    session.policy.state = 1;
    session
        .tick(&mut stage, &mut player, &mut linked, &mut effect)
        .unwrap();
    assert_eq!(session.rebuilds, epoch + 1);
    assert!(session.scene_scale.is_none());
    assert_eq!(session.rendered_scene_scale, [4096; 3]);
    // A newly constructed scene starts with unit node scales, including after
    // a completed (-1) effect. The day startup uses the production reset path.
    session.begin_original_day(&mut linked, load_day_tables(&exe).unwrap());
    assert!(session.scene_scale.is_none());
    assert_eq!(session.rendered_scene_scale, [4096; 3]);
}
