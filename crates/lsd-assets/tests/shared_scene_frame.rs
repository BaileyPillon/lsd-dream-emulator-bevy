use lsd_assets::{
    SceneClockMember, SceneFrameVisit, StageLinkSession, StageSessionEvent, StreamedStage,
    formats::Tmd, load_day_tables, load_effect_palette, load_encounter_tables,
    load_entity_properties, load_generation_effect_tables, load_generation_object_tables,
    load_generation_paths, load_generation_tables, load_link_tables, load_math_tables,
};
use lsd_core::{
    ChunkPlayer, CollisionBounds, EntityTarget, LinkedPlayer, MapEffect, PadState, PlayerControls,
    SceneLinkTables,
};
fn role(member: SceneClockMember) -> &'static str {
    match member {
        SceneClockMember::Actor(_) => "actor",
        SceneClockMember::ActorEffect(_) => "actor-effect",
        SceneClockMember::Effect => "effect",
        SceneClockMember::Map => "map",
        SceneClockMember::Stream => "stream",
        SceneClockMember::Renderer => "renderer",
        SceneClockMember::Session => "session",
        SceneClockMember::Player => "player",
    }
}
#[test]
#[ignore = "Requires the original executable and stage/model/texture corpus"]
fn loaded_shared_clock_birth_render_input_waiting_and_journal_order() {
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
    session.begin_original_day(&mut linked, load_day_tables(&exe).unwrap());
    session
        .tick(&mut stage, &mut player, &mut linked, &mut effect)
        .unwrap();

    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    let encounters = session.encounters.as_ref().unwrap();
    assert!(!encounters.actors.is_empty());
    assert!(
        encounters
            .actors
            .iter()
            .all(|actor| actor.actor.animation.ticks == 0)
    );
    assert!(
        !session
            .frame_trace
            .iter()
            .any(|v| matches!(v, SceneFrameVisit::Clock(SceneClockMember::Actor(_))))
    );
    // Independent original-emulator graph establishes the static subscriber order.
    let checks: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/frame-order-checks.json")).unwrap();
    let expected = checks["original_emulator_observation"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] != "actor")
        .map(|m| m["role"].as_str().unwrap())
        .collect::<Vec<_>>();
    let actual = session
        .frame_trace
        .iter()
        .filter_map(|visit| match visit {
            SceneFrameVisit::Clock(member)
                if !matches!(
                    member,
                    SceneClockMember::Actor(_) | SceneClockMember::Effect
                ) =>
            {
                Some(role(*member))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(
        session.frame_trace[session.frame_trace.len() - 2..],
        [SceneFrameVisit::Pad, SceneFrameVisit::Handoff]
    );
    assert_eq!(session.encounter_frame.phase, 1);
    assert_eq!(session.session_ticks, 1);
    assert_eq!(linked.links.execution.dream_timer, 1);
    let timer = linked.links.execution.dream_timer;
    session.encounter_frame.waiting = 1;
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    assert_eq!(session.encounter_frame.phase, 1);
    assert_eq!(linked.links.execution.dream_timer, timer);
    assert!(
        session
            .encounters
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .all(|a| a.actor.animation.ticks == 0)
    );
    session.encounter_frame.waiting = 0;
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    assert!(
        session
            .encounters
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .all(|a| a.actor.animation.ticks == 1)
    );

    // Trigger the imported Futon with the previously native-compared target placement.
    let futon = session
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .find(|a| a.entity == 5)
        .unwrap();
    let forward = i32::from(futon.properties.fields[11])
        .abs()
        .wrapping_shl(11);
    let offset = EntityTarget {
        position: [0; 3],
        local_y: 0,
        matrix: math.rotation(player.controls.angles),
        eye: player.controls.eye,
        target: player.controls.target,
    }
    .resolve(forward)
    .unwrap();
    player.position =
        std::array::from_fn(|axis| futon.actor.pose.position[axis].wrapping_sub(offset[axis]));
    linked
        .refresh_key(&stage.collision, player.position)
        .unwrap();
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    assert!(session.events.iter().any(|event| matches!(
        event,
        StageSessionEvent::ActorJournal {
            entity: 5,
            code: 9,
            applied: true
        }
    )));
    assert_eq!(session.day.as_ref().unwrap().instance_score, -50000);

    // The renderer snapshot precedes the player's look callback; input is polled last.
    player.controls.input_locked = false;
    session
        .step_frame(
            &mut stage,
            &mut linked,
            &mut player,
            &math,
            &mut effect,
            0x10,
        )
        .unwrap();
    session
        .step_frame(
            &mut stage,
            &mut linked,
            &mut player,
            &math,
            &mut effect,
            0x10,
        )
        .unwrap();
    let previous_target = player.controls.target;
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    assert_eq!(
        session.rendered_player.as_ref().unwrap().controls.target,
        previous_target
    );
    assert_ne!(player.controls.target, previous_target);
    println!(
        "shared_frame_loaded_births_delayed=true waiting_holds_player_and_actor_ticks=true journal_survives_handoff=true render_precedes_player_view=true"
    );
}
