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
fn loaded_gray_flash_reset_waiting_birth_delay_and_completion() {
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
    let records = std::array::from_fn(|i| lsd_core::FlashbackRecord {
        day: i as i32 + 10,
        kind: 1,
        ..Default::default()
    });
    {
        let day = session.day.as_mut().unwrap();
        day.flashbacks = records;
        day.flashback_count = 10;
        day.flashback_index = 7;
    }
    let target = EntityTarget {
        position: player.position,
        local_y: player.position[1],
        matrix: math.rotation(player.controls.angles),
        eye: player.controls.eye,
        target: player.controls.target,
    };
    let supplied_position = target.resolve(2048).unwrap();
    let serial;
    {
        let encounters = session.encounters.as_mut().unwrap();
        encounters.set_encounter_player(player.position, target.matrix);
        encounters
            .activate_gray_man(&mut session.encounter_audio)
            .unwrap();
        let gray = encounters
            .actors
            .iter_mut()
            .find(|actor| actor.entity == 98)
            .unwrap();
        gray.actor.translate(supplied_position, true);
        serial = gray.serial;
    }
    session.encounter_frame.waiting = 1;
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    let gray = session
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .find(|a| a.serial == serial)
        .unwrap();
    assert_eq!(gray.behavior.triggered, 1);
    assert!(gray.behavior.effect.is_none());
    assert_eq!(gray.actor.pose.position, supplied_position);
    assert_eq!(session.day.as_ref().unwrap().flashback_count, 10);
    // Interaction/journal work can replace a recall slot before the action runs.
    let retained_records = session.day.as_ref().unwrap().flashbacks;
    session.encounter_frame.waiting = 0;
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    let gray = session
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .find(|a| a.serial == serial)
        .unwrap();
    assert_eq!(gray.behavior.active, 0);
    assert_eq!(gray.behavior.link_enabled, 0);
    let flash = gray.behavior.effect.as_ref().unwrap();
    assert_eq!(flash.state.life, 25);
    assert_eq!(flash.state.color, [255; 3]);
    assert_eq!(
        flash.packet([320, 240]).unwrap().rectangle(),
        [0, 0, 320, 240]
    );
    assert!(
        !session
            .frame_trace
            .contains(&SceneFrameVisit::Clock(SceneClockMember::ActorEffect(
                serial
            )))
    );
    assert!(
        session
            .events
            .iter()
            .any(|v| matches!(v, StageSessionEvent::ActorFlashbackReset { entity: 98 }))
    );
    assert_eq!(session.day.as_ref().unwrap().flashback_count, 0);
    assert_eq!(session.day.as_ref().unwrap().flashback_index, 7);
    assert_eq!(session.day.as_ref().unwrap().flashbacks, retained_records);
    session
        .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
        .unwrap();
    assert_eq!(
        session.frame_trace[0],
        SceneFrameVisit::Clock(SceneClockMember::ActorEffect(serial))
    );
    let flash = session
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .find(|a| a.serial == serial)
        .unwrap()
        .behavior
        .effect
        .as_ref()
        .unwrap();
    assert_eq!(flash.state.life, 24);
    assert_eq!(flash.state.color, [245; 3]);
    for _ in 0..25 {
        session
            .step_frame(&mut stage, &mut linked, &mut player, &math, &mut effect, 0)
            .unwrap();
    }
    let flash = session
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .find(|a| a.serial == serial)
        .unwrap()
        .behavior
        .effect
        .as_ref()
        .unwrap();
    assert_eq!(flash.state.state, 0);
    assert!(!flash.clock_attached);
    assert!(flash.owner_bound);
    assert!(flash.packet([320, 240]).is_none());
    assert!(
        !session
            .frame_members
            .contains(SceneClockMember::ActorEffect(serial))
    );
    assert_eq!(session.day.as_ref().unwrap().flashback_index, 7);
    println!(
        "loaded_gray_flash_reset=true waiting_holds_action=true effect_birth_delay=true completion_detaches_clock=true records_and_index_retained=true role_after_completion={}",
        role(session.frame_members.order()[0])
    );
}
