mod support {
    pub mod animation_observations;
}
use support::animation_observations::compare_cases;

#[test]
fn persistent_synthetic_sequences_match_original_native_observations() {
    let fixture = serde_json::from_str(include_str!("fixtures/animation-player.json")).unwrap();
    let (cases, observations) = compare_cases(&fixture, None);
    assert_eq!(cases, 9);
    assert!(observations > 100);
}

#[test]
#[ignore = "Requires private original animation observations and local game assets"]
fn original_animation_corpus_matches_native_observations() {
    let path = std::env::var("LSD_NATIVE_ANIMATION_CASES")
        .expect("set LSD_NATIVE_ANIMATION_CASES to the private native corpus");
    let fixture = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let (cases, observations) = compare_cases(&fixture, None);
    assert_eq!(cases, 122);
    assert!(observations > 9000);
    println!(
        "Matched {cases} original animation sequences across {observations} observed operations."
    );
}
