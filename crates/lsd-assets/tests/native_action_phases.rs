#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use lsd_assets::{
    ActorJournalBindings, ActorJournalDispatcher, ActorJournalPlayerMotion, ActorJournalServices,
    EncounterEventHost, formats::Mom,
};
use lsd_core::{
    ActorAxis, ActorJournalEvent, ActorJournalHost, BehaviorFault, BehaviorTick, DayFault,
    DayTables, DreamDay, DreamLinkEffects, DreamLinks, EntityBehavior, EntityScriptHost,
    FixedMatrix, GenerationSound, LinkCatalog, LinkContext, LinkExecution, LinkPalette,
    MapLinkPolicy, ModelActor, Mood, NpcScript, PlayerControls, PsxAimTables, PsxMathTables,
    PsxRng, SceneGrid, SpawnLocation, actor_journal,
};
use serde_json::{Value, json};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Default)]
struct SoundState {
    events: Vec<Value>,
    voice: i32,
}
#[derive(Clone, Default)]
struct Sound(Rc<RefCell<SoundState>>);
impl GenerationSound for Sound {
    fn stop_note(&mut self, handle: i32) -> i32 {
        self.0.borrow_mut().events.push(json!(["stop", handle]));
        -1
    }
    fn volume_offset(&mut self, offset: i32) {
        self.0.borrow_mut().events.push(json!(["volume", offset]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        let mut state = self.0.borrow_mut();
        state.events.push(json!(["play", note, period, counter]));
        let voice = state.voice;
        state.voice += 1;
        voice
    }
}
fn tables() -> DayTables {
    DayTables {
        grids: [SceneGrid {
            columns: 2,
            rows: 2,
            mode: 0,
        }; 14],
        moods: std::array::from_fn(|_| vec![Mood([0, 0]); 4]),
        special_days: [0; 42],
        special_marker: Mood([-4, 6]),
        colors: [0, 1, 2, 3, 4, 5, 6, 7, 0],
        time_limits: [60; 14],
        save_magic: 0,
        start_rotation: [65536; 3],
    }
}
struct Player {
    controls: PlayerControls,
    position: [i32; 3],
    move_distance: i16,
    view: i32,
    map: MapLinkPolicy,
    links: DreamLinks,
    context: LinkContext,
    day: DreamDay,
    notifications: Vec<i32>,
}
impl Default for Player {
    fn default() -> Self {
        let spawn = SpawnLocation {
            chunk: 257,
            tile: 513,
            position: [11, 22, 33],
        };
        Self {
            controls: PlayerControls::default(),
            position: [13, 17, 19],
            move_distance: 300,
            view: 1,
            map: MapLinkPolicy {
                state: 0,
                tick: 0,
                location: 0,
                move_callback: 1,
                reset_view_pending: false,
                view_active: false,
            },
            links: DreamLinks {
                execution: LinkExecution {
                    state: 0,
                    stage: 2,
                    dream_timer: 123,
                    flashback: false,
                },
                spawn,
                source_transform: None,
                target_transform: None,
                auto_action: None,
                auto_step: 0,
                auto_target: SpawnLocation::default(),
                time_limit: 100000,
                default_teleport_transform: false,
            },
            context: LinkContext {
                dynamic_penalty: 17,
                ..Default::default()
            },
            day: DreamDay {
                cinematic: [3, 4],
                ..Default::default()
            },
            notifications: vec![],
        }
    }
}
impl Player {
    fn value(&self) -> Value {
        let spawn = self.links.spawn;
        json!({"action_pressed":self.controls.action_pressed,"view_mode":self.view,"move_mode":self.map.move_callback,
            "view_enabled":self.view!=0,"move_enabled":self.map.move_callback!=0,
            "reset_view_pending":self.map.reset_view_pending,"view_active":self.map.view_active,
            "link_state":self.links.execution.state,"stage":self.links.execution.stage,"timer":self.links.execution.dream_timer,
            "spawn":[spawn.chunk,spawn.tile,spawn.position[0],spawn.position[1],spawn.position[2]],
            "penalty":self.context.dynamic_penalty,"input_locked":self.controls.input_locked,"notifications":self.notifications,"position":self.position,"angles":self.controls.angles,"move_distance":self.move_distance,"cinematic":self.day.cinematic})
    }
    fn clear(&mut self, catalog: &LinkCatalog, with_view: bool) -> Result<(), BehaviorFault> {
        let mut commands = vec![];
        let mut sounds = vec![];
        let mut pending = vec![];
        let mut events = vec![];
        let mut body_contact = lsd_core::BodyContactState { current: None };
        let tables = tables();
        let mut services = ActorJournalServices::new(
            catalog,
            ActorJournalBindings {
                context: &mut self.context,
                map: &mut self.map,
                palette: LinkPalette {
                    dream: 0,
                    flashback: 0,
                },
                commands: &mut commands,
                sounds: &mut sounds,
                current_key: self.links.spawn,
                attached: true,
                pending: &mut pending,
                events: &mut events,
                body_contact: &mut body_contact,
            },
        );
        services.view_callback = Some(&mut self.view);
        let mut dispatcher = ActorJournalDispatcher {
            day: &mut self.day,
            tables: &tables,
            links: &mut self.links,
            controls: &mut self.controls,
            services,
        };
        dispatcher.clear_player_callbacks(with_view)
    }
    fn move_local(
        &mut self,
        catalog: &LinkCatalog,
        math: &PsxMathTables,
        axis: ActorAxis,
        amount: i32,
    ) -> Result<(), BehaviorFault> {
        let mut commands = vec![];
        let mut sounds = vec![];
        let mut pending = vec![];
        let mut events = vec![];
        let mut body_contact = lsd_core::BodyContactState { current: None };
        let tables = tables();
        let mut services = ActorJournalServices::new(
            catalog,
            ActorJournalBindings {
                context: &mut self.context,
                map: &mut self.map,
                palette: LinkPalette {
                    dream: 0,
                    flashback: 0,
                },
                commands: &mut commands,
                sounds: &mut sounds,
                current_key: self.links.spawn,
                attached: true,
                pending: &mut pending,
                events: &mut events,
                body_contact: &mut body_contact,
            },
        );
        services.view_callback = Some(&mut self.view);
        services.player_motion = Some(ActorJournalPlayerMotion {
            position: &mut self.position,
            move_distance: &mut self.move_distance,
            math,
        });
        let mut dispatcher = ActorJournalDispatcher {
            day: &mut self.day,
            tables: &tables,
            links: &mut self.links,
            controls: &mut self.controls,
            services,
        };
        dispatcher.move_player_local(axis, amount)
    }
}
struct LinkReceiver<'a> {
    sound: Sound,
    notifications: &'a mut Vec<i32>,
}
impl DreamLinkEffects for LinkReceiver<'_> {
    fn notify(&mut self, _links: &mut DreamLinks, _controls: &mut PlayerControls, event: i32) {
        // These original constructed graphs have a base Scene parent. Its notification
        // is observed, but it has no MapScene fade or input-lock handler.
        self.notifications.push(event);
    }
    fn sound(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        parameters: [i32; 3],
    ) {
        self.sound
            .play_note(parameters[0], parameters[1], parameters[2]);
    }
    fn rotate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _absolute: bool,
        _packet: [u32; 3],
    ) {
        unreachable!()
    }
    fn translate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _delta: [i32; 3],
    ) {
        unreachable!()
    }
    fn relocate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _spawn: SpawnLocation,
    ) {
        unreachable!()
    }
}
struct Journal<'a> {
    catalog: &'a LinkCatalog,
    context: &'a mut LinkContext,
    receiver: LinkReceiver<'a>,
}
impl ActorJournalHost for Journal<'_> {
    fn base_journal(&mut self, _event: ActorJournalEvent) {}
    fn acknowledge_actor(&mut self, _code: i32) -> Option<i32> {
        unreachable!()
    }
    fn capture_flashback(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        unreachable!()
    }
    fn dynamic_link(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        links.force_dynamic(
            controls,
            self.catalog,
            self.context,
            rng,
            &mut self.receiver,
        )?;
        Ok(())
    }
}
struct Host<'a> {
    math: &'a PsxMathTables,
    aim: &'a PsxAimTables,
    catalog: &'a LinkCatalog,
    target: [i32; 3],
    notifications: Vec<i32>,
    aim_calls: u32,
    player: Player,
    sound: Sound,
    fields: [i8; 12],
}
impl EntityScriptHost for Host<'_> {
    fn move_player_local(&mut self, axis: ActorAxis, amount: i32) -> Result<(), BehaviorFault> {
        self.player
            .move_local(self.catalog, self.math, axis, amount)
    }
    fn player_action_pressed(&mut self) -> Result<bool, BehaviorFault> {
        Ok(self.player.controls.action_pressed)
    }
    fn clear_player_callbacks(&mut self, with_view: bool) -> Result<(), BehaviorFault> {
        self.player.clear(self.catalog, with_view)
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
    fn notify(&mut self, rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault> {
        self.notifications.push(code);
        let mut journal = Journal {
            catalog: self.catalog,
            context: &mut self.player.context,
            receiver: LinkReceiver {
                sound: self.sound.clone(),
                notifications: &mut self.player.notifications,
            },
        };
        actor_journal(
            ActorJournalEvent {
                sender_type: 0x1f234,
                sender_identity: 1,
                code,
                fields: self.fields,
            },
            &mut self.player.day,
            &mut self.player.links,
            &mut self.player.controls,
            rng,
            &mut journal,
        )
        .map_err(BehaviorFault::Journal)
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/action-phase-runtime.json")).unwrap()
}
fn assert_state(actual: Value, expected: &Value, label: &str) {
    if let (Some(actual), Some(expected)) = (actual.as_object(), expected.as_object()) {
        assert_eq!(actual.len(), expected.len(), "{label} field count");
        for (key, value) in actual {
            assert_state(value.clone(), &expected[key], &format!("{label}.{key}"));
        }
    } else {
        assert_eq!(actual, *expected, "{label}");
    }
}
fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn array<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
fn script(name: &str) -> NpcScript {
    match name {
        "entity_ferris_wheel" => NpcScript::FerrisWheel,
        "entity_bell_striker" => NpcScript::BellStriker,
        "entity_elephant" => NpcScript::Elephant,
        "entity_minotaur" => NpcScript::Minotaur,
        "entity_giant_minotaur" => NpcScript::GiantMinotaur,
        "entity_boatman" => NpcScript::Boatman,
        "entity_tea_doll" => NpcScript::TeaDoll,
        "entity_buddha" => NpcScript::Buddha,
        "entity_ox_cart" => NpcScript::OxCart,
        "entity_opera_singer" => NpcScript::OperaSinger,
        "entity_marching_band" => NpcScript::MarchingBand,
        "entity_gears" => NpcScript::Gears,
        _ => panic!("Unknown script"),
    }
}
fn state(
    actor: &ModelActor,
    behavior: &EntityBehavior,
    sound: &Sound,
    host: &Host<'_>,
    rng: &PsxRng,
) -> Value {
    json!({"interaction_timer":actor.animation.ticks,"behavior_tick":behavior.ticks,"state":behavior.state,"triggered":behavior.triggered,
        "link_enabled":behavior.link_enabled,"behavior_active":behavior.active,"animation_enabled":actor.animation.enabled,
        "action_enabled":actor.animation.action_enabled,"clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|part|part.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|part|part.coordinate_translation).collect::<Vec<_>>(),
        "part_angles":actor.animation.parts.iter().map(|part|part.angles).collect::<Vec<_>>(),"context":behavior.context.words,
        "rng":rng.state(),"audio":sound.0.borrow().events,"root_pose":{"position":actor.pose.position,"angles":actor.pose.angles,"scale":actor.pose.scale},
        "move_distance":actor.move_distance,"cache":actor.animation.actor_coordinate_cache,"notifications":host.notifications,"aim_calls":host.aim_calls,"player":host.player.value()})
}
fn compare(
    case: &Value,
    model: &Mom,
    fields: [i8; 12],
    entity: u8,
    math: &PsxMathTables,
    aim: &PsxAimTables,
    catalog: &LinkCatalog,
) {
    let name = case["callback"].as_str().unwrap();
    let mut sound = Sound::default();
    sound.0.borrow_mut().voice = 41;
    let mut actor = model.construct_actor().unwrap();
    let mut behavior =
        EntityBehavior::construct(entity, script(name), fields, &mut actor, &mut sound);
    behavior.enable_link(&mut actor);
    behavior.start_behavior(&mut actor);
    actor.pose.position = [100, 200, 300];
    actor.pose.angles = [-512, 333, 99];
    actor.animation.actor_coordinate_cache = 1;
    sound.0.borrow_mut().events.clear();
    behavior.ticks = case["tick"].as_i64().unwrap() as i32;
    behavior.state = case["initial_state"].as_i64().unwrap_or(0) as i32;
    behavior.triggered = case["initial_trigger"].as_i64().unwrap() as i32;
    behavior.context.words[1] = case["context_tick"].as_i64().unwrap() as i32;
    let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
    let mut host = Host {
        math,
        aim,
        catalog,
        target: [10000, 0, 10000],
        notifications: vec![],
        aim_calls: 0,
        player: Player::default(),
        sound: sound.clone(),
        fields,
    };
    assert_state(
        state(&actor, &behavior, &sound, &host, &rng),
        &case["initial"],
        &format!("{name} initial"),
    );
    for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
        host.target = array(&step["target"]);
        host.player.controls.action_pressed = step["action"].as_bool().unwrap();
        host.notifications.clear();
        host.player.notifications.clear();
        host.aim_calls = 0;
        sound.0.borrow_mut().events.clear();
        behavior
            .tick_with_services(
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
        assert_state(
            state(&actor, &behavior, &sound, &host, &rng),
            &step["state"],
            &format!("{name} tick {} step {index}", case["tick"]),
        );
    }
}

#[test]
fn owned_event_motion_and_sound_actions_match_original_cpu() {
    let value = fixture();
    let model = Mom::parse(&bytes(value["owned_model_hex"].as_str().unwrap())).unwrap();
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            value["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new([0; 512]),
    };
    let aim = PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| {
            value["owned_atan"][i].as_i64().unwrap() as i16
        })),
    };
    let catalog = support::catalog(&value["catalog"]);
    for case in value["cases"].as_array().unwrap() {
        compare(
            case,
            &model,
            array::<12>(&case["fields"]).map(|v| v as i8),
            17,
            &math,
            &aim,
            &catalog,
        );
    }
}
#[test]
fn player_local_motion_adapter_matches_original_scene_node_movements() {
    let value = fixture();
    let catalog = support::catalog(&value["catalog"]);
    let motion: Value = serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/actor-motion.json"
    ))
    .unwrap();
    let mut math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    for entry in motion["owned_trig_entries"].as_array().unwrap() {
        math.sin_cos[entry[0].as_u64().unwrap() as usize] = entry[1].as_u64().unwrap() as u32;
    }
    let mut compared = 0;
    for case in motion["motion_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["input"]["notify"] == false)
    {
        let input = &case["input"];
        let mut player = Player {
            position: array(&input["position"]),
            controls: PlayerControls {
                angles: array::<3>(&input["angles"]).map(|v| v as i16),
                ..Default::default()
            },
            ..Default::default()
        };
        let axis = match input["axis"].as_str().unwrap() {
            "X" => ActorAxis::X,
            "Y" => ActorAxis::Y,
            "Z" => ActorAxis::Z,
            _ => unreachable!(),
        };
        player
            .move_local(
                &catalog,
                &math,
                axis,
                input["amount"].as_i64().unwrap() as i32,
            )
            .unwrap();
        assert_eq!(
            json!(player.position),
            case["expected"]["pose"]["position"],
            "{input}"
        );
        assert_eq!(
            i64::from(player.move_distance),
            case["expected"]["move_distance"].as_i64().unwrap(),
            "{input}"
        );
        assert!(player.notifications.is_empty());
        compared += 1;
    }
    assert_eq!(compared, 144);
}
#[test]
fn ferris_wheel_scene_adapter_preserves_ride_video_and_end_consequences() {
    use lsd_assets::{ActorFrame, EncounterScene, EntityProperties, SceneActor, StageSessionEvent};
    use std::sync::Arc;
    let value = fixture();
    let resource =
        Arc::new(Mom::parse(&bytes(value["owned_model_hex"].as_str().unwrap())).unwrap());
    let math = PsxMathTables {
        sin_cos: Box::new(std::array::from_fn(|i| {
            value["owned_trig"][i].as_u64().unwrap() as u32
        })),
        sqrt_window: Box::new([0; 512]),
    };
    let aim = PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| {
            value["owned_atan"][i].as_i64().unwrap() as i16
        })),
    };
    let catalog = support::catalog(&value["catalog"]);
    let tables = tables();
    let mut compared = 0;
    for case in value["cases"].as_array().unwrap().iter().filter(|case| {
        case["callback"] == "entity_ferris_wheel" && case["steps"].as_array().unwrap().len() == 140
    }) {
        let fields = array::<12>(&case["fields"]).map(|v| v as i8);
        let mut sound = Sound::default();
        sound.0.borrow_mut().voice = 41;
        let mut actor = resource.construct_actor().unwrap();
        let script = NpcScript::from_reference_address(Some(0x8006_090c));
        assert_eq!(script, NpcScript::FerrisWheel);
        let mut behavior = EntityBehavior::construct(17, script, fields, &mut actor, &mut sound);
        behavior.enable_link(&mut actor);
        behavior.start_behavior(&mut actor);
        behavior.triggered = 1;
        actor.pose.position = [100, 200, 300];
        actor.pose.angles = [-512, 333, 99];
        actor.animation.actor_coordinate_cache = 1;
        sound.0.borrow_mut().events.clear();
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
                callback: Some(0x8006_090c),
            },
            behavior_active: true,
            behavior_pending: false,
            behavior,
        });
        let mut host = Host {
            math: &math,
            aim: &aim,
            catalog: &catalog,
            target: [10000, 0, 10000],
            notifications: vec![],
            aim_calls: 0,
            player: Player::default(),
            sound: sound.clone(),
            fields,
        };
        let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        let mut body_contact = lsd_core::BodyContactState { current: None };
        for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            host.target = array(&step["target"]);
            host.player.controls.action_pressed = step["action"].as_bool().unwrap();
            let mut commands = vec![];
            let mut link_sounds = vec![];
            let mut pending = vec![];
            let mut events = vec![];
            let mut services = ActorJournalServices::new(
                &catalog,
                ActorJournalBindings {
                    context: &mut host.player.context,
                    map: &mut host.player.map,
                    palette: LinkPalette {
                        dream: 0,
                        flashback: 0,
                    },
                    commands: &mut commands,
                    sounds: &mut link_sounds,
                    current_key: host.player.links.spawn,
                    attached: true,
                    pending: &mut pending,
                    events: &mut events,
                    body_contact: &mut body_contact,
                },
            );
            services.view_callback = Some(&mut host.player.view);
            services.player_motion = Some(ActorJournalPlayerMotion {
                position: &mut host.player.position,
                move_distance: &mut host.player.move_distance,
                math: &math,
            });
            let mut dispatcher = ActorJournalDispatcher {
                day: &mut host.player.day,
                tables: &tables,
                links: &mut host.player.links,
                controls: &mut host.player.controls,
                services,
            };
            scene
                .tick_actor_frame(
                    ActorFrame {
                        serial,
                        target: lsd_core::EntityTarget {
                            position: host.target,
                            local_y: 17,
                            matrix: FixedMatrix::IDENTITY,
                            eye: [0; 3],
                            target: [0, 0, 1024],
                        },
                        signal: lsd_core::FrameSignal::Tick,
                    },
                    &mut rng,
                    &mut sound,
                    &math,
                    &mut dispatcher,
                )
                .unwrap();
            host.notifications = events
                .iter()
                .filter_map(|event| match event {
                    StageSessionEvent::ActorJournal { code, .. } => Some(*code),
                    _ => None,
                })
                .collect();
            assert!(pending.is_empty());
            assert!(link_sounds.is_empty());
            let actor = &scene.actors[0];
            assert_state(
                state(&actor.actor, &actor.behavior, &sound, &host, &rng),
                &step["state"],
                &format!("Ferris Wheel scene {index}"),
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 280);
}
#[test]
#[ignore = "Requires original game files and privately recorded event, motion and sound NPC comparisons"]
fn canonical_event_motion_and_sound_actions_match_original_cpu() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let cases: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ACTION_PHASES").expect("Set LSD_NATIVE_ACTION_PHASES"),
        )
        .unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let math = lsd_assets::load_math_tables(&exe).unwrap();
    let aim = lsd_assets::load_special_encounter_assets(&exe, &files)
        .unwrap()
        .aim;
    let catalog = lsd_assets::load_link_tables(&exe).unwrap().catalog;
    for case in cases["cases"].as_array().unwrap() {
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
        compare(
            case,
            &Mom::parse(&source[at..end]).unwrap(),
            array::<12>(&binding["fields"]).map(|v| v as i8),
            binding["entity"].as_u64().unwrap() as u8,
            &math,
            &aim,
            &catalog,
        );
    }
}
