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
fn loaded_triangle_head_stops_and_resumes_player_callbacks() {
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

    let library =
        lsd_assets::formats::Lbd::parse(&std::fs::read(files.join("CDI/STG05/M018.LBD")).unwrap())
            .unwrap()
            .model_library
            .unwrap();
    let model = std::sync::Arc::new(library.models[0].clone());
    let properties = load_entity_properties(&exe).unwrap()[62];
    let fields = properties.fields;
    let mut actor = model.construct_actor().unwrap();
    let mut behavior = lsd_core::EntityBehavior::construct(
        62,
        lsd_core::NpcScript::TriangleHead,
        fields,
        &mut actor,
        &mut session.encounter_audio,
    );
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    behavior.state = 11;
    behavior.ticks = 300;
    actor.pose.position = [player.position[0] + 2048, 2000, player.position[2]];
    let collision = model.collision_model(&actor).unwrap();
    let encounters = session.encounters.as_mut().unwrap();
    let serial = encounters.add_actor(lsd_assets::SceneActor {
        serial: 0,
        slot: 0,
        map: 0,
        entity: 62,
        model: 0,
        resource: model,
        actor,
        collision,
        properties,
        behavior_active: true,
        behavior_pending: false,
        behavior,
    });

    player.controls.paused = 0;
    player.controls.input_locked = false;
    player.controls.move_state = 1;
    player.controls.move_step = 0;
    player.controls.move_direction = 1;
    player.controls.previous_move_direction = 1;
    player.controls.screen_shake = false;
    player.controls.look_state = 1;
    player.controls.look_vertical = 100;
    let stopped_position = player.position;
    session
        .step_frame(
            &mut stage,
            &mut linked,
            &mut player,
            &math,
            &mut effect,
            0x1000,
        )
        .unwrap();
    assert_eq!(linked.map.move_callback, 0);
    assert_eq!(linked.view_callback, 1);
    assert_eq!(player.position, stopped_position);
    assert_ne!(player.controls.look_vertical, 100);
    session
        .encounters
        .as_mut()
        .unwrap()
        .actors
        .iter_mut()
        .find(|actor| actor.serial == serial)
        .unwrap()
        .behavior
        .ticks = 500;
    player.controls.move_state = 1;
    player.controls.move_step = 0;
    session
        .step_frame(
            &mut stage,
            &mut linked,
            &mut player,
            &math,
            &mut effect,
            0x1000,
        )
        .unwrap();
    assert_eq!(linked.map.move_callback, 1);
    assert_eq!(linked.view_callback, 1);
    assert_ne!(
        [player.position[0], player.position[2]],
        [stopped_position[0], stopped_position[2]],
        "resumed movement must run in the same shared clock"
    );
    assert!(
        session
            .encounters
            .as_ref()
            .unwrap()
            .pending_notifications
            .is_empty()
    );
    println!(
        "triangle_head_same_frame_stop_resume=true before={stopped_position:?} after={:?}",
        player.position
    );
    // Supply the already compared odd/odd branch while retaining the imported
    // model and the real shared scene/journal services.
    {
        let actor = session
            .encounters
            .as_mut()
            .unwrap()
            .actors
            .iter_mut()
            .find(|actor| actor.serial == serial)
            .unwrap();
        actor.behavior.state = 0;
        actor.behavior.ticks = 0;
        actor.behavior.triggered = 1;
        actor.actor.animation.ticks = 0;
        actor.actor.pose.position = player.position;
        actor.actor.pose.angles = [0; 3];
    }
    linked.rng = lsd_core::PsxRng::new(3);
    linked.map.move_callback = 0;
    player.controls.move_state = 0;
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    let scale = session.scene_scale.as_ref().unwrap();
    assert_eq!((scale.remaining, scale.scale), (63, [4096, 4032, 4096]));
    assert_eq!(session.rendered_scene_scale, [4096; 3]);
    for _ in 0..64 {
        session
            .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
            .unwrap();
    }
    let scale = session.scene_scale.as_ref().unwrap();
    assert_eq!((scale.remaining, scale.scale), (-1, [4096, 0, 4096]));
    assert_eq!(session.rendered_scene_scale, [4096, 0, 4096]);
    println!("triangle_head_shared_negative_scale_completed=true original_model=true");
}
