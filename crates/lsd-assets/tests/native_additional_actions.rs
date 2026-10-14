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
    serde_json::from_str(include_str!("fixtures/additional-actions-npc-runtime.json")).unwrap()
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
    scene_scale: lsd_core::SceneScale,
    scale_calls: Vec<Value>,
    runtime: DayRuntime,
    signals: Vec<i32>,
    journal: Vec<i32>,
    callback_calls: Vec<Value>,
    sound: Sound,
}
impl EntityScriptHost for Host<'_> {
    fn restore_player_callbacks(&mut self) -> Result<(), BehaviorFault> {
        self.callback_calls.push(json!(["set", 1, 1]));
        self.runtime.dispatch(self.math, |dispatcher| {
            dispatcher.restore_player_callbacks()
        })
    }
    fn clear_player_callbacks(&mut self, with_view: bool) -> Result<(), BehaviorFault> {
        self.callback_calls
            .push(json!(["clear", i32::from(with_view)]));
        self.runtime.dispatch(self.math, |dispatcher| {
            dispatcher.clear_player_callbacks(with_view)
        })
    }
    fn request_scene_scale(&mut self, amount: i32, mode: i32) -> Result<(), BehaviorFault> {
        self.scale_calls.push(json!([amount, mode]));
        self.runtime.scene_scale = Some(self.scene_scale.clone());
        let result = self.runtime.dispatch(self.math, |dispatcher| {
            dispatcher.request_scene_scale(amount, mode)
        });
        self.scene_scale = self.runtime.scene_scale.take().unwrap();
        result
    }
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
    fn notify_actor(
        &mut self,
        _actor: &mut ModelActor,
        rng: &mut PsxRng,
        code: i32,
    ) -> Result<(), BehaviorFault> {
        if matches!(code, 2 | 3 | 5 | 6 | 7 | 8) {
            self.signals.push(code);
        }
        self.notify(rng, code)
    }
    fn notify(&mut self, rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault> {
        self.journal.push(code);
        self.runtime.allow_journal = true;
        let result = self
            .runtime
            .dispatch(self.math, |dispatcher| {
                dispatcher.dispatch(
                    lsd_assets::EncounterNotification {
                        serial: 1,
                        entity: 17,
                        kind: lsd_assets::EncounterNotificationKind::Notify(code),
                    },
                    self.fields,
                    rng,
                )
            })
            .map_err(BehaviorFault::Journal);
        self.sound.journal(&self.runtime.journal_audio);
        self.runtime.journal_audio.clear();
        result
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
        "scene_request":[host.scene_scale.remaining,host.scene_scale.selected],"scale_calls":host.scale_calls,"player_callbacks":callbacks(&host.runtime),"callback_calls":host.callback_calls,"signals":host.signals,"journal":host.journal,"aim_calls":host.aim_calls,"day_queries":host.day_queries,"proximity_calls":host.proximity_calls})
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
struct SoundState {
    events: Vec<Value>,
    voice: i32,
}
#[derive(Clone, Default)]
struct Sound(std::rc::Rc<std::cell::RefCell<SoundState>>);
impl Sound {
    fn new(voice: i32) -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(SoundState {
            events: vec![],
            voice,
        })))
    }
    fn clear(&self) {
        self.0.borrow_mut().events.clear();
    }
    fn events(&self) -> Vec<Value> {
        self.0.borrow().events.clone()
    }
    fn journal(&self, events: &[[i32; 3]]) {
        let mut state = self.0.borrow_mut();
        for event in events {
            state
                .events
                .push(json!(["play", event[0], event[1], event[2]]));
            state.voice += 1;
        }
    }
}
impl lsd_core::GenerationSound for Sound {
    fn stop_note(&mut self, voice: i32) -> i32 {
        self.0.borrow_mut().events.push(json!(["stop", voice]));
        -1
    }
    fn volume_offset(&mut self, value: i32) {
        self.0.borrow_mut().events.push(json!(["volume", value]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        let mut state = self.0.borrow_mut();
        state.events.push(json!(["play", note, period, counter]));
        let voice = state.voice;
        state.voice += 1;
        voice
    }
}

fn script(name: &str) -> lsd_core::NpcScript {
    match name {
        "entity_ship" => lsd_core::NpcScript::Ship,
        "entity_maiko_palace" => lsd_core::NpcScript::PalaceMaiko,
        "entity_acrobats" => lsd_core::NpcScript::Acrobats,
        "entity_light" => lsd_core::NpcScript::Light,
        "entity_ring" => lsd_core::NpcScript::Ring,
        "entity_hoop_girl" => lsd_core::NpcScript::HoopGirl,
        "entity_hoop_girl_giant" => lsd_core::NpcScript::GiantHoopGirl,
        "entity_elephant_small" => lsd_core::NpcScript::SmallElephant,
        "entity_maiko_bridge" => lsd_core::NpcScript::BridgeMaiko,
        "entity_question_mark_penguin" => lsd_core::NpcScript::QuestionMarkPenguin,
        "entity_standing_penguin_flock" => lsd_core::NpcScript::StandingPenguinFlock,
        "entity_triangle_head_giant" => lsd_core::NpcScript::GiantTriangleHead,
        _ => panic!("Unknown additional action"),
    }
}
fn runtime(case: &Value) -> DayRuntime {
    let mut runtime = DayRuntime::new([0; 9]);
    runtime.day.year = case["year"].as_i64().unwrap() as i32;
    runtime.day.day = case["day"].as_i64().unwrap() as i32;
    let callbacks = &case["initial"]["player_callbacks"];
    runtime.move_callback = callbacks[0].as_i64().unwrap() as i32;
    runtime.view_callback = callbacks[1].as_i64().unwrap() as i32;
    runtime.reset_view_pending = callbacks[4].as_bool().unwrap();
    runtime.view_active = callbacks[5].as_bool().unwrap();
    runtime
}
fn host<'a>(
    case: &Value,
    fields: [i8; 12],
    math: &'a PsxMathTables,
    aim: &'a PsxAimTables,
    tables: lsd_core::SceneScaleTables,
    sound: Sound,
) -> Host<'a> {
    Host {
        math,
        aim,
        target: array(&case["target"]),
        aim_calls: 0,
        day_queries: 0,
        proximity_calls: vec![],
        fields,
        scene_scale: lsd_core::SceneScale::new(tables),
        scale_calls: vec![],
        runtime: runtime(case),
        signals: vec![],
        journal: vec![],
        callback_calls: vec![],
        sound,
    }
}
fn action_state(
    actor: &ModelActor,
    behavior: &lsd_core::EntityBehavior,
    host: &Host<'_>,
    sound: &Sound,
    rng: &PsxRng,
) -> Value {
    let audio = sound.events();
    let mut result = state(actor, host);
    result.as_object_mut().unwrap().extend(json!({"interaction_timer":actor.animation.ticks,
        "behavior_tick":behavior.ticks,"state":behavior.state,"triggered":behavior.triggered,
        "link_enabled":behavior.link_enabled,"behavior_active":behavior.active,
        "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
        "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|part|part.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|part|part.coordinate_translation).collect::<Vec<_>>(),
        "context":behavior.context.words,"rng":rng.state(),"audio":audio}).as_object().unwrap().clone());
    result
}

