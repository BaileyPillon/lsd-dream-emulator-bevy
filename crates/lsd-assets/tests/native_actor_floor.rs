use lsd_assets::formats::Mom;
use lsd_core::{
    ActorAxis, ActorContact, ActorFloorQuery, BehaviorFault, CollisionBounds, CollisionChunk,
    CollisionFace, CollisionModel, CollisionPolicy, CollisionTile, EntityScriptHost, FixedMatrix,
    ModelActor, PsxMathTables, PsxRng, SceneCollision, SceneContact, SceneGrid, SceneSlot,
    SceneSlots, WorldContact, move_actor_on_floor,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/actor-floor-runtime.json")).unwrap()
}
fn array<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
fn scene(value: &Value) -> SceneCollision {
    let layout = array::<3>(&value["layout"]);
    let slots = &value["slots"];
    SceneCollision {
        slots: SceneSlots {
            grid: SceneGrid {
                columns: layout[0] as i16,
                rows: layout[1] as i16,
                mode: layout[2],
            },
            slots: std::array::from_fn(|i| SceneSlot {
                label: slots[i]["label"].as_i64().unwrap() as i16,
                map_index: slots[i]["map"].as_i64().unwrap() as i16,
                origin: array(&slots[i]["origin"]),
                ready: slots[i]["ready"].as_bool().unwrap(),
            }),
        },
        chunks: std::array::from_fn(|slot| {
            let mut models = vec![];
            let mut cells = vec![vec![]; 400];
            for (cell, records) in slots[slot]["cells"].as_object().unwrap() {
                for record in records.as_array().unwrap() {
                    let points: Vec<[i16; 3]> = record["points"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|point| array::<3>(point).map(|v| v as i16))
                        .collect();
                    let face = match points.len() {
                        3 => CollisionFace::Triangle(points.as_slice().try_into().unwrap()),
                        4 => CollisionFace::Quad(points.as_slice().try_into().unwrap()),
                        _ => panic!("invalid face"),
                    };
                    let model = models.len();
                    models.push(CollisionModel {
                        bounds: CollisionBounds::from_points(&points),
                        faces: vec![face],
                    });
                    cells[cell.parse::<usize>().unwrap()].push(CollisionTile {
                        model,
                        translation: array(&record["translation"]),
                        heading: record["heading"].as_i64().unwrap() as i16,
                        category: 0,
                        flags: record["flags"].as_u64().unwrap() as u8,
                    });
                }
            }
            Some(CollisionChunk {
                policy: CollisionPolicy::for_stage(0),
                models,
                cells,
            })
        }),
    }
}
struct Host<'a> {
    scene: &'a SceneCollision,
    math: &'a PsxMathTables,
    bound: bool,
    signals: Vec<i32>,
    journal: Vec<i32>,
}
impl EntityScriptHost for Host<'_> {
    fn movement_matrix(&mut self, angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault> {
        Ok(self.math.rotation(angles))
    }
    fn notify(&mut self, _rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault> {
        self.signals.push(code);
        self.journal.push(code);
        Ok(())
    }
    fn actor_floor_query(&mut self, position: [i32; 3]) -> Result<ActorFloorQuery, BehaviorFault> {
        if !self.bound {
            return Ok(ActorFloorQuery::Unlocated);
        }
        self.scene
            .actor_floor_query(position, self.math)
            .map_err(BehaviorFault::Scene)
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
fn state(actor: &ModelActor, host: &Host<'_>) -> Value {
    let contact = actor.body_contact.current.map(|contact| match contact {
        ActorContact::Peer(_) => json!("peer"),
        ActorContact::Floor(contact) => {
            json!([contact.slot, contact.geometry.cell, contact.geometry.tile])
        }
    });
    json!({"pose":{"position":actor.pose.position,"angles":actor.pose.angles,"scale":actor.pose.scale},
        "cache":actor.animation.actor_coordinate_cache,"move_distance":actor.move_distance,
        "contact":contact,"signals":host.signals,"journal":host.journal})
}
fn assert_state(actual: &Value, expected: &Value, label: &str) {
    if let (Some(actual), Some(expected)) = (actual.as_object(), expected.as_object()) {
        assert_eq!(actual.len(), expected.len(), "{label}");
        for (name, value) in actual {
            assert_state(value, &expected[name], &format!("{label}.{name}"));
        }
    } else {
        assert_eq!(actual, expected, "{label}");
    }
}
#[test]
fn original_actor_floor_wrapper_lookup_correction_and_local_signals_match() {
    let value = fixture();
    let hex = value["owned_model_hex"].as_str().unwrap();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let model = Mom::parse(&bytes).unwrap();
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            value["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new(std::array::from_fn(|i| {
            value["owned_sqrt"][i].as_i64().unwrap() as i16
        })),
    };
    let scenes: Vec<_> = value["graphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(scene)
        .collect();
    for (index, case) in value["cases"].as_array().unwrap().iter().enumerate() {
        let graph = case["graph"].as_u64().unwrap() as usize;
        let input = &case["input"];
        let mut actor = model.construct_actor().unwrap();
        actor.pose.position = array(&input["position"]);
        actor.pose.angles = array::<3>(&input["angles"]).map(|v| v as i16);
        actor.animation.actor_coordinate_cache = 0;
        if let Some(contact) = case["initial"]["contact"].as_array() {
            let slot = contact[0].as_u64().unwrap() as usize;
            actor.body_contact.current = Some(ActorContact::Floor(SceneContact {
                slot,
                map_index: scenes[graph].slots.slots[slot].map_index,
                geometry: WorldContact {
                    cell: contact[1].as_u64().unwrap() as usize,
                    tile: contact[2].as_u64().unwrap() as usize,
                    model: 0,
                    position: actor.pose.position,
                },
            }));
        }
        let mut host = Host {
            scene: &scenes[graph],
            math: &math,
            bound: value["graphs"][graph]["bound"].as_bool().unwrap(),
            signals: vec![],
            journal: vec![],
        };
        assert_state(
            &state(&actor, &host),
            &case["initial"],
            &format!("case {index} initial"),
        );
        let axis = if input["axis"] == "Z" {
            ActorAxis::Z
        } else {
            ActorAxis::X
        };
        move_actor_on_floor(
            &mut actor,
            axis,
            input["amount"].as_i64().unwrap() as i32,
            input["notify"].as_bool().unwrap(),
            &mut PsxRng::new(1),
            &mut host,
        )
        .unwrap();
        assert_state(
            &state(&actor, &host),
            &case["state"],
            &format!("case {index} graph {graph} {input}"),
        );
    }
    assert_eq!(value["cases"].as_array().unwrap().len(), 2304);
}

#[derive(Default)]
struct Sound {
    events: Vec<Value>,
    voice: i32,
}
impl lsd_core::GenerationSound for Sound {
    fn stop_note(&mut self, voice: i32) -> i32 {
        self.events.push(json!(["stop", voice]));
        -1
    }
    fn volume_offset(&mut self, value: i32) {
        self.events.push(json!(["volume", value]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        self.events.push(json!(["play", note, period, counter]));
        let voice = self.voice;
        self.voice += 1;
        voice
    }
}
fn horse_state(
    actor: &ModelActor,
    behavior: &lsd_core::EntityBehavior,
    host: &Host<'_>,
    sound: &Sound,
    rng: &PsxRng,
) -> Value {
    let mut result = state(actor, host);
    result.as_object_mut().unwrap().extend(json!({"interaction_timer":actor.animation.ticks,
        "behavior_tick":behavior.ticks,"state":behavior.state,"triggered":behavior.triggered,
        "link_enabled":behavior.link_enabled,"behavior_active":behavior.active,
        "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
        "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|part|part.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|part|part.coordinate_translation).collect::<Vec<_>>(),
        "context":behavior.context.words,"rng":rng.state(),"audio":sound.events}).as_object().unwrap().clone());
    result
}
fn compare_horse(
    case: &Value,
    model: &Mom,
    fields: [i8; 12],
    entity: u8,
    graph: &Value,
    math: &PsxMathTables,
) {
    let scene = scene(graph);
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let mut actor = model.construct_actor().unwrap();
    let mut behavior = lsd_core::EntityBehavior::construct(
        entity,
        lsd_core::NpcScript::Horse,
        fields,
        &mut actor,
        &mut sound,
    );
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    actor.pose.position = array(&case["initial"]["pose"]["position"]);
    actor.animation.actor_coordinate_cache = 0;
    behavior.ticks = case["tick"].as_i64().unwrap() as i32;
    behavior.context.words[1] = behavior.ticks;
    let mut host = Host {
        scene: &scene,
        math,
        bound: graph["bound"].as_bool().unwrap(),
        signals: vec![],
        journal: vec![],
    };
    let mut rng = PsxRng::new(1);
    sound.events.clear();
    assert_state(
        &horse_state(&actor, &behavior, &host, &sound, &rng),
        &case["initial"],
        "Horse initial",
    );
    for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
        host.signals.clear();
        host.journal.clear();
        sound.events.clear();
        behavior
            .tick_with_services(
                &mut actor,
                &model.animations,
                lsd_core::BehaviorTick {
                    target: [10000, 0, 10000],
                    proximity: fields[10],
                },
                &mut rng,
                &mut sound,
                &mut host,
            )
            .unwrap();
        assert_state(
            &horse_state(&actor, &behavior, &host, &sound, &rng),
            expected,
            &format!(
                "Horse graph {} tick {} step {index}",
                case["graph"], case["tick"]
            ),
        );
    }
}
#[test]
fn owned_horse_action_follows_floor_and_preserves_timed_movement_sound_and_animation() {
    let value: Value =
        serde_json::from_str(include_str!("fixtures/horse-floor-runtime.json")).unwrap();
    let hex = value["owned_model_hex"].as_str().unwrap();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let model = Mom::parse(&bytes).unwrap();
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            value["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new(std::array::from_fn(|i| {
            value["owned_sqrt"][i].as_i64().unwrap() as i16
        })),
    };
    for case in value["cases"].as_array().unwrap() {
        compare_horse(
            case,
            &model,
            array::<12>(&case["fields"]).map(|v| v as i8),
            17,
            &value["graphs"][case["graph"].as_u64().unwrap() as usize],
            &math,
        );
    }
}
#[test]
#[ignore = "Requires original Horse model/properties/math and private original CPU observations"]
fn canonical_horse_action_matches_original_through_floor_graphs() {
    let owned: Value =
        serde_json::from_str(include_str!("fixtures/horse-floor-runtime.json")).unwrap();
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let data: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_HORSE_FLOOR").expect("Set LSD_NATIVE_HORSE_FLOOR"),
        )
        .unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let math = lsd_assets::load_math_tables(&exe).unwrap();
    for case in data["cases"].as_array().unwrap() {
        let binding = &case["binding"];
        let source = std::fs::read(files.join(binding["path"].as_str().unwrap())).unwrap();
        assert_eq!(
            lsd_assets::sha256_hex(&source),
            binding["source_sha256"].as_str().unwrap()
        );
        let at = binding["offset"].as_u64().unwrap() as usize;
        let end = at + binding["bytes"].as_u64().unwrap() as usize;
        assert_eq!(
            lsd_assets::sha256_hex(&source[at..end]),
            binding["mom_sha256"].as_str().unwrap()
        );
        compare_horse(
            case,
            &Mom::parse(&source[at..end]).unwrap(),
            array::<12>(&case["fields"]).map(|v| v as i8),
            binding["entity"].as_u64().unwrap() as u8,
            &owned["graphs"][case["graph"].as_u64().unwrap() as usize],
            &math,
        );
    }
}

