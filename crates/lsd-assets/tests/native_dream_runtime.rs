use lsd_assets::{
    StageLinkSession, StreamedStage, formats::Tmd, load_day_tables, load_effect_palette,
    load_link_tables, load_math_tables,
};
use lsd_core::{
    ChunkPlayer, CollisionBounds, LinkedPlayer, MapCompletionCommand, MapEffect, PadState,
    PlayerControls, SceneLinkTables,
};
use std::path::PathBuf;

#[test]
#[ignore = "Requires locally extracted original maps, models and executable"]
fn original_first_day_starts_at_its_spawn_expires_and_commits_the_next_day() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let original = load_link_tables(&executable).unwrap();
    let content = load_day_tables(&executable).unwrap();
    let model = Tmd::parse(&std::fs::read(files.join("CDI/ETC/DREAMER.TMD")).unwrap()).unwrap();
    let bounds = CollisionBounds::from_points(
        &model.objects[0]
            .vertices
            .iter()
            .map(|value| value.xyz)
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
    let mut session = StageLinkSession::default();
    session.begin_original_day(&mut linked, content);
    assert!(
        session
            .tick(&mut stage, &mut player, &mut linked, &mut effect)
            .unwrap()
    );
    let day = session.day.as_ref().unwrap();
    assert_eq!(day.day, 0);
    assert_eq!(day.stored_day, 0);
    assert_eq!(day.cinematic[1], -1);
    assert_eq!(linked.keys.current.chunk, linked.links.spawn.chunk);
    assert_eq!(linked.keys.current.tile, linked.links.spawn.tile);
    let original_limit = linked.links.time_limit;
    assert!(original_limit > 0);
    let mut timeout_at = None;
    let math = load_math_tables(&executable).unwrap();
    for tick in 1..original_limit + 200 {
        session
            .tick(&mut stage, &mut player, &mut linked, &mut effect)
            .unwrap();
        if session.finished.is_some() {
            break;
        }
        for event in effect.tick(&mut linked.map, &mut player.controls).unwrap() {
            if let MapCompletionCommand::Notify(code) = event {
                session.receive(code);
            }
        }
        let ready = std::array::from_fn(|i| stage.protocol.slots[i].ready);
        linked
            .step_with_clock(
                &mut player,
                &stage.collision,
                &math,
                ready,
                0,
                Some((
                    session.day.as_mut().unwrap(),
                    session.day_tables.as_ref().unwrap(),
                )),
            )
            .unwrap();
        for command in linked.commands.clone() {
            for event in effect.consume(command).unwrap() {
                if let MapCompletionCommand::Notify(code) = event {
                    session.receive(code);
                }
            }
        }
        if linked.map.state == 4 && timeout_at.is_none() {
            timeout_at = Some(tick);
        }
    }
    assert_eq!(timeout_at, Some(original_limit + 1));
    assert_eq!(session.finished, Some(1));
    let day = session.day.as_ref().unwrap();
    assert_eq!(day.day, 1);
    assert_eq!(day.year, 0);
    assert_eq!(day.stored_day, 0);
    assert_eq!(day.history[0], day.chart().unwrap());
    assert_eq!(day.total_unlock_score, 0);
    assert_eq!(session.pending_day_event, None);
    assert!(player.controls.input_locked);
    let retained_rng = linked.rng.state();
    session.begin_next_dream().unwrap();
    assert_eq!(linked.rng.state(), retained_rng);
    session
        .tick(&mut stage, &mut player, &mut linked, &mut effect)
        .unwrap();
    assert_eq!(session.day.as_ref().unwrap().stored_day, 1);
}
