use lsd_assets::{
    EncounterEventHost, EncounterNotification, EncounterScene, EntityProperties, SceneActor,
    formats::Mom,
};
use lsd_core::{
    EncounterTables, EntityBehavior, EntityTarget, FramePhase, GenerationSound, NpcScript,
    PsxMathTables, PsxRng,
};
use serde_json::{Value, json};
use std::sync::Arc;
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
struct Events;
impl EncounterEventHost for Events {
    fn dispatch(
        &mut self,
        event: EncounterNotification,
        _fields: [i8; 12],
        _rng: &mut PsxRng,
    ) -> Result<(), lsd_core::DayFault> {
        panic!("controlled clock graph should emit no journal requests: {event:?}");
    }
}
fn empty_scene() -> EncounterScene {
    EncounterScene::new(
        EncounterTables {
            chunks: std::array::from_fn(|_| vec![]),
            objects: std::array::from_fn(|_| vec![]),
            color_lookup: vec![],
            variants: vec![],
            positions: vec![],
            rotations: vec![],
        },
        vec![],
    )
}
fn snapshot(
    scene: &EncounterScene,
    frame: FramePhase,
    rng: PsxRng,
    sound: &Sound,
    order: &[usize],
    codes: &[i32],
) -> Value {
    let actors=(1..=3).map(|serial|scene.actors.iter().find(|a|a.serial==serial).map(|a| {
        let actor=&a.actor;let behavior=&a.behavior;
        json!({"interaction_timer":actor.animation.ticks,"behavior_tick":behavior.ticks,"state":behavior.state,
            "triggered":behavior.triggered,"link_enabled":behavior.link_enabled,"behavior_active":behavior.active,
            "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
            "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
            "part_attributes":actor.animation.parts.iter().map(|p|p.attributes).collect::<Vec<_>>(),
            "part_positions":actor.animation.parts.iter().map(|p|p.coordinate_translation).collect::<Vec<_>>(),
            "context":behavior.context.words})
    })).collect::<Vec<_>>();
    json!({"phase":frame.phase,"waiting":frame.waiting,"finished":frame.finished,"notify_cursor_cleared":true,
        "subscription_order":scene.frame_actor_order().iter().map(|s|s-1).collect::<Vec<_>>(),
        "update_order":order,"codes":codes,"actors":actors,"rng":rng.state(),"audio":sound.events})
}
#[test]
fn original_actor_clock_order_waiting_restart_rng_and_finish_cleanup_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/actor-frame-clock.json")).unwrap();
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
        eye: [0; 3],
        target: [0, 0, 1024],
    };
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let fields = std::array::from_fn(|i| input["fields"][i].as_i64().unwrap() as i8);
        let mut scene = empty_scene();
        let mut sound = Sound {
            voice: 41,
            events: vec![],
        };
        let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
        let mut frame = FramePhase::default();
        for (index, callback) in input["callbacks"].as_array().unwrap().iter().enumerate() {
            let script = match callback.as_str() {
                None => NpcScript::None,
                Some("entity_television") => NpcScript::Television,
                Some("entity_futon") => NpcScript::Futon,
                Some("entity_drumstick") => NpcScript::Drumstick,
                _ => unreachable!(),
            };
            let mut actor = resource.construct_actor().unwrap();
            let behavior =
                EntityBehavior::construct(17 + index as u8, script, fields, &mut actor, &mut sound);
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
        sound.events.clear();
        assert_eq!(
            snapshot(&scene, frame, rng, &sound, &[], &[]),
            case["initial"],
            "initial {number}"
        );
        for (step, entry) in case["steps"].as_array().unwrap().iter().enumerate() {
            sound.events.clear();
            let operation = &entry["operation"];
            let mut order = vec![];
            let mut codes = vec![];
            match operation["kind"].as_str().unwrap() {
                "advance" => {
                    frame.waiting = operation["waiting"].as_i64().unwrap() as i32;
                    frame.finished = operation["finished"].as_i64().unwrap() as i32;
                    let signal = frame.advance();
                    order = scene
                        .frame_actor_order()
                        .iter()
                        .map(|s| (s - 1) as usize)
                        .collect();
                    codes = vec![signal.code(); order.len()];
                    scene
                        .tick_frame(target, &mut rng, &mut sound, &math, &mut Events, signal)
                        .unwrap();
                }
                "stop" => {
                    assert!(scene.stop_actor(operation["actor"].as_u64().unwrap() + 1, &mut sound));
                }
                "restart" => {
                    assert!(scene.restart_actor(operation["actor"].as_u64().unwrap() + 1, [0; 3]));
                }
                "set_phase" => frame.phase = operation["phase"].as_i64().unwrap() as i32,
                "reset" => frame.reset(operation["phase"].as_i64().unwrap() as i32),
                _ => unreachable!(),
            }
            assert_eq!(
                snapshot(&scene, frame, rng, &sound, &order, &codes),
                entry["expected"],
                "clock case {number} step {step} {operation}"
            );
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 32);
}
