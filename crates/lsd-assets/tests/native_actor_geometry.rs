use lsd_assets::{formats::Mom, sha256_hex};
use serde_json::{Value, json};
fn snapshot(actor: &lsd_core::ModelActor, geometry: &Option<lsd_core::CollisionModel>) -> Value {
    json!({"collision_primitive":actor.collision_primitive,"root_primitive":actor.animation.parts[actor.root_part].primitive,
        "bounds":geometry.as_ref().and_then(|g|g.bounds).map(|b|[b.minimum[0],b.minimum[1],b.minimum[2],b.maximum[0],b.maximum[1],b.maximum[2]])})
}
#[test]
#[ignore = "Requires canonical MOM corpus and private actor geometry observations"]
fn canonical_copied_root_geometry_bounds_and_animation_binding_match() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_ACTOR_GEOMETRY").expect("Set LSD_NATIVE_ACTOR_GEOMETRY"),
        )
        .unwrap(),
    )
    .unwrap();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let relative = std::path::Path::new(input["path"].as_str().unwrap());
        assert!(
            relative
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_)))
        );
        let data = std::fs::read(files.join(relative)).unwrap();
        assert_eq!(sha256_hex(&data), input["source_sha256"].as_str().unwrap());
        let offset = input["offset"].as_u64().unwrap() as usize;
        let data = &data[offset..offset + input["bytes"].as_u64().unwrap() as usize];
        assert_eq!(sha256_hex(data), input["mom_sha256"].as_str().unwrap());
        let model = Mom::parse(data).unwrap();
        let mut actor = model.construct_actor().unwrap();
        let geometry = model.collision_model(&actor).unwrap();
        assert_eq!(
            snapshot(&actor, &geometry),
            case["initial"],
            "constructor {number}"
        );
        actor.animation.enabled = true;
        for (step, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
            actor.tick_animation(&model.animations).unwrap();
            assert_eq!(
                snapshot(&actor, &geometry),
                *expected,
                "binding {number} step {step}"
            );
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 100);
}
