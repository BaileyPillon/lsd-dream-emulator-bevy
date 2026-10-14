#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use lsd_assets::{
    ActorJournalBindings, ActorJournalDispatcher, ActorJournalServices, EncounterEventHost,
    formats::Mom,
};
use lsd_core::{
    ActorJournalEvent, ActorJournalHost, BehaviorFault, BehaviorTick, DayFault, DayTables,
    DreamDay, DreamLinkEffects, DreamLinks, EntityBehavior, EntityScriptHost, FixedMatrix,
    GenerationSound, LinkCatalog, LinkContext, LinkExecution, LinkPalette, MapLinkPolicy,
    ModelActor, Mood, NpcScript, PlayerControls, PsxAimTables, PsxMathTables, PsxRng, SceneGrid,
    SpawnLocation, actor_journal,
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
            day: DreamDay::default(),
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
            "penalty":self.context.dynamic_penalty,"input_locked":self.controls.input_locked,"notifications":self.notifications})
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
    serde_json::from_str(include_str!("fixtures/player-npc-runtime.json")).unwrap()
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
        "entity_dog" => NpcScript::Dog,
        "entity_gargoyle" => NpcScript::Gargoyle,
        "entity_demon_king" => NpcScript::DemonKing,
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
fn owned_player_interacting_npc_actions_match_original_cpu() {
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
fn player_callback_clear_adapter_matches_original_cpu() {
    let value = fixture();
    let catalog = support::catalog(&value["catalog"]);
    for case in value["clear_cases"].as_array().unwrap() {
        let mut player = Player {
            view: case["input"]["view"].as_i64().unwrap() as i32,
            ..Default::default()
        };
        player.map.move_callback = case["input"]["move"].as_i64().unwrap() as i32;
        // Installing original move callback two also activates reset/view flags.
        if player.map.move_callback == 2 {
            player.map.reset_view_pending = true;
            player.map.view_active = true;
        }
        player.controls.action_pressed = true;
        assert_eq!(player.value(), case["initial"], "initial {}", case["input"]);
        player
            .clear(&catalog, case["input"]["with_view"].as_bool().unwrap())
            .unwrap();
        assert_eq!(player.value(), case["state"], "clear {}", case["input"]);
    }
}
fn controls(value: &Value) -> PlayerControls {
    let number = |name: &str| value[name].as_i64().unwrap() as i32;
    let flag = |name: &str| value[name].as_bool().unwrap();
    PlayerControls {
        paused: number("paused"),
        input_locked: flag("input_locked"),
        auto_move_locked: flag("auto_move_locked"),
        auto_move_active: flag("auto_move_active"),
        action_pressed: flag("action_pressed"),
        look_state: number("look_state"),
        look_vertical: number("look_vertical"),
        turn_state: number("turn_state"),
        turn_offset: number("turn_offset"),
        left_right: number("left_right"),
        is_forward: flag("is_forward"),
        move_state: number("move_state"),
        move_direction: number("move_direction"),
        previous_move_direction: number("previous_move_direction"),
        move_step: number("move_step"),
        angles: array::<3>(&value["angles"]).map(|v| v as i16),
        eye: array(&value["eye"]),
        target: array(&value["target"]),
        screen_shake: flag("screen_shake"),
    }
}
#[test]
fn cleared_callbacks_preserve_original_player_clock_and_view() {
    use lsd_core::{
        AutoMoveTables, ChunkPlayer, CollisionBounds, FrameSignal, LinkMotionRules, LinkedPlayer,
        PadState, SceneCollision, SceneLinkTables, SceneSlot, SceneSlots,
    };
    let value = fixture();
    let scene = SceneCollision {
        slots: SceneSlots {
            grid: SceneGrid {
                columns: 1,
                rows: 1,
                mode: 0,
            },
            slots: std::array::from_fn(|i| SceneSlot {
                label: i as i16,
                map_index: 0,
                origin: [0; 3],
                ready: i == 0,
            }),
        },
        chunks: std::array::from_fn(|_| None),
    };
    let math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    for case in value["clock_cases"].as_array().unwrap() {
        let mut player = ChunkPlayer {
            controls: controls(&case["initial"]["state"]),
            pad: PadState::default(),
            position: [0; 3],
            bounds: CollisionBounds {
                minimum: [0; 3],
                maximum: [0; 3],
            },
        };
        let mut linked = LinkedPlayer::new(
            0,
            &player,
            &scene,
            SceneLinkTables {
                catalog: support::catalog(&value["catalog"]),
                motion: LinkMotionRules {
                    distances: [0; 5],
                    signs: [0; 5],
                },
                auto_move: AutoMoveTables {
                    targets: [[0; 3]; 4],
                    clockwise: [0; 3],
                    counterclockwise: [0; 3],
                },
            },
            1,
        )
        .unwrap();
        linked.map.move_callback = 0;
        linked.view_callback = case["initial_callbacks"]["view_mode"].as_i64().unwrap() as i32;
        linked.links.execution.dream_timer = case["initial"]["timer"].as_u64().unwrap() as u32;
        linked.can_dynamic = case["initial"]["can_dynamic"].as_u64().unwrap() != 0;
        linked
            .tick_clock(&mut player, &scene, &math, [1; 7], None, FrameSignal::Tick)
            .unwrap();
        assert_eq!(
            player.controls,
            controls(&case["expected"]["state"]),
            "{case}"
        );
        assert_eq!(
            u64::from(linked.links.execution.dream_timer),
            case["expected"]["timer"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(linked.can_dynamic),
            case["expected"]["can_dynamic"].as_u64().unwrap()
        );
    }
}
#[test]
#[ignore = "Requires original game files and privately recorded player-interacting NPC comparisons"]
fn canonical_player_interacting_npc_actions_match_original_cpu() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let cases: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_PLAYER_NPCS").expect("Set LSD_NATIVE_PLAYER_NPCS"),
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
