use lsd_assets::formats::Mom;
use lsd_core::{
    ActorContact, ActorFloorQuery, BehaviorFault, EntityScriptHost, FixedMatrix, ModelActor,
    PsxAimTables, PsxMathTables, PsxRng, SceneCollision,
};
#[path = "support/actor_floor.rs"]
mod floor_support;
use floor_support::{array, scene};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/branch-floor-runtime.json")).unwrap()
}
struct Host<'a> {
    scene: &'a SceneCollision,
    math: &'a PsxMathTables,
    bound: bool,
    aim: &'a PsxAimTables,
    target: [i32; 3],
    action: bool,
    aim_calls: u32,
    action_queries: u32,
    signals: Vec<i32>,
    journal: Vec<i32>,
}
impl EntityScriptHost for Host<'_> {
    fn player_action_pressed(&mut self) -> Result<bool, BehaviorFault> {
        self.action_queries += 1;
        Ok(self.action)
    }
    fn face_player(&mut self, actor: &mut ModelActor) -> Result<(), BehaviorFault> {
        actor.pose.angles = self
            .aim
            .object_angles(actor.pose.position, self.target, true, false)
            .map_err(BehaviorFault::Aim)?;
        actor.animation.actor_coordinate_cache = 0;
        self.aim_calls += 1;
        Ok(())
    }
    fn movement_matrix(&mut self, angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault> {
        Ok(self.math.rotation(angles))
    }
    fn notify(&mut self, _rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault> {
        if code == 6 || code == 7 {
            self.signals.push(code);
        }
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
        "contact":contact,"signals":host.signals,"journal":host.journal,"aim_calls":host.aim_calls,"action_queries":host.action_queries})
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
fn script(name: &str) -> lsd_core::NpcScript {
    match name {
        "entity_maiko_giant" => lsd_core::NpcScript::GiantMaiko,
        "entity_fox" => lsd_core::NpcScript::Fox,
        "entity_standing_penguin" => lsd_core::NpcScript::StandingPenguin,
        "entity_sliding_penguin" => lsd_core::NpcScript::SlidingPenguin,
        _ => panic!("Unknown branching floor action"),
    }
}
fn action_state(
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
fn compare_floor_action(
    case: &Value,
    model: &Mom,
    fields: [i8; 12],
    entity: u8,
    graph: &Value,
    math: &PsxMathTables,
    aim: &PsxAimTables,
) {
    let scene = scene(graph);
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let mut actor = model.construct_actor().unwrap();
    let mut behavior = lsd_core::EntityBehavior::construct(
        entity,
        script(case["callback"].as_str().unwrap()),
        fields,
        &mut actor,
        &mut sound,
    );
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    actor.pose.position = array(&case["initial"]["pose"]["position"]);
    actor.animation.actor_coordinate_cache = 0;
    behavior.ticks = case["tick"].as_i64().unwrap() as i32;
    behavior.state = case["initial_state"].as_i64().unwrap() as i32;
    behavior.context.words[1] = behavior.ticks;
    let mut host = Host {
        scene: &scene,
        math,
        bound: graph["bound"].as_bool().unwrap(),
        aim,
        target: array(&case["target"]),
        action: case["action"].as_bool().unwrap(),
        aim_calls: 0,
        action_queries: 0,
        signals: vec![],
        journal: vec![],
    };
    let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
    sound.events.clear();
    assert_state(
        &action_state(&actor, &behavior, &host, &sound, &rng),
        &case["initial"],
        &format!("{} initial", case["callback"]),
    );
    for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
        host.signals.clear();
        host.journal.clear();
        host.aim_calls = 0;
        host.action_queries = 0;
        sound.events.clear();
        behavior
            .tick_with_services(
                &mut actor,
                &model.animations,
                lsd_core::BehaviorTick {
                    target: array(&case["target"]),
                    proximity: fields[10],
                },
                &mut rng,
                &mut sound,
                &mut host,
            )
            .unwrap();
        assert_state(
            &action_state(&actor, &behavior, &host, &sound, &rng),
            expected,
            &format!(
                "{} graph {} tick {} step {index}",
                case["callback"], case["graph"], case["tick"]
            ),
        );
    }
}

#[test]
fn owned_branching_floor_actions_preserve_original_state_rng_motion_and_sound() {
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
    let aim = PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| {
            value["owned_atan"][i].as_i64().unwrap() as i16
        })),
    };
    for case in value["cases"].as_array().unwrap() {
        compare_floor_action(
            case,
            &model,
            array::<12>(&case["fields"]).map(|v| v as i8),
            17,
            &value["graphs"][case["graph"].as_u64().unwrap() as usize],
            &math,
            &aim,
        );
    }
}
#[test]
#[ignore = "Requires original Maiko/Fox/Penguin models, properties, math and private original CPU observations"]
fn canonical_branching_floor_actions_match_original_cpu() {
    let owned = fixture();
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let data: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_BRANCH_FLOOR").expect("Set LSD_NATIVE_BRANCH_FLOOR"),
        )
        .unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(files.join("SLPS_015.56")).unwrap();
    assert_eq!(
        lsd_assets::sha256_hex(&exe),
        "282308ade239ce61b5603651dbbab0476a681b5d3618e82c0d3c3a95316d302f"
    );
    let math = lsd_assets::load_math_tables(&exe).unwrap();
    let aim = lsd_assets::load_special_encounter_assets(&exe, &files)
        .unwrap()
        .aim;
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
        compare_floor_action(
            case,
            &Mom::parse(&source[at..end]).unwrap(),
            array::<12>(&case["fields"]).map(|v| v as i8),
            binding["entity"].as_u64().unwrap() as u8,
            &owned["graphs"][case["graph"].as_u64().unwrap() as usize],
            &math,
            &aim,
        );
    }
}
#[test]
fn branching_floor_encounter_adapter_preserves_original_action_trajectories() {
    use lsd_assets::{
        ActorFrame, EncounterEventHost, EncounterNotification, EncounterScene, EntityProperties,
        SceneActor,
    };
    use std::sync::Arc;
    struct Events<'a>(&'a SceneCollision, bool, Vec<i32>);
    impl EncounterEventHost for Events<'_> {
        fn player_action_pressed(&mut self) -> Result<bool, BehaviorFault> {
            Ok(self.1)
        }
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
            let lsd_assets::EncounterNotificationKind::Notify(code) = event.kind else {
                panic!("Unexpected event {event:?}");
            };
            self.2.push(code);
            Ok(())
        }
    }
    let value: Value =
        serde_json::from_str(include_str!("fixtures/branch-floor-runtime.json")).unwrap();
    let cases: Vec<_> = value["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| {
            case["graph"] == 0 && case["tick"] == 0 && case["target"][0].as_i64().unwrap() > 10000
        })
        .collect();
    assert_eq!(cases.len(), 11);
    for case in cases {
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
        let callback = match case["callback"].as_str().unwrap() {
            "entity_maiko_giant" => 0x8005_e7f8,
            "entity_fox" => 0x8005_ed30,
            "entity_standing_penguin" => 0x8006_0d80,
            "entity_sliding_penguin" => 0x8006_0f38,
            _ => unreachable!(),
        };
        let script = lsd_core::NpcScript::from_reference_address(Some(callback));
        assert_eq!(script, self::script(case["callback"].as_str().unwrap()));
        let mut behavior =
            lsd_core::EntityBehavior::construct(17, script, fields, &mut actor, &mut sound);
        behavior.enable_link(&mut actor);
        behavior.start_behavior(&mut actor);
        behavior.state = case["initial_state"].as_i64().unwrap() as i32;
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
                callback: Some(callback),
            },
            behavior_active: true,
            behavior_pending: false,
            behavior,
        });
        let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        let aim = PsxAimTables {
            atan: Box::new(std::array::from_fn(|i| {
                value["owned_atan"][i].as_i64().unwrap() as i16
            })),
        };
        encounters.enable_aim(aim.clone());
        let mut events = Events(&floor, case["action"].as_bool().unwrap(), vec![]);
        let host = Host {
            scene: &floor,
            math: &math,
            bound: true,
            aim: &aim,
            target: array(&case["target"]),
            action: case["action"].as_bool().unwrap(),
            aim_calls: 0,
            action_queries: 0,
            signals: vec![],
            journal: vec![],
        };
        for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
            sound.events.clear();
            events.2.clear();
            encounters
                .tick_actor_frame(
                    ActorFrame {
                        serial,
                        target: lsd_core::EntityTarget {
                            position: array(&case["target"]),
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
            let mut actual = action_state(&actor.actor, &actor.behavior, &host, &sound, &rng);
            let mut expected = expected.clone();
            actual["journal"] = json!(events.2);
            // Direct replay compares local calls and service counts; the adapter compares resulting state.
            for key in ["aim_calls", "action_queries"] {
                actual.as_object_mut().unwrap().remove(key);
                expected.as_object_mut().unwrap().remove(key);
            }
            actual.as_object_mut().unwrap().remove("signals");
            expected.as_object_mut().unwrap().remove("signals");
            assert_state(
                &actual,
                &expected,
                &format!("{} encounter step {index}", case["callback"]),
            );
            assert!(encounters.pending_notifications.is_empty());
        }
    }
}
