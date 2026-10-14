#[path = "support/actor_floor.rs"]
mod floor_support;
#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use floor_support::{array, scene};
use lsd_assets::{
    ActorBodyQueryCounts, ActorJournalBindings, ActorJournalBody, ActorJournalDispatcher,
    ActorJournalServices, EncounterEventHost, EncounterNotification, EncounterNotificationKind,
    StageSessionEvent,
    formats::{Mom, Tmd},
};
use lsd_core::{
    ActorAxis, ActorContact, ActorFloorQuery, BehaviorFault, BodyContactState, BodyWorldFrame,
    CollisionBounds, CollisionModel, CollisionPolicy, DayTables, DreamDay, DreamLinks,
    EntityScriptHost, FixedMatrix, LinkContext, LinkExecution, LinkPalette, MapLinkPolicy,
    ModelActor, Mood, PlayerControls, PsxMathTables, PsxRng, SceneContact, SceneGrid,
    SpawnLocation, WorldContact, move_actor_on_floor,
};
use serde_json::{Value, json};

fn bytes(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/floor-bypass-runtime.json")).unwrap()
}
fn tables() -> DayTables {
    DayTables {
        grids: [SceneGrid {
            columns: 1,
            rows: 3,
            mode: 0,
        }; 14],
        moods: std::array::from_fn(|_| vec![Mood([0, 0]); 3]),
        special_days: [0; 42],
        special_marker: Mood([-4, 6]),
        colors: [0; 9],
        time_limits: [60; 14],
        save_magic: 0,
        start_rotation: [65536; 3],
    }
}
fn receiver(tmd: &Tmd) -> CollisionModel {
    let object = &tmd.objects[0];
    CollisionModel {
        bounds: CollisionBounds::from_points(
            &object
                .vertices
                .iter()
                .map(|vertex| vertex.xyz)
                .collect::<Vec<_>>(),
        ),
        faces: object.collision_faces(),
    }
}
struct Host<'a, 'services> {
    math: &'a PsxMathTables,
    model: &'a CollisionModel,
    world: BodyWorldFrame,
    events: &'a mut ActorJournalDispatcher<'services>,
    fields: [i8; 12],
    signals: Vec<i32>,
    journal: Vec<i32>,
    floor_calls: u32,
    clouds: Vec<Value>,
}
impl EntityScriptHost for Host<'_, '_> {
    fn movement_matrix(&mut self, angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault> {
        Ok(self.math.rotation(angles))
    }
    fn notify(&mut self, _rng: &mut PsxRng, _code: i32) -> Result<(), BehaviorFault> {
        unreachable!("actor identity is required")
    }
    fn notify_actor(
        &mut self,
        actor: &mut ModelActor,
        rng: &mut PsxRng,
        code: i32,
    ) -> Result<(), BehaviorFault> {
        self.signals.push(code);
        self.journal.push(code);
        let cloud = self
            .model
            .prepared_actor_cloud(
                self.world,
                code,
                actor.move_distance,
                actor.collision_margin,
            )
            .unwrap();
        self.clouds.push(json!(
            cloud.cloud.0.into_iter().flatten().collect::<Vec<_>>()
        ));
        let first = self.events.services.events.len();
        self.events.dispatch_actor_cloud(
            actor,
            EncounterNotification {
                serial: 1,
                entity: 17,
                kind: EncounterNotificationKind::Notify(code),
            },
            self.fields,
            cloud,
            rng,
        )?;
        // Outer notification entry precedes its synchronous recursive journal request.
        self.journal.extend(
            self.events.services.events[first..]
                .iter()
                .filter_map(|event| match event {
                    StageSessionEvent::ActorJournal { code: nested, .. } if *nested != code => {
                        Some(*nested)
                    }
                    _ => None,
                }),
        );
        Ok(())
    }
    fn actor_floor_query(&mut self, position: [i32; 3]) -> Result<ActorFloorQuery, BehaviorFault> {
        self.floor_calls += 1;
        self.events.actor_floor_query(position, self.math)
    }
    fn actor_floor_signal(
        &mut self,
        _actor: &mut ModelActor,
        code: i32,
    ) -> Result<(), BehaviorFault> {
        self.signals.push(code);
        Ok(())
    }
}
fn assert_state(actual: &Value, expected: &Value, label: &str) {
    if let (Some(actual), Some(expected)) = (actual.as_object(), expected.as_object()) {
        assert_eq!(actual.len(), expected.len(), "{label}");
        for (key, value) in actual {
            assert_state(value, &expected[key], &format!("{label}.{key}"));
        }
    } else {
        assert_eq!(actual, expected, "{label}");
    }
}
fn compare(
    data: &Value,
    graphs: &Value,
    model: &Mom,
    receiver: &CollisionModel,
    math: &PsxMathTables,
) {
    let link_data: Value =
        serde_json::from_str(include_str!("../../lsd-core/tests/fixtures/links.json")).unwrap();
    let catalog = support::catalog(&link_data["catalog"]);
    let tables = tables();
    let scenes: Vec<_> = graphs.as_array().unwrap().iter().map(scene).collect();
    for (index, case) in data["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let expected = &case["state"];
        let initial = &case["initial"];
        let graph = case["graph"].as_u64().unwrap() as usize;
        let scene = &scenes[graph];
        let mut actor = model.construct_actor().unwrap();
        actor.pose.position = array(&input["position"]);
        actor.animation.actor_coordinate_cache = initial["cache"].as_i64().unwrap() as i32;
        let fields = array::<12>(&input["fields"]).map(|v| v as i8);
        if let Some(contact) = initial["contact"].as_array() {
            let slot = contact[0].as_u64().unwrap() as usize;
            actor.body_contact.current = Some(ActorContact::Floor(SceneContact {
                slot,
                map_index: scene.slots.slots[slot].map_index,
                geometry: WorldContact {
                    cell: contact[1].as_u64().unwrap() as usize,
                    tile: contact[2].as_u64().unwrap() as usize,
                    model: 0,
                    position: actor.pose.position,
                },
            }));
        }
        let geometry = model.collision_model(&actor).unwrap().unwrap();
        let world = BodyWorldFrame {
            position: actor.pose.position,
            matrix: math.rotation(actor.pose.angles).scaled(actor.pose.scale),
        };
        let mut day = DreamDay {
            cinematic: [3, 4],
            ..Default::default()
        };
        let mut links = DreamLinks {
            execution: LinkExecution {
                state: input["link_state"].as_i64().unwrap() as i32,
                stage: 2,
                dream_timer: 123,
                flashback: false,
            },
            spawn: SpawnLocation::default(),
            source_transform: None,
            target_transform: None,
            auto_action: None,
            auto_step: 0,
            auto_target: SpawnLocation::default(),
            time_limit: 3600,
            default_teleport_transform: false,
        };
        let mut controls = PlayerControls::default();
        let mut context = LinkContext::default();
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
                current_key: SpawnLocation::default(),
                attached: true,
                pending: &mut pending,
                events: &mut events,
                body_contact: &mut body_contact,
            },
        );
        services.floor_scene = Some(scene);
        services.body_query = Some(ActorJournalBody {
            model: receiver,
            position: [1024, 0, 1024],
            inverse: FixedMatrix::IDENTITY,
            policy: CollisionPolicy {
                ignore_low_midpoint: input["filter"].as_i64().unwrap() != 0,
            },
            math,
            counts: &mut counts,
        });
        let mut dispatcher = ActorJournalDispatcher {
            day: &mut day,
            tables: &tables,
            links: &mut links,
            controls: &mut controls,
            services,
        };
        let mut host = Host {
            math,
            model: &geometry,
            world,
            events: &mut dispatcher,
            fields,
            signals: vec![],
            journal: vec![],
            floor_calls: 0,
            clouds: vec![],
        };
        move_actor_on_floor(
            &mut actor,
            if input["axis"] == "Z" {
                ActorAxis::Z
            } else {
                ActorAxis::X
            },
            input["amount"].as_i64().unwrap() as i32,
            input["notify"].as_bool().unwrap(),
            &mut PsxRng::new(1),
            &mut host,
        )
        .unwrap();
        let contact = actor.body_contact.current.map(|contact| match contact {
            ActorContact::Peer(_) => json!("peer"),
            ActorContact::Floor(contact) => {
                json!([contact.slot, contact.geometry.cell, contact.geometry.tile])
            }
        });
        let actual = json!({"pose":{"position":actor.pose.position,"angles":actor.pose.angles,"scale":actor.pose.scale},
            "cache":actor.animation.actor_coordinate_cache,"move_distance":actor.move_distance,"contact":contact,
            "signals":host.signals,"journal":host.journal,"floor_calls":host.floor_calls,"prepared_clouds":host.clouds,
            "body_calls":host.events.services.body_query.as_ref().unwrap().counts.queries,
            "player_contact":host.events.services.body_contact.current.is_some(),
            "cinematic":host.events.day.cinematic,"timer":host.events.links.execution.dream_timer,
            "link_state":host.events.links.execution.state,"audio":sounds});
        assert_state(
            &actual,
            expected,
            &format!("coupled floor case {index} {input}"),
        );
        assert!(pending.is_empty());
    }
}
#[test]
fn original_player_acknowledgement_bypasses_floor_only_after_actor_contact() {
    let value = fixture();
    let model = Mom::parse(&bytes(&value["owned_sender_mom_hex"])).unwrap();
    let receiver = receiver(&Tmd::parse(&bytes(&value["owned_receiver_tmd_hex"])).unwrap());
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            value["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new(std::array::from_fn(|i| {
            value["owned_sqrt"][i].as_i64().unwrap() as i16
        })),
    };
    compare(&value, &value["graphs"], &model, &receiver, &math);
}
#[test]
#[ignore = "Requires original actor/player meshes and private coupled floor observations"]
fn original_model_contact_and_floor_bypass_match_native_cpu() {
    let owned = fixture();
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let value: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_FLOOR_BYPASS").expect("Set LSD_NATIVE_FLOOR_BYPASS"),
        )
        .unwrap(),
    )
    .unwrap();
    let binding = &value["sender_input"];
    let source = std::fs::read(files.join(binding["path"].as_str().unwrap())).unwrap();
    let at = binding["offset"].as_u64().unwrap() as usize;
    let end = at + binding["bytes"].as_u64().unwrap() as usize;
    assert_eq!(
        lsd_assets::sha256_hex(&source),
        binding["source_sha256"].as_str().unwrap()
    );
    assert_eq!(
        lsd_assets::sha256_hex(&source[at..end]),
        binding["mom_sha256"].as_str().unwrap()
    );
    let player = std::fs::read(files.join(value["receiver_path"].as_str().unwrap())).unwrap();
    assert_eq!(
        lsd_assets::sha256_hex(&player),
        value["receiver_sha256"].as_str().unwrap()
    );
    let math =
        lsd_assets::load_math_tables(&std::fs::read(files.join("SLPS_015.56")).unwrap()).unwrap();
    compare(
        &value,
        &owned["graphs"],
        &Mom::parse(&source[at..end]).unwrap(),
        &receiver(&Tmd::parse(&player).unwrap()),
        &math,
    );
}
