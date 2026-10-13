use lsd_assets::{formats::Mom, is_canonical_reference, sha256_hex};

#[path = "../../lsd-core/tests/support/animation_observations.rs"]
mod animation_observations;

#[test]
#[ignore = "Requires local original inputs and private native animation observations"]
fn decoded_original_animations_match_native_runtime_states() {
    let root = std::path::PathBuf::from(
        std::env::var("LSD_GAME_FILES").expect("set LSD_GAME_FILES to the extraction root"),
    );
    assert!(is_canonical_reference(
        &std::fs::read(root.join("SLPS_015.56")).unwrap()
    ));
    let path = std::env::var("LSD_NATIVE_ANIMATION_CASES")
        .expect("set LSD_NATIVE_ANIMATION_CASES to the associated private corpus");
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut clips = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let relative = std::path::Path::new(input["path"].as_str().unwrap());
        assert!(
            relative
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_)))
        );
        let data = std::fs::read(root.join(relative)).unwrap();
        assert_eq!(sha256_hex(&data), input["source_sha256"].as_str().unwrap());
        let at = input["offset"].as_u64().unwrap() as usize;
        let bytes = input["bytes"].as_u64().unwrap() as usize;
        let data = &data[at..at + bytes];
        assert_eq!(sha256_hex(data), input["mom_sha256"].as_str().unwrap());
        let model = Mom::parse(data).unwrap();
        let types: Vec<_> = case["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| part.as_u64().unwrap() as u8)
            .collect();
        assert_eq!(model.initial_part_types, types);
        assert_eq!(
            model.tmd.objects.len(),
            case["primitive_count"].as_u64().unwrap() as usize
        );
        clips.push(model.animations);
    }
    let (cases, observations) = animation_observations::compare_cases(&fixture, Some(&clips));
    assert_eq!((cases, observations), (122, 10228));
    println!(
        "Decoded original inputs match {observations} native states across {cases} animation sequences."
    );
}
