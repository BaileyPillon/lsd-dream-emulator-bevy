use lsd_assets::{
    ActorFrame, EncounterEventHost, EncounterNotification, EncounterScene, EncounterSubscription,
    EntityProperties, SceneActor, SceneClockMember, formats::Mom,
};
use lsd_core::{
    EncounterTables, EntityBehavior, EntityTarget, FramePhase, FrameSubscribers, GenerationSound,
    NpcScript, PsxMathTables, PsxRng,
};
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Default)]
struct Sound(Vec<Value>);
impl GenerationSound for Sound {
    fn stop_note(&mut self, handle: i32) -> i32 {
        self.0.push(json!(["stop", handle]));
        -1
    }
    fn volume_offset(&mut self, offset: i32) {
        self.0.push(json!(["volume", offset]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        self.0.push(json!(["play", note, period, counter]));
        -1
    }
}
struct Events;
impl EncounterEventHost for Events {
    fn dispatch(
        &mut self,
        event: EncounterNotification,
        _fields: [i8; 12],
        _rng: &mut PsxRng,
    ) -> Result<(), lsd_core::DayFault> {
        panic!("controlled outside-range graph emitted {event:?}");
    }
}
fn sync(scene: &mut EncounterScene, members: &mut FrameSubscribers<SceneClockMember>) {
    for event in scene.take_frame_changes() {
        match event {
            EncounterSubscription::Attach(serial) => {
                members.subscribe(SceneClockMember::Actor(serial))
            }
            EncounterSubscription::Detach(serial) => {
                members.remove(SceneClockMember::Actor(serial));
            }
            EncounterSubscription::AttachEffect(serial) => {
                members.subscribe(SceneClockMember::ActorEffect(serial))
            }
            EncounterSubscription::DetachEffect(serial) => {
                members.remove(SceneClockMember::ActorEffect(serial));
            }
        }
    }
}
fn role(member: SceneClockMember) -> String {
    match member {
        SceneClockMember::Actor(serial) => format!("actor:{}", serial - 1),
        SceneClockMember::Stream => "stream".into(),
        SceneClockMember::Renderer => "renderer".into(),
        SceneClockMember::Session => "session".into(),
        SceneClockMember::Player => "player".into(),
        _ => unreachable!(),
    }
}
fn actors(scene: &EncounterScene, count: usize) -> Vec<Value> {
    (1..=count as u64).map(|serial| {
        scene.actors.iter().find(|actor| actor.serial == serial).map_or(Value::Null, |entry| {
            let actor=&entry.actor; let behavior=&entry.behavior;
            json!({"interaction_timer":actor.animation.ticks,"behavior_tick":behavior.ticks,"state":behavior.state,
                "triggered":behavior.triggered,"link_enabled":behavior.link_enabled,"behavior_active":behavior.active,
                "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
                "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
                "part_attributes":actor.animation.parts.iter().map(|part|part.attributes).collect::<Vec<_>>(),
                "part_positions":actor.animation.parts.iter().map(|part|part.coordinate_translation).collect::<Vec<_>>(),
                "context":behavior.context.words})
        })
    }).collect()
}
#[test]
fn original_stream_birth_first_update_rng_and_finish_order_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/shared-clock-birth.json")).unwrap();
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let resource = Arc::new(Mom::parse(&bytes).unwrap());
    let mut math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    math.sin_cos[0] = 4096 << 16;
    let target = EntityTarget {
        position: [5000, 0, 0],
        local_y: 0,
        matrix: lsd_core::FixedMatrix::IDENTITY,
        eye: [0, -512, 0],
        target: [0, -512, 1024],
    };
    for (case_number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
        let mut clock = FramePhase::default();
        let mut members = FrameSubscribers::default();
        for member in [
            SceneClockMember::Player,
            SceneClockMember::Session,
            SceneClockMember::Renderer,
            SceneClockMember::Stream,
        ] {
            members.subscribe(member);
        }
        let mut scene = EncounterScene::new(
            EncounterTables {
                chunks: std::array::from_fn(|_| vec![]),
                objects: std::array::from_fn(|_| vec![]),
                color_lookup: vec![],
                variants: vec![],
                positions: vec![],
                rotations: vec![],
            },
            vec![],
        );
        let mut sound = Sound::default();
        let mut born = false;
        for (step_number, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            sound.0.clear();
            clock.waiting = step["waiting"].as_i64().unwrap() as i32;
            clock.finished = step["finished"].as_i64().unwrap() as i32;
            let signal = clock.advance();
            let mut visits = vec![];
            members.begin_notification();
            while let Some(member) = members.next_subscriber() {
                visits.push(role(member));
                match member {
                    SceneClockMember::Actor(serial) => scene
                        .tick_actor_frame(
                            ActorFrame {
                                serial,
                                target,
                                signal,
                            },
                            &mut rng,
                            &mut sound,
                            &math,
                            &mut Events,
                        )
                        .unwrap(),
                    SceneClockMember::Stream if !born => {
                        let fields =
                            std::array::from_fn(|i| input["fields"][i].as_i64().unwrap() as i8);
                        for (index, callback) in
                            input["callbacks"].as_array().unwrap().iter().enumerate()
                        {
                            let script = match callback.as_str() {
                                Some("entity_television") => NpcScript::Television,
                                Some("entity_futon") => NpcScript::Futon,
                                None => NpcScript::None,
                                _ => unreachable!(),
                            };
                            let mut actor = resource.construct_actor().unwrap();
                            let behavior = EntityBehavior::construct(
                                17 + index as u8,
                                script,
                                fields,
                                &mut actor,
                                &mut sound,
                            );
                            let collision = resource.collision_model(&actor).unwrap();
                            scene.add_actor(SceneActor {
                                serial: 0,
                                slot: 0,
                                map: 0,
                                entity: 17 + index as u8,
                                model: 0,
                                resource: Arc::clone(&resource),
                                actor,
                                collision,
                                properties: EntityProperties {
                                    fields,
                                    callback: None,
                                },
                                behavior_active: behavior.active != 0,
                                behavior_pending: false,
                                behavior,
                            });
                        }
                        born = true;
                    }
                    _ => {}
                }
                sync(&mut scene, &mut members);
            }
            members.end_notification();
            assert_eq!(
                json!({"order":members.order().into_iter().map(role).collect::<Vec<_>>(),
                    "visits":visits,"actors":actors(&scene,3),"rng":rng.state(),"audio":sound.0,"phase":clock.phase}),
                step["expected"],
                "case {case_number} step {step_number}"
            );
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 8);
}
