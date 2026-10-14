use lsd_assets::formats::Mom;
use lsd_core::{
    ActorContact, BehaviorFault, EntityScriptHost, FixedMatrix, ModelActor, PsxAimTables,
    PsxMathTables, PsxRng,
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/motion-npc-runtime.json")).unwrap()
}
fn array<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
fn bytes(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn math(value: &Value) -> PsxMathTables {
    PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            value["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new(std::array::from_fn(|i| {
            value["owned_sqrt"][i].as_i64().unwrap() as i16
        })),
    }
}
fn aim(value: &Value) -> PsxAimTables {
    PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| {
            value["owned_atan"][i].as_i64().unwrap() as i16
        })),
    }
}
struct Host<'a> {
    math: &'a PsxMathTables,
    aim: &'a PsxAimTables,
    target: [i32; 3],
    aim_calls: u32,
    signals: Vec<i32>,
    journal: Vec<i32>,
}
impl EntityScriptHost for Host<'_> {
    fn face_player(&mut self, actor: &mut ModelActor) -> Result<(), BehaviorFault> {
        self.aim_calls += 1;
        actor.pose.angles = self
            .aim
            .object_angles(actor.pose.position, self.target, true, false)
            .map_err(BehaviorFault::Aim)?;
        actor.animation.actor_coordinate_cache = 0;
        Ok(())
    }
    fn movement_matrix(&mut self, angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault> {
        Ok(self.math.rotation(angles))
    }
    fn notify(&mut self, _rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault> {
        self.signals.push(code);
        self.journal.push(code);
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
        "cache":actor.animation.actor_coordinate_cache,"move_distance":actor.move_distance,"contact":contact,
        "signals":host.signals,"journal":host.journal,"aim_calls":host.aim_calls})
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
        "entity_tennyo" => lsd_core::NpcScript::Tennyo,
        "entity_bird" => lsd_core::NpcScript::Bird,
        "entity_rocket" => lsd_core::NpcScript::Rocket,
        "entity_ufo" => lsd_core::NpcScript::Ufo,
        "entity_shark" => lsd_core::NpcScript::Shark,
        "entity_fish" => lsd_core::NpcScript::Fish,
        "entity_mirage" => lsd_core::NpcScript::Mirage,
        _ => panic!("Unknown movement action"),
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

fn compare_case(case: &Value, model: &Mom, math: &PsxMathTables, aim: &PsxAimTables) {
    let fields = array::<12>(&case["fields"]).map(|v| v as i8);
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let mut actor = model.construct_actor().unwrap();
    let mut behavior = lsd_core::EntityBehavior::construct(
        case["entity"].as_u64().unwrap() as u8,
        script(case["callback"].as_str().unwrap()),
        fields,
        &mut actor,
        &mut sound,
    );
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    behavior.state = case["initial_state"].as_i64().unwrap() as i32;
    behavior.triggered = case["initial_trigger"].as_i64().unwrap() as i32;
    behavior.ticks = case["tick"].as_i64().unwrap() as i32;
    behavior.context.words[1] = case["context_tick"].as_i64().unwrap() as i32;
    actor.pose.position = array(&case["initial"]["pose"]["position"]);
    actor.pose.angles = array::<3>(&case["initial"]["pose"]["angles"]).map(|v| v as i16);
    actor.animation.actor_coordinate_cache = 0;
    let mut host = Host {
        math,
        aim,
        target: array(&case["target"]),
        aim_calls: 0,
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
        sound.events.clear();
        host.aim_calls = 0;
        host.signals.clear();
        host.journal.clear();
        behavior
            .tick_with_services(
                &mut actor,
                &model.animations,
                lsd_core::BehaviorTick {
                    target: host.target,
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
                "{} tick {} context {} trigger {} step {index}",
                case["callback"], case["tick"], case["context_tick"], case["initial_trigger"]
            ),
        );
    }
}
#[test]
fn owned_motion_actions_match_original_timing_rng_sound_and_shutdown() {
    let v = fixture();
    let model = Mom::parse(&bytes(&v["owned_model_hex"])).unwrap();
    let math = math(&v);
    let aim = aim(&v);
    for case in v["cases"].as_array().unwrap() {
        compare_case(case, &model, &math, &aim);
    }
}
#[test]
#[ignore = "Requires original seven-NPC models/properties/math and private original CPU observations"]
fn canonical_motion_actions_match_original_cpu() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let v: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_MOTION_NPCS").expect("Set LSD_NATIVE_MOTION_NPCS"),
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
    for case in v["cases"].as_array().unwrap() {
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
        compare_case(case, &Mom::parse(&source[at..end]).unwrap(), &math, &aim);
    }
}
fn compare_encounter_case(
    case: &Value,
    model: std::sync::Arc<Mom>,
    math: &PsxMathTables,
    aim: &PsxAimTables,
) {
    use lsd_assets::{
        ActorFrame, EncounterEventHost, EncounterNotification, EncounterNotificationKind,
        EncounterScene, EntityProperties, SceneActor,
    };
    use std::sync::Arc;
    struct Events(Vec<i32>);
    impl EncounterEventHost for Events {
        fn dispatch(
            &mut self,
            event: EncounterNotification,
            _fields: [i8; 12],
            _rng: &mut PsxRng,
        ) -> Result<(), lsd_core::DayFault> {
            let EncounterNotificationKind::Notify(code) = event.kind else {
                panic!("Unexpected event {event:?}")
            };
            self.0.push(code);
            Ok(())
        }
    }

    let callback = match case["callback"].as_str().unwrap() {
        "entity_tennyo" => 0x8005_f1d4,
        "entity_bird" => 0x8005_f368,
        "entity_rocket" => 0x8005_f454,
        "entity_ufo" => 0x8005_f608,
        "entity_shark" => 0x8005_fb6c,
        "entity_fish" => 0x8005_fc58,
        "entity_mirage" => 0x8005_fdfc,
        _ => unreachable!(),
    };
    assert_eq!(
        lsd_core::NpcScript::from_reference_address(Some(callback)),
        script(case["callback"].as_str().unwrap())
    );
    let fields = array::<12>(&case["fields"]).map(|v| v as i8);
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let mut actor = model.construct_actor().unwrap();
    let mut behavior = lsd_core::EntityBehavior::construct(
        case["entity"].as_u64().unwrap() as u8,
        script(case["callback"].as_str().unwrap()),
        fields,
        &mut actor,
        &mut sound,
    );
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    behavior.state = case["initial_state"].as_i64().unwrap() as i32;
    behavior.triggered = case["initial_trigger"].as_i64().unwrap() as i32;
    behavior.ticks = case["tick"].as_i64().unwrap() as i32;
    behavior.context.words[1] = case["context_tick"].as_i64().unwrap() as i32;
    actor.pose.position = array(&case["initial"]["pose"]["position"]);
    actor.pose.angles = array::<3>(&case["initial"]["pose"]["angles"]).map(|v| v as i16);
    actor.animation.actor_coordinate_cache = 0;
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
    encounters.enable_aim(aim.clone());
    let collision = model.collision_model(&actor).unwrap();
    let serial = encounters.add_actor(SceneActor {
        serial: 0,
        slot: 0,
        map: 0,
        entity: case["entity"].as_u64().unwrap() as u8,
        model: 0,
        resource: Arc::clone(&model),
        actor,
        collision,
        properties: EntityProperties {
            fields,
            callback: Some(callback),
        },
        behavior_active: true,
        behavior_pending: false,
        behavior,
    });
    let mut events = Events(vec![]);
    let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
    let host = Host {
        math,
        aim,
        target: array(&case["target"]),
        aim_calls: 0,
        signals: vec![],
        journal: vec![],
    };
    for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
        sound.events.clear();
        events.0.clear();
        encounters
            .tick_actor_frame(
                ActorFrame {
                    serial,
                    target: lsd_core::EntityTarget {
                        position: array(&case["target"]),
                        local_y: 17,
                        matrix: FixedMatrix::IDENTITY,
                        eye: [0; 3],
                        target: [0, 0, 1024],
                    },
                    signal: lsd_core::FrameSignal::Tick,
                },
                &mut rng,
                &mut sound,
                math,
                &mut events,
            )
            .unwrap();
        let actor = &encounters.actors[0];
        let mut actual = action_state(&actor.actor, &actor.behavior, &host, &sound, &rng);
        let mut expected = expected.clone();
        actual["journal"] = json!(events.0);
        // Direct replay compares local virtual calls and facing-call instrumentation.
        for key in ["aim_calls", "signals"] {
            actual.as_object_mut().unwrap().remove(key);
            expected.as_object_mut().unwrap().remove(key);
        }
        assert_state(
            &actual,
            &expected,
            &format!(
                "{} encounter tick {} trigger {} step {index}",
                case["callback"], case["tick"], case["initial_trigger"]
            ),
        );
        assert!(encounters.pending_notifications.is_empty());
    }
}

#[test]
fn motion_encounter_adapter_preserves_all_owned_action_trajectories() {
    let v: Value =
        serde_json::from_str(include_str!("fixtures/motion-npc-encounters.json")).unwrap();
    let model = std::sync::Arc::new(Mom::parse(&bytes(&v["owned_model_hex"])).unwrap());
    let math = math(&v);
    let aim = aim(&v);
    for case in v["cases"].as_array().unwrap() {
        compare_encounter_case(case, std::sync::Arc::clone(&model), &math, &aim);
    }
}
#[test]
#[ignore = "Requires original NPC models/properties/math and complete original entity-update observations"]
fn canonical_motion_encounter_updates_match_original_interaction_and_actions() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let v: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_MOTION_ENCOUNTERS")
                .expect("Set LSD_NATIVE_MOTION_ENCOUNTERS"),
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
    for case in v["cases"].as_array().unwrap() {
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
        compare_encounter_case(
            case,
            std::sync::Arc::new(Mom::parse(&source[at..end]).unwrap()),
            &math,
            &aim,
        );
    }
}