#[test]
fn horse_encounter_adapter_uses_live_floor_scene_and_retains_original_frame_state() {
    use lsd_assets::{
        ActorFrame, EncounterEventHost, EncounterNotification, EncounterScene, EntityProperties,
        SceneActor,
    };
    use std::sync::Arc;
    struct Events<'a>(&'a SceneCollision);
    impl EncounterEventHost for Events<'_> {
        fn actor_floor_query(
            &mut self,
            position: [i32; 3],
            math: &PsxMathTables,
        ) -> Result<ActorFloorQuery, BehaviorFault> {
            self.0
                .actor_floor_query(position, math)
                .map_err(BehaviorFault::Scene)
        }
        fn dispatch(
            &mut self,
            event: EncounterNotification,
            _fields: [i8; 12],
            _rng: &mut PsxRng,
        ) -> Result<(), lsd_core::DayFault> {
            panic!("Supplied far Horse target emitted {event:?}");
        }
    }
    let value: Value =
        serde_json::from_str(include_str!("fixtures/horse-floor-runtime.json")).unwrap();
    let case = value["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["graph"] == 0 && case["tick"] == 0)
        .unwrap();
    let hex = value["owned_model_hex"].as_str().unwrap();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let resource = Arc::new(Mom::parse(&bytes).unwrap());
    let floor = scene(&value["graphs"][0]);
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            value["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new(std::array::from_fn(|i| {
            value["owned_sqrt"][i].as_i64().unwrap() as i16
        })),
    };
    let fields = array::<12>(&case["fields"]).map(|v| v as i8);
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let mut actor = resource.construct_actor().unwrap();
    let script = lsd_core::NpcScript::from_reference_address(Some(0x8005_f544));
    assert_eq!(script, lsd_core::NpcScript::Horse);
    let mut behavior =
        lsd_core::EntityBehavior::construct(17, script, fields, &mut actor, &mut sound);
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    actor.pose.position = array(&case["initial"]["pose"]["position"]);
    actor.animation.actor_coordinate_cache = 0;
    sound.events.clear();
    let mut encounters = EncounterScene::new(
        lsd_core::EncounterTables {
            chunks: std::array::from_fn(|_| vec![]),
            objects: std::array::from_fn(|_| vec![]),
            variants: vec![],
            positions: vec![],
            rotations: vec![],
            color_lookup: vec![],
        },
        vec![],
    );
    let serial = encounters.add_actor(SceneActor {
        serial: 0,
        slot: 0,
        map: 0,
        entity: 17,
        model: 0,
        resource: Arc::clone(&resource),
        actor,
        collision: None,
        properties: EntityProperties {
            fields,
            callback: Some(0x8005_f544),
        },
        behavior_active: true,
        behavior_pending: false,
        behavior,
    });
    let mut rng = PsxRng::new(1);
    let mut events = Events(&floor);
    let host = Host {
        scene: &floor,
        math: &math,
        bound: true,
        signals: vec![],
        journal: vec![],
    };
    for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
        sound.events.clear();
        encounters
            .tick_actor_frame(
                ActorFrame {
                    serial,
                    target: lsd_core::EntityTarget {
                        position: [10000, 0, 10000],
                        local_y: 0,
                        matrix: FixedMatrix::IDENTITY,
                        eye: [0; 3],
                        target: [0, 0, 1024],
                    },
                    signal: lsd_core::FrameSignal::Tick,
                },
                &mut rng,
                &mut sound,
                &math,
                &mut events,
            )
            .unwrap();
        let actor = &encounters.actors[0];
        let mut actual = horse_state(&actor.actor, &actor.behavior, &host, &sound, &rng);
        let mut expected = expected.clone();
        // Local virtual-call instrumentation belongs to the direct wrapper replay.
        actual.as_object_mut().unwrap().remove("signals");
        expected.as_object_mut().unwrap().remove("signals");
        assert_state(&actual, &expected, &format!("Horse encounter step {index}"));
        assert!(encounters.pending_notifications.is_empty());
    }
}
