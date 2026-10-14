use lsd_assets::{
    EncounterScene, EntityProperties, SceneActor, SpecialEncounterAssets, formats::Mom,
};
use lsd_core::{EntityBehavior, GenerationSound, NpcScript, PsxAimTables};
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
    fn volume_offset(&mut self, offset: i32) {
        self.events.push(json!(["volume", offset]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        self.events.push(json!(["play", note, period, counter]));
        -1
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/special-encounter-activation.json")).unwrap()
}
fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn scene(fields: [i8; 12], assets: SpecialEncounterAssets) -> EncounterScene {
    let tables = lsd_core::EncounterTables {
        chunks: std::array::from_fn(|_| Vec::new()),
        objects: std::array::from_fn(|_| Vec::new()),
        variants: Vec::new(),
        positions: Vec::new(),
        rotations: Vec::new(),
        color_lookup: Vec::new(),
    };
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
    let mut out = EncounterScene::new(tables, properties);
    out.enable_special_encounter(assets);
    out
}
fn snapshot(scene: &EncounterScene, sound: &Sound) -> Value {
    let a = scene.actors.iter().find(|v| v.entity == 98).unwrap();
    let actor = &a.actor;
    let behavior = &a.behavior;
    let mut order = scene
        .frame_actor_order()
        .iter()
        .map(|s| if *s == a.serial { "gray" } else { "marker" })
        .collect::<Vec<_>>();
    order.push("player");
    json!({"interaction_timer":actor.animation.ticks,"behavior_tick":behavior.ticks,"state":behavior.state,
        "triggered":behavior.triggered,"link_enabled":behavior.link_enabled,"behavior_active":behavior.active,
        "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
        "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|p|p.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|p|p.coordinate_translation).collect::<Vec<_>>(),
        "context":behavior.context.words,"rng":1,"audio":sound.events,
        "position":actor.pose.position,"angles":actor.pose.angles,"subscription_order":order})
}
fn activation_comparison(fixture: &Value, case: &Value, assets: SpecialEncounterAssets) {
    let fields = std::array::from_fn(|i| fixture["fields"][i].as_i64().unwrap() as i8);
    let marker_resource =
        Arc::new(Mom::parse(&bytes(fixture["owned_model_hex"].as_str().unwrap())).unwrap());
    let mut scene = scene(fields, assets);
    let mut sound = Sound::default();
    scene.ensure_special_actor(&mut sound).unwrap();
    assert_eq!(
        snapshot(&scene, &sound),
        case["initial"],
        "dormant constructor"
    );
    assert!(scene.frame_actor_order().is_empty());
    assert!(
        scene.actors[0].behavior_pending,
        "Activation does not accept the pending Gray Man action"
    );
    let mut actor = marker_resource.construct_actor().unwrap();
    let behavior = EntityBehavior::construct(17, NpcScript::None, [0; 12], &mut actor, &mut sound);
    scene.add_actor(SceneActor {
        serial: 0,
        slot: 0,
        map: 0,
        entity: 17,
        model: 0,
        resource: marker_resource,
        actor,
        collision: None,
        properties: EntityProperties {
            fields: [0; 12],
            callback: None,
        },
        behavior_active: true,
        behavior_pending: false,
        behavior,
    });
    scene.take_frame_changes();
    let gray_serial = scene.actors[0].serial;
    for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
        let input = &step["input"];
        let position = std::array::from_fn(|i| input["position"][i].as_i64().unwrap() as i32);
        let matrix = lsd_core::FixedMatrix(std::array::from_fn(|row| {
            std::array::from_fn(|col| input["matrix"][row][col].as_i64().unwrap() as i16)
        }));
        scene.set_encounter_player(position, matrix);
        sound.events.clear();
        scene.activate_gray_man(&mut sound).unwrap();
        assert_eq!(
            snapshot(&scene, &sound),
            step["state"],
            "activation {index}"
        );
        assert_eq!(scene.actors.iter().filter(|v| v.entity == 98).count(), 1);
        assert_eq!(scene.actors[0].serial, gray_serial);
        let changes = scene.take_frame_changes();
        assert_eq!(
            changes.last(),
            Some(&lsd_assets::EncounterSubscription::Attach(gray_serial))
        );
        assert_eq!(changes.len(), if index == 0 { 1 } else { 2 });
    }
    scene.stop_with_audio(&mut sound);
    scene.ensure_special_actor(&mut sound).unwrap();
    assert_ne!(scene.actors[0].serial, gray_serial);
    assert!(scene.frame_actor_order().is_empty());
}
#[test]
fn owned_lookup_quadrants_shift_boundaries_and_special_reactivation_match_original_cpu() {
    let fixture = fixture();
    let aim = PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| (i / 2) as i16)),
    };
    for case in fixture["atan_cases"].as_array().unwrap() {
        let y = case["input"][0].as_i64().unwrap() as i32;
        let x = case["input"][1].as_i64().unwrap() as i32;
        assert_eq!(
            aim.atan2(y, x).unwrap(),
            case["result"].as_i64().unwrap() as i32,
            "atan2({y},{x})"
        );
    }
    let assets = SpecialEncounterAssets {
        model: Arc::new(Mom::parse(&bytes(fixture["owned_model_hex"].as_str().unwrap())).unwrap()),
        path: PathBuf::from("owned.MOM"),
        offset: std::array::from_fn(|i| fixture["offset"][i].as_i64().unwrap() as i32),
        aim,
    };
    activation_comparison(&fixture, &fixture["cases"][0], assets);
    println!(
        "owned_aim_cases=425 owned_special_activations=6 dormant_clock_subscription=false stable_instance_on_reactivation=true gray_action_accepted=false"
    );
}
#[test]
#[ignore = "Requires canonical executable and the original special encounter model"]
fn canonical_special_model_placement_facing_and_reactivation_match_original_cpu() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let assets = lsd_assets::load_special_encounter_assets(&executable, &files).unwrap();
    let fixture = fixture();
    assert_eq!(
        lsd_assets::sha256_hex(&std::fs::read(files.join(&assets.path)).unwrap()),
        fixture["canonical_model_sha256"].as_str().unwrap()
    );
    let math = lsd_assets::load_math_tables(&executable).unwrap();
    for step in fixture["cases"][1]["steps"].as_array().unwrap() {
        let angles = std::array::from_fn(|i| step["input"]["angles"][i].as_i64().unwrap() as i16);
        let matrix = lsd_core::FixedMatrix(std::array::from_fn(|row| {
            std::array::from_fn(|col| step["input"]["matrix"][row][col].as_i64().unwrap() as i16)
        }));
        assert_eq!(math.rotation(angles), matrix);
    }
    activation_comparison(&fixture, &fixture["cases"][1], assets);
    println!(
        "canonical_special_activations=6 original_special_model_and_aim_table=true whole_natural_gray_encounter_accepted=false"
    );
}

