#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use lsd_assets::{
    ActorBodyQueryCounts, ActorJournalBindings, ActorJournalBody, ActorJournalDispatcher,
    ActorJournalServices, EncounterEventHost, EncounterNotification, EncounterNotificationKind,
    formats::{Mom, Tmd},
    sha256_hex,
};
use lsd_core::{
    BodyContactState, CollisionBounds, CollisionModel, CollisionPolicy, DayTables, DreamDay,
    DreamLinks, FixedMatrix, LinkContext, LinkExecution, LinkPalette, MapLinkPolicy, Mood,
    PlayerControls, PsxMathTables, PsxRng, SceneGrid, SpawnLocation,
};
use serde_json::{Value, json};

fn decode_hex(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn receiver(tmd: &Tmd) -> CollisionModel {
    let object = &tmd.objects[0];
    CollisionModel {
        bounds: CollisionBounds::from_points(
            &object.vertices.iter().map(|v| v.xyz).collect::<Vec<_>>(),
        ),
        faces: object.collision_faces(),
    }
}
fn compare(fixture: &Value, model: &Mom, receiver: &CollisionModel, math: &PsxMathTables) -> usize {
    let catalog = support::catalog(&fixture["catalog"]);
    let tables = DayTables {
        grids: [SceneGrid {
            columns: 2,
            rows: 2,
            mode: 0,
        }; 14],
        moods: std::array::from_fn(|_| vec![Mood([0, 0]); 4]),
        special_days: [0; 42],
        special_marker: Mood([-4, 6]),
        colors: [0; 9],
        time_limits: [60; 14],
        save_magic: 0,
        start_rotation: [65536; 3],
    };
    let mut contacts = 0;
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let initial = &case["initial"];
        let expected = &case["expected"];
        let mut actor = model.construct_actor().unwrap();
        actor.pose.position =
            std::array::from_fn(|i| input["position"][i].as_i64().unwrap() as i32);
        actor.move_distance = input["distance"].as_i64().unwrap() as i16;
        let geometry = model.collision_model(&actor).unwrap().unwrap();
        let code = input["code"].as_i64().unwrap() as i32;
        let cloud = geometry
            .prepared_cloud(
                FixedMatrix::IDENTITY,
                matches!(code, 6..=8).then_some((code == 7, actor.move_distance)),
            )
            .unwrap();
        let flat = cloud.0.into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(json!(flat), case["prepared_cloud"], "cloud {number}");
        let mut day = DreamDay {
            cinematic: [3, 4],
            ..Default::default()
        };
        let key = support::location(&initial["spawn"]);
        let mut links = DreamLinks {
            execution: LinkExecution {
                state: input["state"].as_i64().unwrap() as i32,
                stage: 2,
                dream_timer: 123,
                flashback: false,
            },
            spawn: key,
            source_transform: None,
            target_transform: None,
            auto_action: None,
            auto_step: 0,
            auto_target: SpawnLocation::default(),
            time_limit: 3600,
            default_teleport_transform: false,
        };
        let mut context = LinkContext {
            dynamic_penalty: 17,
            ..Default::default()
        };
        let mut map = MapLinkPolicy {
            state: 0,
            tick: 0,
            location: 0,
            move_callback: 1,
            reset_view_pending: false,
            view_active: false,
        };
        let mut commands = vec![];
        let mut sounds = vec![];
        let mut pending = vec![];
        let mut events = vec![];
        let mut body_contact = BodyContactState { current: None };
        let mut counts = ActorBodyQueryCounts::default();
        let mut services = ActorJournalServices::new(
            &catalog,
            ActorJournalBindings {
                context: &mut context,
                map: &mut map,
                palette: LinkPalette {
                    dream: 0,
                    flashback: 0,
                },
                commands: &mut commands,
                sounds: &mut sounds,
                current_key: key,
                attached: true,
                pending: &mut pending,
                events: &mut events,
                body_contact: &mut body_contact,
            },
        );
        services.body_query = Some(ActorJournalBody {
            model: receiver,
            position: [0; 3],
            inverse: FixedMatrix::IDENTITY,
            policy: CollisionPolicy {
                ignore_low_midpoint: input["filter"].as_i64().unwrap() != 0,
            },
            math,
            counts: &mut counts,
        });
        let mut controls = PlayerControls::default();
        let mut rng = PsxRng::new(1);
        let mut dispatcher = ActorJournalDispatcher {
            day: &mut day,
            tables: &tables,
            links: &mut links,
            controls: &mut controls,
            services,
        };
        dispatcher
            .dispatch_actor_cloud(
                &mut actor,
                EncounterNotification {
                    serial: 1,
                    entity: 17,
                    kind: EncounterNotificationKind::Notify(code),
                },
                [
                    0,
                    0,
                    2,
                    0,
                    0,
                    1,
                    1,
                    input["link_stage"].as_i64().unwrap() as i8,
                    input["video"].as_i64().unwrap() as i8,
                    1,
                    10,
                    0,
                ],
                cloud,
                &mut rng,
            )
            .unwrap();
        assert!(
            pending.is_empty(),
            "no deferred query/callback for case {number}"
        );
        assert_eq!(counts.queries, 1);
        assert_eq!(
            counts.contacts,
            u64::from(case["player_contact"].as_bool().unwrap())
        );
        contacts += counts.contacts as usize;
        let actual = json!({"player_contact":body_contact.current.is_some(),
            "actor_contact":actor.body_contact.current.is_some(),"state":links.execution.state,
            "stage":links.execution.stage,"timer":links.execution.dream_timer,"cinematic":day.cinematic,
            "spawn":[links.spawn.chunk,links.spawn.tile,links.spawn.position[0],links.spawn.position[1],links.spawn.position[2]],
            "penalty":context.dynamic_penalty,"rng":rng.state()});
        let expected = json!({"player_contact":case["player_contact"],"actor_contact":case["actor_contact"],
            "state":expected["state"],"stage":expected["stage"],"timer":expected["timer"],"cinematic":expected["cinematic"],
            "spawn":expected["spawn"],"penalty":expected["penalty"],"rng":expected["rng"]});
        assert_eq!(actual, expected, "pair {number}: {input}");
        assert!(case["cloud_cleared"].as_bool().unwrap());
        let body_index = events
            .iter()
            .position(|e| matches!(e, lsd_assets::StageSessionEvent::ActorBodyQuery { .. }))
            .unwrap();
        assert!(events[..body_index].iter().all(|e| matches!(
            e,
            lsd_assets::StageSessionEvent::ActorJournal { code: 10..=12, .. }
        )));
    }
    contacts
}
#[test]
fn original_actor_player_geometry_contact_and_journal_consequences_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/actor-player-query.json")).unwrap();
    let model = Mom::parse(&decode_hex(&fixture["owned_sender_mom_hex"])).unwrap();
    let tmd = Tmd::parse(&decode_hex(&fixture["owned_receiver_tmd_hex"])).unwrap();
    let mut math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    math.sin_cos[0] = 4096 << 16;
    for (i, value) in math.sqrt_window[320..].iter_mut().enumerate() {
        *value = (((i + 64) as f64).sqrt() * 512.0) as i16;
    }
    assert_eq!(compare(&fixture, &model, &receiver(&tmd), &math), 40);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 300);
}
#[test]
#[ignore = "Requires original model/player corpus and private contact observations"]
fn canonical_yokai_player_mesh_contacts_match_original_cpu() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ACTOR_PLAYER").expect("Set LSD_NATIVE_ACTOR_PLAYER"),
        )
        .unwrap(),
    )
    .unwrap();
    let source = &fixture["sender_input"];
    let relative = std::path::Path::new(source["path"].as_str().unwrap());
    assert!(
        relative
            .components()
            .all(|p| matches!(p, std::path::Component::Normal(_)))
    );
    let data = std::fs::read(files.join(relative)).unwrap();
    assert_eq!(sha256_hex(&data), source["source_sha256"].as_str().unwrap());
    let offset = source["offset"].as_u64().unwrap() as usize;
    let mom = &data[offset..offset + source["bytes"].as_u64().unwrap() as usize];
    assert_eq!(sha256_hex(mom), source["mom_sha256"].as_str().unwrap());
    let model = Mom::parse(mom).unwrap();
    let data = std::fs::read(files.join("CDI/ETC/DREAME5.TMD")).unwrap();
    assert_eq!(fixture["receiver_path"], "CDI/ETC/DREAME5.TMD");
    assert_eq!(
        sha256_hex(&data),
        fixture["receiver_sha256"].as_str().unwrap()
    );
    let tmd = Tmd::parse(&data).unwrap();
    let math =
        lsd_assets::load_math_tables(&std::fs::read(files.join("SLPS_015.56")).unwrap()).unwrap();
    assert_eq!(compare(&fixture, &model, &receiver(&tmd), &math), 4);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 20);
}
