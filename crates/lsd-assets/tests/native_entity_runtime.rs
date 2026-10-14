use lsd_assets::formats::Mom;
use lsd_core::{BehaviorTick, EntityBehavior, GenerationSound, ModelActor, NpcScript, PsxRng};
use serde_json::{Value, json};
#[derive(Default)]
struct Sound {
    events: Vec<Value>,
    voice: i32,
}
impl GenerationSound for Sound {
    fn stop_note(&mut self, handle: i32) -> i32 {
        self.events.push(json!(["stop", handle]));
        -1
    }
    fn volume_offset(&mut self, offset: i32) {
        self.events.push(json!(["volume", offset]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        self.events.push(json!(["play", note, period, counter]));
        let voice = self.voice;
        self.voice += 1;
        voice
    }
}
fn snapshot(actor: &ModelActor, behavior: &EntityBehavior, rng: &PsxRng, sound: &Sound) -> Value {
    json!({"interaction_timer":actor.animation.ticks,"behavior_tick":behavior.ticks,
        "state":behavior.state,"triggered":behavior.triggered,"link_enabled":behavior.link_enabled,
        "behavior_active":behavior.active,"animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
        "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|part|part.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|part|part.coordinate_translation).collect::<Vec<_>>(),
        "context":behavior.context.words,"rng":rng.state(),"audio":sound.events})
}

#[test]
fn coupled_spawn_cpu_fixture_contains_a_valid_owned_model() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/encounter-program.json"
    ))
    .unwrap();
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let model = Mom::parse(&bytes).unwrap();
    assert_eq!(model.animations[0].frames.len(), 2);
    assert_eq!(model.initial_part_types, vec![9, 5]);
    assert_eq!(model.tmd.objects.len(), 2);
}
#[test]
fn original_entity_lifecycle_three_clocks_named_callbacks_and_animation_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/entity-runtime.json"
    ))
    .unwrap();
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let model =
        Mom::parse(&bytes).expect("Owned CPU input must decode as a valid complete MOM/TMD/MOS");
    assert_eq!(model.animations[0].frames.len(), 2);
    assert_eq!(model.tmd.objects.len(), 2);
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let script = match input["callback"].as_str() {
            None => NpcScript::None,
            Some("entity_futon") => NpcScript::Futon,
            Some("entity_television") => NpcScript::Television,
            Some("entity_drumstick") => NpcScript::Drumstick,
            _ => unreachable!(),
        };
        let fields = [
            0,
            0,
            input["unlock"].as_i64().unwrap() as i8,
            input["behaviour"].as_i64().unwrap() as i8,
            0,
            1,
            1,
            0,
            0,
            1,
            input["proximity"].as_i64().unwrap() as i8,
            input["link"].as_i64().unwrap() as i8,
        ];
        let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
        let mut actor = model.construct_actor().unwrap();
        let mut sound = Sound {
            events: vec![],
            voice: 41,
        };
        let mut behavior = EntityBehavior::construct(
            input["entity"].as_u64().unwrap() as u8,
            script,
            fields,
            &mut actor,
            &mut sound,
        );
        assert_eq!(
            snapshot(&actor, &behavior, &rng, &sound),
            case["expected"],
            "constructor case {number}"
        );
        for (step_number, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            sound.events.clear();
            match step["operation"].as_str().unwrap() {
                "tick" => behavior
                    .tick(
                        &mut actor,
                        &model.animations,
                        BehaviorTick {
                            target: std::array::from_fn(|i| {
                                step["target"][i].as_i64().unwrap() as i32
                            }),
                            proximity: fields[10],
                        },
                        &mut rng,
                        &mut sound,
                    )
                    .unwrap(),
                "start" => behavior.start_behavior(&mut actor),
                "stop" => behavior.stop_behavior(&mut actor, &mut sound),
                "disable" => behavior.disable_link(&mut actor, &mut sound),
                "enable" => behavior.enable_link(&mut actor),
                "timers" => {
                    actor.animation.ticks = u32::MAX;
                    behavior.ticks = i32::MAX;
                    behavior.context.words[1] = i32::MAX;
                }
                _ => unreachable!(),
            }
            assert_eq!(
                snapshot(&actor, &behavior, &rng, &sound),
                step["expected"],
                "case {number} step {step_number} operation {}",
                step["operation"]
            );
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 32);
}