#[test]
#[ignore = "Requires canonical original encounter tables, streamed maps and special model"]
fn loaded_original_day_gate_activates_one_special_actor_and_slot_release_preserves_it() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let tables = lsd_assets::load_encounter_tables(&executable).unwrap();
    let grid = lsd_assets::load_scene_grid(&executable, 2).unwrap();
    let gated = tables.chunks[2]
        .iter()
        .find(|chunk| chunk.day_gate == 1)
        .expect("Canonical stage two has an even-day special gate");
    let [column, row] = gated.chunk.to_le_bytes().map(u16::from);
    let mut stage = lsd_assets::StreamedStage::load(
        &files,
        &executable,
        2,
        row * grid.columns as u16 + column,
        [0; 3],
    )
    .unwrap();
    let properties = lsd_assets::load_entity_properties(&executable).unwrap();
    let assets = lsd_assets::load_special_encounter_assets(&executable, &files).unwrap();
    let math = lsd_assets::load_math_tables(&executable).unwrap();
    let position = [6000, -1024, 14000];
    let matrix = math.rotation([0, 321, 0]);
    let expected_position =
        std::array::from_fn(|i| matrix.apply_long(assets.offset)[i] + position[i]);
    let expected_angles = assets
        .aim
        .object_angles(expected_position, position, true, false)
        .unwrap();
    let mut sound = Sound::default();
    let mut matched = None;
    for seed in 1..10000 {
        let mut scene = EncounterScene::new(tables.clone(), properties.clone());
        scene.enable_special_encounter(assets.clone());
        scene.set_encounter_player(position, matrix);
        let mut rng = lsd_core::PsxRng::new(seed);
        scene
            .update_with_audio(
                &stage,
                lsd_assets::EncounterUpdate {
                    epoch: 0,
                    day: 2,
                    palette: 0,
                    generation_type: 0,
                },
                &mut rng,
                &mut sound,
            )
            .unwrap();
        if scene.gray_man_activation_requests > 0 {
            matched = Some((scene, rng));
            break;
        }
    }
    let (mut scene, mut rng) = matched.expect("A supplied seed reaches the original 1/12 gate");
    assert_eq!(scene.teleports_initialization, Some(false));
    let special = scene.actors.iter().find(|v| v.entity == 98).unwrap();
    let serial = special.serial;
    assert_eq!(special.actor.pose.position, expected_position);
    assert_eq!(special.actor.pose.angles, expected_angles);
    assert!(scene.frame_actor_order().contains(&serial));
    assert_eq!(scene.actors.iter().filter(|v| v.entity == 98).count(), 1);
    let activations = scene.gray_man_activation_requests;
    let state = rng.state();
    let input = lsd_assets::EncounterUpdate {
        epoch: 0,
        day: 2,
        palette: 0,
        generation_type: 0,
    };
    scene
        .update_with_audio(&stage, input, &mut rng, &mut sound)
        .unwrap();
    assert_eq!(rng.state(), state);
    assert_eq!(scene.gray_man_activation_requests, activations);
    for slot in &mut stage.collision.slots.slots {
        slot.ready = false;
    }
    scene
        .update_with_audio(&stage, input, &mut rng, &mut sound)
        .unwrap();
    assert_eq!(scene.actors.len(), 1);
    assert_eq!(scene.actors[0].serial, serial);
    assert!(scene.frame_actor_order().contains(&serial));
    scene
        .update_with_audio(
            &stage,
            lsd_assets::EncounterUpdate { epoch: 1, ..input },
            &mut rng,
            &mut sound,
        )
        .unwrap();
    assert_ne!(scene.actors[0].serial, serial);
    assert!(scene.frame_actor_order().is_empty());
    println!(
        "loaded_canonical_special_gate_connected=true ordinary_slot_release_preserves_special=true rebuild_recreates_dormant_instance=true gray_action_accepted=false"
    );
}
