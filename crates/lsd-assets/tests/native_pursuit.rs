use lsd_assets::formats::Mom;
use lsd_core::{
    BehaviorTick, EntityBehavior, EntityInteractionHost, EntityScriptHost, FixedMatrix,
    GenerationSound, ModelActor, NpcScript, PsxAimTables, PsxMathTables, PsxRng,
};
use serde_json::{Value, json};
use std::path::PathBuf;
#[derive(Default)]
struct Sound {
    events: Vec<Value>,
    voice: i32,
}
impl GenerationSound for Sound {
    fn stop_note(&mut self, h: i32) -> i32 {
        self.events.push(json!(["stop", h]));
        -1
    }
    fn volume_offset(&mut self, v: i32) {
        self.events.push(json!(["volume", v]));
    }
    fn play_note(&mut self, a: i32, b: i32, c: i32) -> i32 {
        self.events.push(json!(["play", a, b, c]));
        let out = self.voice;
        self.voice += 1;
        out
    }
}
struct Host<'a> {
    math: &'a PsxMathTables,
    aim: &'a PsxAimTables,
    target: [i32; 3],
    notifications: Vec<i32>,
    aim_calls: u32,
}
impl EntityScriptHost for Host<'_> {
    fn face_player(&mut self, actor: &mut ModelActor) -> Result<(), lsd_core::BehaviorFault> {
        actor.pose.angles = self
            .aim
            .object_angles(actor.pose.position, self.target, true, false)
            .map_err(lsd_core::BehaviorFault::Aim)?;
        actor.animation.actor_coordinate_cache = 0;
        self.aim_calls += 1;
        Ok(())
    }
    fn movement_matrix(&mut self, a: [i16; 3]) -> Result<FixedMatrix, lsd_core::BehaviorFault> {
        Ok(self.math.rotation(a))
    }
    fn notify(&mut self, _r: &mut PsxRng, c: i32) -> Result<(), lsd_core::BehaviorFault> {
        self.notifications.push(c);
        Ok(())
    }
}
impl EntityInteractionHost for Host<'_> {
    fn proximity(&mut self, position: [i32; 3], forward: i32, radius: i32) -> bool {
        lsd_core::EntityTarget {
            position: self.target,
            local_y: 17,
            matrix: FixedMatrix::IDENTITY,
            eye: [0; 3],
            target: [0, 0, 1024],
        }
        .proximity(position, forward, radius)
        .unwrap()
    }
    fn facing_target(&mut self, _p: [i32; 3], _d: i32) -> bool {
        panic!("This cohort has no facing gate")
    }
    fn aim_at_target(&mut self, _r: &mut PsxRng) {
        panic!("The actor must be supplied for original aim")
    }
    fn aim_at_target_actor(&mut self, a: &mut ModelActor, _r: &mut PsxRng) {
        EntityScriptHost::face_player(self, a).unwrap();
    }
    fn notify(&mut self, _r: &mut PsxRng, c: i32) {
        self.notifications.push(c);
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/pursuit-runtime.json")).unwrap()
}
fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn array<const N: usize>(v: &Value) -> [i32; N] {
    std::array::from_fn(|i| v[i].as_i64().unwrap() as i32)
}
fn script(name: &str) -> NpcScript {
    match name {
        "entity_bear" => NpcScript::Bear,
        "entity_rabbit" => NpcScript::Rabbit,
        "entity_peacock" => NpcScript::Peacock,
        _ => panic!("Unknown script"),
    }
}
fn state(
    actor: &ModelActor,
    b: &EntityBehavior,
    sound: &Sound,
    host: &Host<'_>,
    rng: &PsxRng,
) -> Value {
    json!({"interaction_timer":actor.animation.ticks,"behavior_tick":b.ticks,"state":b.state,"triggered":b.triggered,
        "link_enabled":b.link_enabled,"behavior_active":b.active,"animation_enabled":actor.animation.enabled,
        "action_enabled":actor.animation.action_enabled,"clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|p|p.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|p|p.coordinate_translation).collect::<Vec<_>>(),
        "part_angles":actor.animation.parts.iter().map(|p|p.angles).collect::<Vec<_>>(),"context":b.context.words,
        "rng":rng.state(),"audio":sound.events,"root_pose":{"position":actor.pose.position,"angles":actor.pose.angles,"scale":actor.pose.scale},
        "move_distance":actor.move_distance,"cache":actor.animation.actor_coordinate_cache,"notifications":host.notifications,"aim_calls":host.aim_calls})
}
fn construct(
    model: &Mom,
    entity: u8,
    name: &str,
    fields: [i8; 12],
    sound: &mut Sound,
) -> (ModelActor, EntityBehavior) {
    let mut actor = model.construct_actor().unwrap();
    let mut b = EntityBehavior::construct(entity, script(name), fields, &mut actor, sound);
    b.enable_link(&mut actor);
    b.start_behavior(&mut actor);
    actor.pose.position = [100, 200, 300];
    actor.pose.angles = [-512, 333, 99];
    actor.animation.actor_coordinate_cache = 1;
    sound.events.clear();
    (actor, b)
}
fn compare(
    case: &Value,
    model: &Mom,
    fields: [i8; 12],
    entity: u8,
    math: &PsxMathTables,
    aim: &PsxAimTables,
) {
    let name = case["callback"].as_str().unwrap();
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let (mut actor, mut b) = construct(model, entity, name, fields, &mut sound);
    b.ticks = case["tick"].as_i64().unwrap() as i32;
    b.state = case["initial_state"].as_i64().unwrap_or(0) as i32;
    let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
    let mut host = Host {
        math,
        aim,
        target: [10000, 0, 10000],
        notifications: vec![],
        aim_calls: 0,
    };
    assert_eq!(
        state(&actor, &b, &sound, &host, &rng),
        case["initial"],
        "{name} initial"
    );
    for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
        host.target = array(&step["target"]);
        host.notifications.clear();
        host.aim_calls = 0;
        sound.events.clear();
        b.tick_with_services(
            &mut actor,
            &model.animations,
            BehaviorTick {
                target: host.target,
                proximity: fields[10],
            },
            &mut rng,
            &mut sound,
            &mut host,
        )
        .unwrap();
        assert_eq!(
            state(&actor, &b, &sound, &host, &rng),
            step["state"],
            "{name} tick {} step {i}",
            case["tick"]
        );
    }
}
#[test]
fn owned_animal_actions_and_continuous_facing_match_original_cpu() {
    let f = fixture();
    let model = Mom::parse(&bytes(f["owned_model_hex"].as_str().unwrap())).unwrap();
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            f["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new([0; 512]),
    };
    let aim = PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| {
            f["owned_atan"][i].as_i64().unwrap() as i16
        })),
    };
    for case in f["cases"].as_array().unwrap() {
        compare(
            case,
            &model,
            array::<12>(&case["fields"]).map(|v| v as i8),
            17,
            &math,
            &aim,
        );
    }
    for case in f["policy_cases"].as_array().unwrap() {
        let fields = array::<12>(&case["input"]["fields"]).map(|v| v as i8);
        let mut sound = Sound {
            events: vec![],
            voice: 41,
        };
        let (mut actor, mut b) = construct(&model, 17, "entity_peacock", fields, &mut sound);
        actor.pose.angles = [123, 456, 789];
        b.link_enabled = case["input"]["link"].as_i64().unwrap() as i32;
        b.triggered = case["input"]["trigger"].as_i64().unwrap() as i32;
        let mut rng = PsxRng::new(1);
        let mut host = Host {
            math: &math,
            aim: &aim,
            target: array(&case["input"]["target"]),
            notifications: vec![],
            aim_calls: 0,
        };
        assert_eq!(state(&actor, &b, &sound, &host, &rng), case["initial"]);
        b.update_interaction(&mut actor, fields, &mut rng, &mut sound, &mut host);
        assert_eq!(
            state(&actor, &b, &sound, &host, &rng),
            case["state"],
            "policy {}",
            case["input"]
        );
    }
}
#[test]
#[ignore = "Requires original game files and privately recorded original animal model comparisons"]
fn canonical_animal_actions_match_original_cpu() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let cases: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_NATIVE_PURSUIT").expect("Set LSD_NATIVE_PURSUIT"))
            .unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let math = lsd_assets::load_math_tables(&exe).unwrap();
    let aim = lsd_assets::load_special_encounter_assets(&exe, &files)
        .unwrap()
        .aim;
    for case in cases["cases"].as_array().unwrap() {
        let b = &case["binding"];
        let source = std::fs::read(files.join(b["path"].as_str().unwrap())).unwrap();
        assert_eq!(
            lsd_assets::sha256_hex(&source),
            b["source_sha256"].as_str().unwrap()
        );
        let at = b["offset"].as_u64().unwrap() as usize;
        let end = at + b["bytes"].as_u64().unwrap() as usize;
        assert_eq!(
            lsd_assets::sha256_hex(&source[at..end]),
            b["mom_sha256"].as_str().unwrap()
        );
        let model = Mom::parse(&source[at..end]).unwrap();
        compare(
            case,
            &model,
            array::<12>(&b["fields"]).map(|v| v as i8),
            b["entity"].as_u64().unwrap() as u8,
            &math,
            &aim,
        );
    }
}