struct InteractionHost {
    answers: Vec<bool>,
    index: usize,
    events: Vec<Value>,
}
impl lsd_core::EntityInteractionHost for InteractionHost {
    fn proximity(&mut self, position: [i32; 3], forward: i32, radius: i32) -> bool {
        self.events
            .push(json!(["proximity", position, forward, radius]));
        let result = self.answers[self.index % self.answers.len()];
        self.index += 1;
        result
    }
    fn facing_target(&mut self, _position: [i32; 3], distance: i32) -> bool {
        self.events.push(json!(["facing", distance]));
        true
    }
    fn aim_at_target(&mut self) {
        self.events.push(json!(["aim"]));
    }
    fn notify(&mut self, code: i32) {
        self.events.push(json!(["notify", code]));
    }
}
#[test]
fn original_entity_interaction_policy_proximity_arguments_and_order_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/entity-interaction.json"
    ))
    .unwrap();
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let model = Mom::parse(&bytes).unwrap();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let fields = std::array::from_fn(|i| input["fields"][i].as_i64().unwrap() as i8);
        let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
        let mut actor = model.construct_actor().unwrap();
        let mut sound = Sound {
            events: vec![],
            voice: 41,
        };
        let mut behavior =
            EntityBehavior::construct(17, NpcScript::None, fields, &mut actor, &mut sound);
        let pre = &input["pre"];
        behavior.link_enabled = pre["link"].as_i64().unwrap() as i32;
        behavior.active = pre["active"].as_i64().unwrap() as i32;
        behavior.triggered = pre["triggered"].as_i64().unwrap() as i32;
        behavior.state = pre["state"].as_i64().unwrap() as i32;
        actor.animation.ticks = pre["timer"].as_i64().unwrap() as u32;
        assert_eq!(
            snapshot(&actor, &behavior, &rng, &sound),
            case["initial"],
            "initial case {number}"
        );
        sound.events.clear();
        let mut host = InteractionHost {
            answers: input["answers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_bool().unwrap())
                .collect(),
            index: 0,
            events: vec![],
        };
        behavior.update_interaction(&mut actor, fields, &mut rng, &mut sound, &mut host);
        assert_eq!(
            snapshot(&actor, &behavior, &rng, &sound),
            case["expected"],
            "policy case {number}: {input}"
        );
        assert_eq!(
            json!(host.events),
            case["events"],
            "order case {number}: {input}"
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 288);
}

#[test]
#[ignore = "Requires canonical original NPC models and private original CPU observations"]
fn canonical_futon_and_television_models_properties_and_behavior_steps_match() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ENTITY_RUNTIME").expect("Set LSD_NATIVE_ENTITY_RUNTIME"),
        )
        .unwrap(),
    )
    .unwrap();
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let properties = lsd_assets::load_entity_properties(&executable).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let path = std::path::Path::new(input["path"].as_str().unwrap());
        assert!(
            path.components()
                .all(|part| matches!(part, std::path::Component::Normal(_)))
        );
        let data = std::fs::read(files.join(path)).unwrap();
        assert_eq!(
            lsd_assets::sha256_hex(&data),
            input["source_sha256"].as_str().unwrap()
        );
        let offset = input["offset"].as_u64().unwrap() as usize;
        let bytes = &data[offset..offset + input["bytes"].as_u64().unwrap() as usize];
        assert_eq!(
            lsd_assets::sha256_hex(bytes),
            input["mom_sha256"].as_str().unwrap()
        );
        let model = Mom::parse(bytes).unwrap();
        let fields = std::array::from_fn(|i| input["fields"][i].as_i64().unwrap() as i8);
        let entity = input["entity"].as_u64().unwrap() as u8;
        assert_eq!(fields, properties[entity as usize].fields);
        let script = NpcScript::from_reference_address(properties[entity as usize].callback);
        assert!(!script.pending());
        let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
        let mut actor = model.construct_actor().unwrap();
        let mut sound = Sound {
            events: vec![],
            voice: 41,
        };
        let mut behavior =
            EntityBehavior::construct(entity, script, fields, &mut actor, &mut sound);
        assert_eq!(snapshot(&actor, &behavior, &rng, &sound), case["expected"]);
        for step in case["steps"].as_array().unwrap() {
            sound.events.clear();
            match step["operation"].as_str().unwrap() {
                "tick" => behavior
                    .tick(
                        &mut actor,
                        &model.animations,
                        BehaviorTick {
                            target: std::array::from_fn(|i| {
                                step["target"][i].as_i64().unwrap() as i32
                            }),
                            proximity: fields[10],
                        },
                        &mut rng,
                        &mut sound,
                    )
                    .unwrap(),
                "start" => behavior.start_behavior(&mut actor),
                "stop" => behavior.stop_behavior(&mut actor, &mut sound),
                "disable" => behavior.disable_link(&mut actor, &mut sound),
                _ => unreachable!(),
            }
            assert_eq!(
                snapshot(&actor, &behavior, &rng, &sound),
                step["expected"],
                "{} {}",
                input["callback"],
                step["operation"]
            );
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 2);
}