fn compare_case(
    case: &Value,
    model: &Mom,
    math: &PsxMathTables,
    aim: &PsxAimTables,
    tables: lsd_core::SceneScaleTables,
) {
    let fields = array::<12>(&case["fields"]).map(|v| v as i8);
    let mut sound = Sound::new(41);
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
    behavior.state = case["initial_state"].as_i64().unwrap() as i32;
    behavior.triggered = 0;
    behavior.ticks = case["tick"].as_i64().unwrap() as i32;
    behavior.context.words[1] = case["context_tick"].as_i64().unwrap() as i32;
    actor.pose.position = array(&case["initial"]["pose"]["position"]);
    actor.pose.angles = array::<3>(&case["initial"]["pose"]["angles"]).map(|v| v as i16);
    actor.animation.actor_coordinate_cache = 0;
    let mut host = host(case, fields, math, aim, tables, sound.clone());
    let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
    sound.clear();
    assert_state(
        &action_state(&actor, &behavior, &host, &sound, &rng),
        &case["initial"],
        &format!("{} initial", case["callback"]),
    );
    for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
        sound.clear();
        host.runtime.journal_audio.clear();
        host.aim_calls = 0;
        host.day_queries = 0;
        host.proximity_calls.clear();
        host.signals.clear();
        host.scale_calls.clear();
        host.journal.clear();
        host.callback_calls.clear();
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
fn owned_additional_actions_actions_match_original_rng_animation_queries_and_sound() {
    let v = fixture();
    let model = Mom::parse(&bytes(&v["owned_model_hex"])).unwrap();
    let math = math(&v);
    let aim = aim(&v);
    for case in v["cases"].as_array().unwrap() {
        compare_case(case, &model, &math, &aim, owned_tables(&v));
    }
}
#[test]
#[ignore = "Requires original additional-action models/properties/math and private original CPU observations"]
fn canonical_additional_actions_actions_match_original_cpu() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let v: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ADDITIONAL_ACTIONS_NPCS")
                .expect("Set LSD_NATIVE_ADDITIONAL_ACTIONS_NPCS"),
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
        compare_case(
            case,
            &Mom::parse(&source[at..end]).unwrap(),
            &math,
            &aim,
            lsd_assets::load_scene_scale_tables(&exe).unwrap(),
        );
    }
}
fn compare_encounter_case(
    case: &Value,
    model: std::sync::Arc<Mom>,
    math: &PsxMathTables,
    aim: &PsxAimTables,
    tables: lsd_core::SceneScaleTables,
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
        scene_scale: lsd_core::SceneScale,
        scale_calls: Vec<Value>,
        callback_calls: Vec<Value>,
        sound: Sound,
    }
    impl EncounterEventHost for Events<'_> {
        fn restore_player_callbacks(&mut self) -> Result<(), BehaviorFault> {
            self.callback_calls.push(json!(["set", 1, 1]));
            self.runtime.dispatch(self.math, |dispatcher| {
                dispatcher.restore_player_callbacks()
            })
        }
        fn clear_player_callbacks(&mut self, with_view: bool) -> Result<(), BehaviorFault> {
            self.callback_calls
                .push(json!(["clear", i32::from(with_view)]));
            self.runtime.dispatch(self.math, |dispatcher| {
                dispatcher.clear_player_callbacks(with_view)
            })
        }
        fn request_scene_scale(&mut self, amount: i32, mode: i32) -> Result<(), BehaviorFault> {
            self.scale_calls.push(json!([amount, mode]));
            self.runtime.scene_scale = Some(self.scene_scale.clone());
            let result = self.runtime.dispatch(self.math, |dispatcher| {
                dispatcher.request_scene_scale(amount, mode)
            });
            self.scene_scale = self.runtime.scene_scale.take().unwrap();
            result
        }
        fn day_number(&mut self) -> Result<i32, BehaviorFault> {
            self.runtime
                .dispatch(self.math, |dispatcher| dispatcher.day_number())
        }
        fn dispatch(
            &mut self,
            event: EncounterNotification,
            fields: [i8; 12],
            rng: &mut PsxRng,
        ) -> Result<(), lsd_core::DayFault> {
            let EncounterNotificationKind::Notify(code) = event.kind else {
                panic!("Unexpected event {event:?}")
            };
            self.journal.push(code);
            // Perception notifications are recorded at the entity boundary;
            // original day/recall consequences have their own identified-state
            // comparisons. These cases also replay the action's direct link.
            if !matches!(code, 10..=12) {
                return Ok(());
            }
            self.runtime.allow_journal = true;
            let result = self.runtime.dispatch(self.math, |dispatcher| {
                dispatcher.dispatch(event, fields, rng)
            });
            self.sound.journal(&self.runtime.journal_audio);
            self.runtime.journal_audio.clear();
            result
        }
    }

    let callback = match case["callback"].as_str().unwrap() {
        "entity_ship" => 0x80061e60,
        "entity_maiko_palace" => 0x80061f30,
        "entity_acrobats" => 0x8006204c,
        "entity_light" => 0x800620c4,
        "entity_ring" => 0x800624bc,
        "entity_hoop_girl" => 0x80062570,
        "entity_hoop_girl_giant" => 0x80064ca4,
        "entity_elephant_small" => 0x80064cec,
        "entity_maiko_bridge" => 0x80064d48,
        "entity_question_mark_penguin" => 0x800650d4,
        "entity_standing_penguin_flock" => 0x800650f4,
        "entity_triangle_head_giant" => 0x8006519c,
        _ => unreachable!(),
    };
    assert_eq!(
        lsd_core::NpcScript::from_reference_address(Some(callback)),
        script(case["callback"].as_str().unwrap())
    );
    let fields = array::<12>(&case["fields"]).map(|v| v as i8);
    let mut sound = Sound::new(41);
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
    behavior.state = case["initial_state"].as_i64().unwrap() as i32;
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
        scene_scale: lsd_core::SceneScale::new(tables),
        scale_calls: vec![],
        callback_calls: vec![],
        sound: sound.clone(),
        math,
    };
    let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
    let host = host(case, fields, math, aim, tables, sound.clone());
    for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
        sound.clear();
        events.journal.clear();
        events.runtime.journal_audio.clear();
        events.scale_calls.clear();
        events.callback_calls.clear();
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
        actual["player_callbacks"] = callbacks(&events.runtime);
        actual["callback_calls"] = json!(events.callback_calls);
        actual["audio"] = json!(sound.events());
        actual["scene_request"] =
            json!([events.scene_scale.remaining, events.scene_scale.selected]);
        actual["scale_calls"] = json!(events.scale_calls);
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
fn additional_actions_encounter_adapter_matches_owned_complete_updates() {
    let v: Value = serde_json::from_str(include_str!(
        "fixtures/additional-actions-npc-encounters.json"
    ))
    .unwrap();
    let model = std::sync::Arc::new(Mom::parse(&bytes(&v["owned_model_hex"])).unwrap());
    let math = math(&v);
    let aim = aim(&v);
    for case in v["cases"].as_array().unwrap() {
        compare_encounter_case(
            case,
            std::sync::Arc::clone(&model),
            &math,
            &aim,
            owned_tables(&v),
        );
    }
}
#[test]
#[ignore = "Requires original NPC models/properties/math and complete original entity-update observations"]
fn canonical_additional_actions_encounter_updates_match_original_cpu() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let v: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ADDITIONAL_ACTIONS_ENCOUNTERS")
                .expect("Set LSD_NATIVE_ADDITIONAL_ACTIONS_ENCOUNTERS"),
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
            lsd_assets::load_scene_scale_tables(&exe).unwrap(),
        );
    }
}

fn owned_tables(value: &Value) -> lsd_core::SceneScaleTables {
    lsd_core::SceneScaleTables {
        packets: std::array::from_fn(|row| {
            std::array::from_fn(|axis| {
                value["owned_scale_packets"][row][axis].as_u64().unwrap() as u32
            })
        }),
    }
}

fn callbacks(runtime: &DayRuntime) -> Value {
    json!([
        runtime.move_callback,
        runtime.view_callback,
        runtime.move_callback != 0,
        runtime.view_callback != 0,
        runtime.reset_view_pending,
        runtime.view_active
    ])
}