#[test]
fn encounter_adapter_applies_facing_while_waiting_then_runs_original_peacock_action() {
    use lsd_assets::{
        EncounterEventHost, EncounterNotification, EncounterScene, EntityProperties, SceneActor,
    };
    use std::sync::Arc;
    struct Events;
    impl EncounterEventHost for Events {
        fn dispatch(
            &mut self,
            e: EncounterNotification,
            _f: [i8; 12],
            _r: &mut PsxRng,
        ) -> Result<(), lsd_core::DayFault> {
            panic!("Far supplied target emitted {e:?}");
        }
    }
    let f = fixture();
    let resource = Arc::new(Mom::parse(&bytes(f["owned_model_hex"].as_str().unwrap())).unwrap());
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            f["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new([0; 512]),
    };
    let aim = PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| {
            f["owned_atan"][i].as_i64().unwrap() as i16
        })),
    };
    let case = f["policy_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["input"]["link"] == 1
                && c["input"]["trigger"] == 0
                && c["input"]["fields"][6] == -1
                && c["input"]["target"] == json!([10000, 0, 10000])
        })
        .unwrap();
    let fields = array::<12>(&case["input"]["fields"]).map(|v| v as i8);
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let (mut actor, b) = construct(&resource, 17, "entity_peacock", fields, &mut sound);
    actor.pose.angles = [123, 456, 789];
    let mut scene = EncounterScene::new(
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
    scene.enable_aim(aim.clone());
    let serial = scene.add_actor(SceneActor {
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
            callback: Some(0x8005_f1a8),
        },
        behavior_active: true,
        behavior_pending: false,
        behavior: b,
    });
    let target = lsd_core::EntityTarget {
        position: [10000, 0, 10000],
        local_y: 17,
        matrix: FixedMatrix::IDENTITY,
        eye: [0; 3],
        target: [0, 0, 1024],
    };
    let mut rng = PsxRng::new(1);
    let host = Host {
        math: &math,
        aim: &aim,
        target: target.position,
        notifications: vec![],
        aim_calls: 1,
    };
    scene
        .tick_actor_frame(
            lsd_assets::ActorFrame {
                serial,
                target,
                signal: lsd_core::FrameSignal::Waiting,
            },
            &mut rng,
            &mut sound,
            &math,
            &mut Events,
        )
        .unwrap();
    let actor = &scene.actors[0];
    let mut observed = state(&actor.actor, &actor.behavior, &sound, &host, &rng);
    let mut expected = case["state"].clone();
    // This adapter exposes resulting state rather than native call instrumentation.
    observed.as_object_mut().unwrap().remove("aim_calls");
    expected.as_object_mut().unwrap().remove("aim_calls");
    assert_eq!(observed, expected);
    assert!(scene.pending_notifications.is_empty());
    scene
        .tick_actor_frame(
            lsd_assets::ActorFrame {
                serial,
                target,
                signal: lsd_core::FrameSignal::Tick,
            },
            &mut rng,
            &mut sound,
            &math,
            &mut Events,
        )
        .unwrap();
    let action = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["callback"] == "entity_peacock" && c["seed"] == 1)
        .unwrap();
    let actor = &scene.actors[0];
    let mut observed = state(&actor.actor, &actor.behavior, &sound, &host, &rng);
    let mut expected = action["steps"][0]["state"].clone();
    observed.as_object_mut().unwrap().remove("aim_calls");
    expected.as_object_mut().unwrap().remove("aim_calls");
    assert_eq!(observed, expected);
    assert!(scene.pending_notifications.is_empty());
}
