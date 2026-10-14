#[path = "support/day_runtime.rs"]
mod day_runtime;
#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use day_runtime::DayRuntime;
use lsd_assets::EncounterEventHost;
use lsd_assets::formats::Mom;
use lsd_core::{
    ActorContact, BehaviorFault, EntityScriptHost, FixedMatrix, ModelActor, PsxAimTables,
    PsxMathTables, PsxRng,
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/day-frame-npc-runtime.json")).unwrap()
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
    day_queries: u32,
    proximity_calls: Vec<Value>,
    fields: [i8; 12],
    runtime: DayRuntime,
    signals: Vec<i32>,
    journal: Vec<i32>,
}
impl EntityScriptHost for Host<'_> {
    fn day_number(&mut self) -> Result<i32, BehaviorFault> {
        self.day_queries += 1;
        self.runtime
            .dispatch(self.math, |dispatcher| dispatcher.day_number())
    }
    fn actor_proximity(
        &mut self,
        actor: &ModelActor,
        forward: i32,
        radius: i32,
    ) -> Result<bool, BehaviorFault> {
        self.proximity_calls
            .push(json!([actor.pose.position, forward, radius]));
        let (position, forward, radius) =
            lsd_core::entity_proximity_args(self.fields, actor.pose.position, forward, radius);
        lsd_core::EntityTarget {
            position: self.target,
            local_y: 17,
            matrix: FixedMatrix::IDENTITY,
            eye: [0; 3],
            target: [0, 0, 1024],
        }
        .proximity(position, forward, radius)
    }
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
        "signals":host.signals,"journal":host.journal,"aim_calls":host.aim_calls,"day_queries":host.day_queries,"proximity_calls":host.proximity_calls})
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
        "entity_gargoyle_pit" => lsd_core::NpcScript::GargoylePit,
        "entity_mushroom" => lsd_core::NpcScript::Mushroom,
        "entity_human_face_flower" => lsd_core::NpcScript::HumanFaceFlower,
        "entity_lips_giant" => lsd_core::NpcScript::GiantLips,
        "entity_whale" => lsd_core::NpcScript::Whale,
        "entity_hopscotch_girl" => lsd_core::NpcScript::HopscotchGirl,
        "entity_corpse" => lsd_core::NpcScript::Corpse,
        _ => panic!("Unknown day/frame action"),
    }
}
fn runtime(case: &Value) -> DayRuntime {
    let mut runtime = DayRuntime::new([0; 9]);
    runtime.day.year = case["year"].as_i64().unwrap() as i32;
    runtime.day.day = case["day"].as_i64().unwrap() as i32;
    runtime
}
fn host<'a>(
    case: &Value,
    fields: [i8; 12],
    math: &'a PsxMathTables,
    aim: &'a PsxAimTables,
) -> Host<'a> {
    Host {
        math,
        aim,
        target: array(&case["target"]),
        aim_calls: 0,
        day_queries: 0,
        proximity_calls: vec![],
        fields,
        runtime: runtime(case),
        signals: vec![],
        journal: vec![],
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
    assert_eq!(
        model.animations[0].frames.len(),
        case["duration"].as_u64().unwrap() as usize
    );
    for _ in 0..case["seek"].as_u64().unwrap() {
        actor.animation.advance_frame(&model.animations).unwrap();
    }
    actor.animation.enabled = case["initial_animation_enabled"].as_bool().unwrap();
    behavior.state = 0;
    behavior.triggered = 0;
    behavior.ticks = case["tick"].as_i64().unwrap() as i32;
    behavior.context.words[1] = case["context_tick"].as_i64().unwrap() as i32;
    actor.pose.position = array(&case["initial"]["pose"]["position"]);
    actor.pose.angles = array::<3>(&case["initial"]["pose"]["angles"]).map(|v| v as i16);
    actor.animation.actor_coordinate_cache = 0;
    let mut host = host(case, fields, math, aim);
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
        host.day_queries = 0;
        host.proximity_calls.clear();
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
fn owned_day_frame_actions_match_original_rng_animation_queries_and_sound() {
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
fn canonical_day_frame_actions_match_original_cpu() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let v: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_DAY_FRAME_NPCS").expect("Set LSD_NATIVE_DAY_FRAME_NPCS"),
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
    struct Events<'a> {
        journal: Vec<i32>,
        runtime: DayRuntime,
        math: &'a PsxMathTables,
    }
    impl EncounterEventHost for Events<'_> {
        fn day_number(&mut self) -> Result<i32, BehaviorFault> {
            self.runtime
                .dispatch(self.math, |dispatcher| dispatcher.day_number())
        }
        fn dispatch(
            &mut self,
            event: EncounterNotification,
            _fields: [i8; 12],
            _rng: &mut PsxRng,
        ) -> Result<(), lsd_core::DayFault> {
            let EncounterNotificationKind::Notify(code) = event.kind else {
                panic!("Unexpected event {event:?}")
            };
            self.journal.push(code);
            Ok(())
        }
    }

    let callback = match case["callback"].as_str().unwrap() {
        "entity_gargoyle_pit" => 0x8005_e4d0,
        "entity_mushroom" => 0x8005_ff7c,
        "entity_human_face_flower" => 0x8006_0800,
        "entity_lips_giant" => 0x8006_0a4c,
        "entity_whale" => 0x8006_0cf0,
        "entity_hopscotch_girl" => 0x8006_1070,
        "entity_corpse" => 0x8006_1158,
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
    assert_eq!(
        model.animations[0].frames.len(),
        case["duration"].as_u64().unwrap() as usize
    );
    for _ in 0..case["seek"].as_u64().unwrap() {
        actor.animation.advance_frame(&model.animations).unwrap();
    }
    actor.animation.enabled = case["initial_animation_enabled"].as_bool().unwrap();
    behavior.state = 0;
    behavior.triggered = 0;
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
    let mut events = Events {
        journal: vec![],
        runtime: runtime(case),
        math,
    };
    let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
    let host = host(case, fields, math, aim);
    for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
        sound.events.clear();
        events.journal.clear();
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
        actual["journal"] = json!(events.journal);
        // Direct replay compares local virtual calls and facing-call instrumentation.
        for key in ["aim_calls", "signals", "day_queries", "proximity_calls"] {
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
fn day_frame_encounter_adapter_matches_owned_complete_updates() {
    let v: Value =
        serde_json::from_str(include_str!("fixtures/day-frame-npc-encounters.json")).unwrap();
    let model = std::sync::Arc::new(Mom::parse(&bytes(&v["owned_model_hex"])).unwrap());
    let math = math(&v);
    let aim = aim(&v);
    for case in v["cases"].as_array().unwrap() {
        compare_encounter_case(case, std::sync::Arc::clone(&model), &math, &aim);
    }
}
#[test]
#[ignore = "Requires original NPC models/properties/math and complete original entity-update observations"]
fn canonical_day_frame_encounter_updates_match_original_cpu() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let v: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_DAY_FRAME_ENCOUNTERS")
                .expect("Set LSD_NATIVE_DAY_FRAME_ENCOUNTERS"),
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

#[test]
fn actor_journal_day_number_matches_original_with_separate_year_and_wrapping() {
    let v = fixture();
    let math = math(&v);
    let mut runtime = DayRuntime::new([0; 9]);
    for case in v["day_cases"].as_array().unwrap() {
        runtime.day.year = case["year"].as_i64().unwrap() as i32;
        runtime.day.day = case["day"].as_i64().unwrap() as i32;
        let before = (runtime.day.year, runtime.day.day);
        let number = runtime
            .dispatch(&math, |dispatcher| dispatcher.day_number())
            .unwrap();
        assert_eq!(number, case["number"].as_i64().unwrap() as i32);
        assert_eq!(before, (runtime.day.year, runtime.day.day));
        assert_eq!(case["returned_year"], case["year"]);
    }
}
