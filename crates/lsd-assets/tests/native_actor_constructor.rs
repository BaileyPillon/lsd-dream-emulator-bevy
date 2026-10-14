use lsd_assets::{formats::Mom, sha256_hex};
use lsd_core::{AnimationParent, ModelActor};
use serde_json::{Value, json};
fn snapshot(actor: &ModelActor) -> Value {
    let player = &actor.animation;
    json!({"root_part":actor.root_part,"pose":{"position":actor.pose.position,"angles":actor.pose.angles,"scale":actor.pose.scale},
        // Historical fixture key; native motion verifies this is the movement halfword.
        "attributes":actor.attributes,"collision_mode":actor.collision_mode,"turn_limit":actor.move_distance,
        "animation":{"ticks":player.ticks,"enabled":player.enabled,"action_enabled":player.action_enabled,
        "clip":player.selected_clip,"frame":player.frame_index,"actor_cache":player.actor_coordinate_cache,"actor_children":player.actor_children,
        "parts":player.parts.iter().map(|part|json!({"part_type":part.part_type,"attributes":part.attributes,"cache":part.coordinate_cache,
        "angles":part.angles,"scale":part.scale,"position":part.translation,"coordinate_position":part.coordinate_translation,
        "primitive":part.primitive,"parent":part.parent.map(|p|match p {AnimationParent::Actor=>-1,AnimationParent::Part(v)=>v as i32}),
        "children":part.children})).collect::<Vec<_>>()}})
}
#[test]
#[ignore = "Requires original MOM inputs and private actor-constructor CPU observations"]
fn original_actor_constructor_sdk_bindings_root_and_animation_match() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ACTOR_CONSTRUCTORS")
                .expect("Set LSD_NATIVE_ACTOR_CONSTRUCTORS"),
        )
        .unwrap(),
    )
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let relative = std::path::Path::new(input["path"].as_str().unwrap());
        assert!(
            relative
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_)))
        );
        let data = std::fs::read(files.join(relative)).unwrap();
        assert_eq!(sha256_hex(&data), input["source_sha256"].as_str().unwrap());
        let start = input["offset"].as_u64().unwrap() as usize;
        let bytes = input["bytes"].as_u64().unwrap() as usize;
        let data = &data[start..start + bytes];
        assert_eq!(sha256_hex(data), input["mom_sha256"].as_str().unwrap());
        let model = Mom::parse(data).unwrap();
        let mut actor = model.construct_actor().unwrap();
        assert_eq!(
            snapshot(&actor),
            case["expected"],
            "constructor {}",
            input["path"]
        );
        for step in case["steps"].as_array().unwrap() {
            actor.animation.enabled = step["enabled"].as_bool().unwrap();
            actor.tick_animation(&model.animations).unwrap();
            assert_eq!(snapshot(&actor), step["expected"], "tick {}", input["path"]);
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 100);
}
