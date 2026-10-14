use lsd_assets::formats::Mom;
use lsd_core::{
    ActorAxis, BehaviorFault, BehaviorTick, EntityBehavior, EntityScriptHost, FixedMatrix,
    GenerationSound, ModelActor, NpcScript, PsxMathTables, PsxRng,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/actor-motion.json"
    ))
    .unwrap()
}
fn model(fixture: &Value) -> Mom {
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    Mom::parse(&bytes).expect("Owned two-clip MOM must decode before any native comparison")
}
fn array<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
fn tables(fixture: &Value) -> PsxMathTables {
    let mut math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    for entry in fixture["owned_trig_entries"].as_array().unwrap() {
        math.sin_cos[entry[0].as_u64().unwrap() as usize] = entry[1].as_u64().unwrap() as u32;
    }
    math
}
fn make_actor(model: &Mom, input: &Value) -> ModelActor {
    let mut actor = ModelActor::construct(
        &model.animations,
        &model.initial_part_types,
        model.tmd.objects.len(),
    )
    .unwrap();
    actor.pose.position = array(&input["position"]);
    actor.pose.angles = array::<3>(&input["angles"]).map(|v| v as i16);
    if !input["scale"].is_null() {
        actor.pose.scale = array(&input["scale"]);
    }
    actor.animation.actor_coordinate_cache = 1;
    actor
}
fn pose(actor: &ModelActor) -> Value {
    json!({"position":actor.pose.position,"angles":actor.pose.angles,"scale":actor.pose.scale})
}
fn motion_state(actor: &ModelActor, notifications: &[i32]) -> Value {
    json!({"pose":pose(actor),"move_distance":actor.move_distance,"cache":actor.animation.actor_coordinate_cache,"notifications":notifications})
}
#[test]
fn original_actor_axis_movement_halfword_wrapping_and_notifications_match() {
    let fixture = fixture();
    let model = model(&fixture);
    let math = tables(&fixture);
    for (number, case) in fixture["motion_cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let input = &case["input"];
        let mut actor = make_actor(&model, input);
        let axis = match input["axis"].as_str().unwrap() {
            "X" => ActorAxis::X,
            "Y" => ActorAxis::Y,
            "Z" => ActorAxis::Z,
            _ => unreachable!(),
        };
        actor.move_local(
            axis,
            input["amount"].as_i64().unwrap() as i32,
            math.rotation(actor.pose.angles),
        );
        let notifications = if input["notify"].as_bool().unwrap() {
            vec![axis.notification()]
        } else {
            vec![]
        };
        assert_eq!(
            motion_state(&actor, &notifications),
            case["expected"],
            "movement {number}"
        );
    }
    assert_eq!(fixture["motion_cases"].as_array().unwrap().len(), 288);
}
#[test]
fn original_actor_rotation_and_scale_packet_setters_match() {
    let fixture = fixture();
    let model = model(&fixture);
    for (number, case) in fixture["setter_cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let input = &case["input"];
        let mut actor = make_actor(&model, input);
        let packet = array::<3>(&input["packet"]).map(|v| v as u32);
        let absolute = input["absolute"].as_bool().unwrap();
        match input["operation"].as_str().unwrap() {
            "rotate" => actor.rotate_packet(packet, absolute).unwrap(),
            "scale" => actor.scale_packet(packet, absolute).unwrap(),
            _ => unreachable!(),
        }
        assert_eq!(
            motion_state(&actor, &[]),
            case["expected"],
            "packet setter {number}"
        );
    }
    assert_eq!(fixture["setter_cases"].as_array().unwrap().len(), 144);
}
#[test]
fn original_invalid_packets_fault_before_mutating_the_actor_transform() {
    let fixture = fixture();
    let model = model(&fixture);
    for (number, case) in fixture["invalid_setter_cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let input = &case["input"];
        let mut actor = make_actor(&model, input);
        let packet = array::<3>(&input["packet"]).map(|v| v as u32);
        let absolute = input["absolute"].as_bool().unwrap();
        let result = match input["operation"].as_str().unwrap() {
            "rotate" => actor.rotate_packet(packet, absolute),
            "scale" => actor.scale_packet(packet, absolute),
            _ => unreachable!(),
        };
        assert_eq!(result, Err(lsd_core::GenerationFault::InvalidDivision));
        assert_eq!(case["native_cpu_exception"], true);
        assert_eq!(
            motion_state(&actor, &[]),
            case["expected"],
            "invalid packet {number}"
        );
    }
    assert_eq!(
        fixture["invalid_setter_cases"].as_array().unwrap().len(),
        12
    );
}
struct Host<'a> {
    math: &'a PsxMathTables,
    notifications: Vec<i32>,
}
impl EntityScriptHost for Host<'_> {
    fn movement_matrix(&mut self, angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault> {
        Ok(self.math.rotation(angles))
    }
    fn notify(&mut self, _rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault> {
        self.notifications.push(code);
        Ok(())
    }
}
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
fn state(
    actor: &ModelActor,
    behavior: &EntityBehavior,
    rng: &PsxRng,
    sound: &Sound,
    host: &Host<'_>,
) -> Value {
    json!({"interaction_timer":actor.animation.ticks,"behavior_tick":behavior.ticks,
        "state":behavior.state,"triggered":behavior.triggered,"link_enabled":behavior.link_enabled,"behavior_active":behavior.active,
        "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
        "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|part|part.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|part|part.coordinate_translation).collect::<Vec<_>>(),
        "context":behavior.context.words,"rng":rng.state(),"audio":sound.events,"root_pose":pose(actor),
        "move_distance":actor.move_distance,"cache":actor.animation.actor_coordinate_cache,"notifications":host.notifications})
}
#[test]
fn nine_original_callbacks_preserve_action_animation_motion_rng_sound_and_request_order() {
    let fixture = fixture();
    let model = model(&fixture);
    let math = tables(&fixture);
    assert_eq!(
        model
            .animations
            .iter()
            .map(|clip| clip.frames.len())
            .collect::<Vec<_>>(),
        vec![32, 4]
    );
    for (number, case) in fixture["callback_cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        compare_callback(number, case, &model, &math);
    }
    assert_eq!(fixture["callback_cases"].as_array().unwrap().len(), 54);
}
fn script(name: &str) -> NpcScript {
    match name {
        "entity_yokai" => NpcScript::Yokai,
        "entity_sailboat" => NpcScript::Sailboat,
        "entity_balloon" => NpcScript::Balloon,
        "entity_fish_giant" => NpcScript::FishGiant,
        "entity_halo" => NpcScript::Halo,
        "entity_siblings_flock" => NpcScript::SiblingsFlock,
        "entity_bed_bone_dead" => NpcScript::BedBoneDead,
        "entity_kicking_man" => NpcScript::KickingMan,
        "entity_prince_princess" => NpcScript::PrincePrincess,
        _ => unreachable!(),
    }
}
fn compare_callback(number: usize, case: &Value, model: &Mom, math: &PsxMathTables) {
    let input = &case["input"];
    let script = script(input["callback"].as_str().unwrap());
    let mut actor = ModelActor::construct(
        &model.animations,
        &model.initial_part_types,
        model.tmd.objects.len(),
    )
    .unwrap();
    let mut sound = Sound {
        events: vec![],
        voice: 41,
    };
    let mut behavior = EntityBehavior::construct(
        input["entity"].as_u64().unwrap() as u8,
        script,
        array::<12>(&input["fields"]).map(|v| v as i8),
        &mut actor,
        &mut sound,
    );
    let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
    let mut host = Host {
        math,
        notifications: vec![],
    };
    sound.events.clear();
    if !case["constructed"].is_null() {
        // This fixture has already run entity_start with a supplied zero origin.
        actor.translate([0; 3], true);
        assert_eq!(
            state(&actor, &behavior, &rng, &sound, &host),
            case["constructed"],
            "constructed callback {number}"
        );
    }
    if input["force_behavior"].as_bool().unwrap_or(false) {
        behavior.start_behavior(&mut actor);
    }
    actor
        .animation
        .select_clip(
            &model.animations,
            input["variant"].as_u64().unwrap() as usize,
        )
        .unwrap();
    for _ in 0..input["warm_frames"].as_u64().unwrap() {
        actor.animation.advance_frame(&model.animations).unwrap();
    }
    actor.animation.ticks = 0;
    behavior.ticks = input["tick"].as_i64().unwrap() as i32;
    behavior.context.words[1] = input["context_tick"].as_i64().unwrap() as i32;
    actor.pose.position = array(&input["position"]);
    actor.pose.angles = array::<3>(&input["angles"]).map(|v| v as i16);
    actor.animation.actor_coordinate_cache = 1;
    sound.events.clear();
    assert_eq!(
        state(&actor, &behavior, &rng, &sound, &host),
        case["initial"],
        "initial callback {number}"
    );
    for (step, tick) in case["steps"].as_array().unwrap().iter().enumerate() {
        sound.events.clear();
        host.notifications.clear();
        behavior
            .tick_with_services(
                &mut actor,
                &model.animations,
                BehaviorTick {
                    target: array(&tick["target"]),
                    proximity: input["fields"][10].as_i64().unwrap() as i8,
                },
                &mut rng,
                &mut sound,
                &mut host,
            )
            .unwrap();
        assert_eq!(
            state(&actor, &behavior, &rng, &sound, &host),
            tick["expected"],
            "callback {number} step {step}"
        );
    }
}
#[test]
#[ignore = "Requires canonical game files and private native callback observations"]
fn canonical_original_models_properties_and_eight_callback_types_match() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ACTOR_MOTION").expect("Set LSD_NATIVE_ACTOR_MOTION"),
        )
        .unwrap(),
    )
    .unwrap();
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let math = lsd_assets::load_math_tables(&executable).unwrap();
    let properties = lsd_assets::load_entity_properties(&executable).unwrap();
    let declarations = lsd_assets::load_encounter_tables(&executable).unwrap();
    let mut callbacks = std::collections::BTreeSet::new();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
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
        let stage = input["stage"].as_u64().unwrap() as usize;
        let entity = input["entity"].as_u64().unwrap() as usize;
        let object_index = input["object_index"].as_u64().unwrap() as usize;
        let chunk = &declarations.chunks[stage][input["chunk_number"].as_u64().unwrap() as usize];
        assert!(chunk.objects.contains(&(object_index as i8)));
        let object = &declarations.objects[stage][object_index];
        assert_eq!(usize::from(object.entity), entity);
        assert_eq!(object.model as u64, input["model_index"].as_u64().unwrap());
        let grid = lsd_assets::load_scene_grid(&executable, stage as u8).unwrap();
        let [x, y] = chunk.chunk.to_le_bytes().map(|v| i32::from(v as i8));
        assert_eq!(
            lsd_assets::stage_map_path(
                &executable,
                stage as u8,
                (x + y * i32::from(grid.columns)) as u16
            )
            .unwrap(),
            path
        );
        assert_eq!(
            array::<12>(&input["fields"]).map(|v| v as i8),
            properties[entity].fields
        );
        assert_eq!(
            NpcScript::from_reference_address(properties[entity].callback),
            script(input["callback"].as_str().unwrap())
        );
        callbacks.insert(input["callback"].as_str().unwrap());
        compare_callback(number, case, &model, &math);
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 9);
    assert_eq!(callbacks.len(), 8);
    println!(
        "canonical_original_callback_types=8 model_property_cases=9 callback_state_observations=378 supplied_activation=true whole_npc_encounter_acceptance=false"
    );
}
