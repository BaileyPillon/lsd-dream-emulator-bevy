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
#[ignore = "Requires original stages, player, twelve NPC models/properties and math"]
fn loaded_additional_actions_run_with_original_resources_and_scene_services() {
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

    let configs = [
        ("entity_elephant_small", "CDI/STG01/M004.LBD", 0, 109),
        ("entity_maiko_bridge", "CDI/STG02/N1/M022.LBD", 0, 110),
        (
            "entity_question_mark_penguin",
            "CDI/STG04/N1/M024.LBD",
            1,
            113,
        ),
        (
            "entity_standing_penguin_flock",
            "CDI/STG04/N1/M024.LBD",
            0,
            114,
        ),
        ("entity_ship", "CDI/STG05/M010.LBD", 0, 64),
        ("entity_triangle_head_giant", "CDI/STG05/M018.LBD", 0, 117),
        ("entity_maiko_palace", "CDI/STG07/M001.LBD", 0, 65),
        ("entity_acrobats", "CDI/STG07/M002.LBD", 0, 66),
        ("entity_light", "CDI/STG07/M004.LBD", 0, 67),
        ("entity_ring", "CDI/STG11/M005.LBD", 0, 70),
        ("entity_hoop_girl", "CDI/STG12/M009.LBD", 0, 71),
        ("entity_hoop_girl_giant", "CDI/STG12/M009.LBD", 0, 108),
    ];
    for (name, path, model_index, entity) in configs {
        let map =
            lsd_assets::formats::Lbd::parse(&std::fs::read(files.join(path)).unwrap()).unwrap();
        let model = std::sync::Arc::new(map.model_library.unwrap().models[model_index].clone());
        let properties = load_entity_properties(&exe).unwrap()[entity];
        let script = lsd_core::NpcScript::from_reference_address(properties.callback);
        assert!(!script.pending(), "{name}");
        let mut actor = model.construct_actor().unwrap();
        let mut behavior = lsd_core::EntityBehavior::construct(
            entity as u8,
            script,
            properties.fields,
            &mut actor,
            &mut session.encounter_audio,
        );
        behavior.enable_link(&mut actor);
        behavior.start_behavior(&mut actor);
        behavior.triggered = 1;
        let target = lsd_core::EntityTarget {
            position: player.position,
            local_y: player.position[1],
            matrix: math.rotation(player.controls.angles),
            eye: player.controls.eye,
            target: player.controls.target,
        };
        let range = i32::from(properties.fields[11]).abs().max(1) * 2048;
        actor.pose.position = target.resolve(range).unwrap();
        if (-9..=-1).contains(&properties.fields[2]) {
            actor.pose.position[1] -= i32::from(properties.fields[2]) * 1024;
        }
        actor.pose.angles = [0; 3];
        let collision = model.collision_model(&actor).unwrap();
        let serial = session
            .encounters
            .as_mut()
            .unwrap()
            .add_actor(lsd_assets::SceneActor {
                serial: 0,
                slot: 0,
                map: 0,
                entity: entity as u8,
                model: model_index as i8,
                resource: model,
                actor,
                collision,
                properties,
                behavior_active: true,
                behavior_pending: false,
                behavior,
            });
        linked.rng = lsd_core::PsxRng::new(13);
        session
            .tick_encounter_actor(
                (serial, lsd_core::FrameSignal::Tick),
                &stage.collision,
                &mut linked,
                &mut player,
                &math,
                &mut effect,
            )
            .unwrap();
        let item = session
            .encounters
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .find(|actor| actor.serial == serial)
            .unwrap();
        assert!(!item.behavior_pending, "{name}");
        assert_ne!(item.behavior.ticks, 0, "{name} must execute the action");
        match script {
            lsd_core::NpcScript::GiantHoopGirl | lsd_core::NpcScript::GiantTriangleHead => {
                assert_eq!(item.actor.pose.scale, [24576; 3])
            }
            lsd_core::NpcScript::SmallElephant => assert_eq!(item.actor.pose.scale, [2048; 3]),
            lsd_core::NpcScript::BridgeMaiko => {
                assert_eq!(item.actor.pose.scale, [1638; 3]);
                assert!(!item.actor.animation.enabled);
            }
            _ => {}
        }
        println!(
            "loaded_action={name} executed=true ticks={} position={:?}",
            item.behavior.ticks, item.actor.pose.position
        );
    }
    assert_eq!(
        session
            .encounters
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .filter(|actor| configs
                .iter()
                .any(|(_, _, _, id)| *id == usize::from(actor.entity)))
            .count(),
        12
    );
    for entity in [71, 108] {
        let item = session
            .encounters
            .as_mut()
            .unwrap()
            .actors
            .iter_mut()
            .find(|actor| actor.entity == entity)
            .unwrap();
        item.behavior.context.words[1] = 0;
        item.actor.pose.angles = [0; 3];
        let before = item.actor.pose.position;
        let serial = item.serial;
        linked.rng = lsd_core::PsxRng::new(5);
        session
            .tick_encounter_actor(
                (serial, lsd_core::FrameSignal::Tick),
                &stage.collision,
                &mut linked,
                &mut player,
                &math,
                &mut effect,
            )
            .unwrap();
        let after = session
            .encounters
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .find(|actor| actor.serial == serial)
            .unwrap()
            .actor
            .pose
            .position;
        assert_eq!(
            after,
            [before[0] - 14336, before[1], before[2] - 30],
            "original signed-halfword movement for entity {entity}"
        );
    }
    println!("loaded_hoop_movement_51200_truncated_to=-14336 ordinary_and_giant=true");
    let ship = session
        .encounters
        .as_mut()
        .unwrap()
        .actors
        .iter_mut()
        .find(|actor| actor.entity == 64)
        .unwrap();
    ship.behavior.context.words[1] = 19;
    let ship_before = ship.actor.pose.position;
    for _ in 0..3 {
        session
            .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
            .unwrap();
    }
    let actors = &session.encounters.as_ref().unwrap().actors;
    assert_eq!(
        actors
            .iter()
            .filter(|actor| configs
                .iter()
                .any(|(_, _, _, id)| *id == usize::from(actor.entity)))
            .count(),
        12
    );
    let ship = actors.iter().find(|actor| actor.entity == 64).unwrap();
    assert_eq!(
        ship.actor.pose.position,
        [ship_before[0], ship_before[1], ship_before[2] - 30]
    );
    assert_eq!(ship.behavior.context.words[1], 22);
    assert!(session.rendered_player.is_some());
    assert!(
        session
            .encounters
            .as_ref()
            .unwrap()
            .pending_notifications
            .is_empty()
    );
    println!(
        "loaded_additional_actions_shared_clock=true ship_three_steps=30 renderer_snapshot=true"
    );
    println!(
        "loaded_additional_action_models=12 original_properties=true production_scene_services=true"
    );
}
