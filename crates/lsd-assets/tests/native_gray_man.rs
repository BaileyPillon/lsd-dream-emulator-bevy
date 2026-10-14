use lsd_assets::{
    EncounterEventHost, EncounterNotification, EncounterNotificationKind, EncounterScene,
    EncounterSubscription, EntityProperties, SceneClockMember, SpecialEncounterAssets,
    formats::Mom,
};
use lsd_core::{
    EntityEffectDefinition, FixedMatrix, FramePhase, FrameSubscribers, GenerationSound,
    PsxAimTables, PsxMathTables, PsxRng,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

#[derive(Default)]
struct Sound {
    events: Vec<Value>,
}
impl GenerationSound for Sound {
    fn stop_note(&mut self, handle: i32) -> i32 {
        self.events.push(json!(["stop", handle]));
        -1
    }
    fn volume_offset(&mut self, value: i32) {
        self.events.push(json!(["volume", value]));
    }
    fn play_note(&mut self, a: i32, b: i32, c: i32) -> i32 {
        self.events.push(json!(["play", a, b, c]));
        -1
    }
}
struct Events {
    count: i32,
    index: i32,
    records: Vec<u8>,
    events: Vec<Value>,
}
impl EncounterEventHost for Events {
    fn actor_destroyed(&mut self, _serial: u64, _entity: u8) -> Result<(), lsd_core::DayFault> {
        self.events.push(json!(["actor_notify", 1]));
        Ok(())
    }
    fn dispatch(
        &mut self,
        event: EncounterNotification,
        _fields: [i8; 12],
        _rng: &mut PsxRng,
    ) -> Result<(), lsd_core::DayFault> {
        match event.kind {
            EncounterNotificationKind::Notify(code) => {
                self.events.push(json!(["actor_notify", code]))
            }
            EncounterNotificationKind::ResetFlashbackCount => {
                self.count = 0;
                self.events.push(json!("reset_flashback_count"));
            }
            other => panic!("Unexpected Gray Man service: {other:?}"),
        }
        Ok(())
    }
}
fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn sync(scene: &mut EncounterScene, members: &mut FrameSubscribers<SceneClockMember>) {
    for change in scene.take_frame_changes() {
        match change {
            EncounterSubscription::Attach(s) => members.subscribe(SceneClockMember::Actor(s)),
            EncounterSubscription::Detach(s) => {
                members.remove(SceneClockMember::Actor(s));
            }
            EncounterSubscription::AttachEffect(s) => {
                members.subscribe(SceneClockMember::ActorEffect(s))
            }
            EncounterSubscription::DetachEffect(s) => {
                members.remove(SceneClockMember::ActorEffect(s));
            }
        }
    }
}
fn role(member: SceneClockMember) -> &'static str {
    match member {
        SceneClockMember::Actor(_) => "gray",
        SceneClockMember::ActorEffect(_) => "effect",
        SceneClockMember::Player => "player",
        _ => panic!("Unexpected member"),
    }
}
fn snapshot(
    scene: &EncounterScene,
    members: &FrameSubscribers<SceneClockMember>,
    sound: &Sound,
    events: &Events,
    rng: &PsxRng,
    visits: &[&str],
) -> Value {
    let actor = scene.actors.first();
    let entity=actor.map(|a| {
        let actor=&a.actor;let b=&a.behavior;
        json!({"interaction_timer":actor.animation.ticks,"behavior_tick":b.ticks,"state":b.state,
            "triggered":b.triggered,"link_enabled":b.link_enabled,"behavior_active":b.active,
            "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
            "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
            "part_attributes":actor.animation.parts.iter().map(|p|p.attributes).collect::<Vec<_>>(),
            "part_positions":actor.animation.parts.iter().map(|p|p.coordinate_translation).collect::<Vec<_>>(),"context":b.context.words})
    });
    let effect=actor.and_then(|a|a.behavior.effect.as_ref()).map(|e| {
        let s=&e.state;
        json!({"color":s.color,"render_flags":s.render_flags,"color_mask":s.color_mask,"state":s.state,
            "variant":s.variant,"color_step":s.color_step,"channels":s.channels,"mode":s.mode,"life":s.life,
            "mask_step":s.mask_step,"linked":s.linked,"linked_divisor":s.linked_divisor,
            "owner_bound":e.owner_bound,"clock_attached":e.clock_attached,"packet":e.packet([320,240]).map(|p|p.words())})
    });
    let pose=actor.map(|a|json!({"position":a.actor.pose.position,"scale":a.actor.pose.scale,"angles":a.actor.pose.angles,
        "attributes":a.actor.attributes,"tick":a.actor.animation.ticks,"mode":0,"parent":true}));
    json!({"actor":entity,"pose":pose,"effect":effect,"order":members.order().into_iter().map(role).collect::<Vec<_>>(),
        "flashback_count":events.count,"flashback_index":events.index,"records_sha256":lsd_assets::sha256_hex(&events.records),
        "rng":rng.state(),"audio":sound.events,"events":events.events,"visits":visits})
}
fn compare(
    fixture: &Value,
    case: &Value,
    assets: SpecialEncounterAssets,
    definition: EntityEffectDefinition,
    math: &PsxMathTables,
) {
    let tables = lsd_core::EncounterTables {
        chunks: std::array::from_fn(|_| Vec::new()),
        objects: std::array::from_fn(|_| Vec::new()),
        variants: vec![],
        positions: vec![],
        rotations: vec![],
        color_lookup: vec![],
    };
    let fields = std::array::from_fn(|i| fixture["fields"][i].as_i64().unwrap() as i8);
    let mut properties = vec![
        EntityProperties {
            fields: [0; 12],
            callback: None
        };
        130
    ];
    properties[98] = EntityProperties {
        fields,
        callback: Some(0x8006_4618),
    };
    let mut scene = EncounterScene::new(tables, properties);
    scene.enable_special_encounter(assets);
    scene.enable_entity_effects(definition);
    let mut sound = Sound::default();
    let mut rng = PsxRng::new(1);
    let mut phase = FramePhase::default();
    let mut events = Events {
        count: 10,
        index: 7,
        records: bytes(fixture["records_hex"].as_str().unwrap()),
        events: vec![],
    };
    scene.set_encounter_player([5000, 0, 0], FixedMatrix::IDENTITY);
    scene.activate_gray_man(&mut sound).unwrap();
    sound.events.clear();
    let mut members = FrameSubscribers::default();
    members.subscribe(SceneClockMember::Player);
    sync(&mut scene, &mut members);
    assert_eq!(
        snapshot(&scene, &members, &sound, &events, &rng, &[]),
        case["initial"],
        "initial {}",
        case["name"]
    );
    let target = lsd_core::EntityTarget {
        position: [5000, 0, 0],
        local_y: 0,
        matrix: FixedMatrix::IDENTITY,
        eye: [0; 3],
        target: [0, 0, 1024],
    };
    for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
        sound.events.clear();
        events.events.clear();
        let mut visits = Vec::new();
        let op = &step["operation"];
        match op["kind"].as_str().unwrap() {
            "trigger" => scene.actors[0].behavior.triggered = 1,
            "handles" => {
                for (word, handle) in [(6, 41), (11, 42), (16, 43)] {
                    scene.actors[0].behavior.context.words[word] = handle;
                }
            }
            "restart" => scene.activate_gray_man(&mut sound).unwrap(),
            "actor_tick" => {
                // Allocation failure is a supplied allocator boundary in this cohort.
                if op["allocation_failure"].as_bool() == Some(true) {
                    struct FailureHost<'a>(&'a mut Events, &'a PsxMathTables);
                    impl lsd_core::EntityScriptHost for FailureHost<'_> {
                        fn movement_matrix(
                            &mut self,
                            a: [i16; 3],
                        ) -> Result<FixedMatrix, lsd_core::BehaviorFault> {
                            Ok(self.1.rotation(a))
                        }
                        fn notify(
                            &mut self,
                            _r: &mut PsxRng,
                            c: i32,
                        ) -> Result<(), lsd_core::BehaviorFault> {
                            self.0.events.push(json!(["actor_notify", c]));
                            Ok(())
                        }
                        fn start_entity_effect(
                            &mut self,
                            _e: &mut Option<lsd_core::EntityEffect>,
                            _a: i32,
                            _b: i32,
                            _c: i32,
                        ) -> Result<bool, lsd_core::BehaviorFault> {
                            Ok(false)
                        }
                    }
                    let actor = &mut scene.actors[0];
                    actor
                        .behavior
                        .tick_with_services(
                            &mut actor.actor,
                            &actor.resource.animations,
                            lsd_core::BehaviorTick {
                                target: target.position,
                                proximity: 0,
                            },
                            &mut rng,
                            &mut sound,
                            &mut FailureHost(&mut events, math),
                        )
                        .unwrap();
                } else {
                    let serial = scene.actors[0].serial;
                    scene
                        .tick_actor_frame(
                            lsd_assets::ActorFrame {
                                serial,
                                target,
                                signal: lsd_core::FrameSignal::Tick,
                            },
                            &mut rng,
                            &mut sound,
                            math,
                            &mut events,
                        )
                        .unwrap();
                }
            }
            "clock" => {
                phase.waiting = op["waiting"].as_i64().unwrap_or(0) as i32;
                phase.finished = op["finished"].as_i64().unwrap_or(0) as i32;
                let signal = phase.advance();
                members.begin_notification();
                while let Some(member) = members.next_subscriber() {
                    visits.push(role(member));
                    match member {
                        SceneClockMember::Actor(serial) => scene
                            .tick_actor_frame(
                                lsd_assets::ActorFrame {
                                    serial,
                                    target,
                                    signal,
                                },
                                &mut rng,
                                &mut sound,
                                math,
                                &mut events,
                            )
                            .unwrap(),
                        SceneClockMember::ActorEffect(serial) => {
                            scene.tick_actor_effect(serial, signal)
                        }
                        SceneClockMember::Player => {}
                        _ => unreachable!(),
                    }
                    for (_, code) in scene.take_effect_notifications() {
                        events.events.push(json!(["effect_notify", code]));
                    }
                    sync(&mut scene, &mut members);
                }
                members.end_notification();
            }
            other => panic!("Unknown operation {other}"),
        }
        sync(&mut scene, &mut members);
        assert_eq!(
            snapshot(&scene, &members, &sound, &events, &rng, &visits),
            step["state"],
            "{} step {index} {op}",
            case["name"]
        );
    }
}
#[test]
fn owned_gray_callback_clock_effect_reuse_packets_failure_and_cleanup_match_original_cpu() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/gray-man-runtime.json")).unwrap();
    let palette = lsd_core::EffectPalette {
        start: std::array::from_fn(|i| {
            std::array::from_fn(|j| fixture["owned_palette"]["start"][i][j].as_u64().unwrap() as u8)
        }),
        attached: std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                fixture["owned_palette"]["attached"][i][j].as_u64().unwrap() as u8
            })
        }),
    };
    let definition = EntityEffectDefinition {
        palette,
        size: [320, 240],
        offset: [-100, -100],
    };
    let assets = SpecialEncounterAssets {
        model: Arc::new(Mom::parse(&bytes(fixture["owned_model_hex"].as_str().unwrap())).unwrap()),
        path: PathBuf::from("owned.MOM"),
        offset: [0, -200, 8000],
        aim: PsxAimTables {
            atan: Box::new(std::array::from_fn(|i| (i / 2) as i16)),
        },
    };
    let mut math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    math.sin_cos[0] = 4096 << 16;
    for case in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["canonical"] == false)
    {
        compare(&fixture, case, assets.clone(), definition.clone(), &math);
    }
}
#[test]
#[ignore = "Requires canonical executable and the original Gray Man model and palettes"]
fn canonical_gray_callback_and_retained_effect_match_original_cpu() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let exe = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/gray-man-runtime.json")).unwrap();
    let assets = lsd_assets::load_special_encounter_assets(&exe, &files).unwrap();
    assert_eq!(
        lsd_assets::sha256_hex(&std::fs::read(files.join(&assets.path)).unwrap()),
        fixture["canonical_model_sha256"].as_str().unwrap()
    );
    let definition = lsd_assets::load_entity_effect_definition(&exe).unwrap();
    let math = lsd_assets::load_math_tables(&exe).unwrap();
    for case in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["canonical"] == true)
    {
        compare(&fixture, case, assets.clone(), definition.clone(), &math);
    }
}
