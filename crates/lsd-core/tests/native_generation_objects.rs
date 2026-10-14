#[path = "support/generation_objects.rs"]
mod support;
use lsd_core::{GenerationStructure, PsxRng};
use serde_json::Value;

#[test]
fn original_generated_model_sprite_constructors_and_updates_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/generation-objects.json")).unwrap();
    let tables = support::tables(&fixture["tables"]);
    for case in fixture["cases"].as_array().unwrap() {
        let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        let base = std::array::from_fn(|i| case["base"][i].as_i64().unwrap() as i32);
        let mut object = GenerationStructure::construct(
            case["mode"].as_i64().unwrap() as i32,
            support::spawn(&case["spawn"]),
            &tables,
            base,
            &mut rng,
        )
        .unwrap();
        assert_eq!(
            support::snapshot(&object),
            case["expected"],
            "constructor mode={} spawn={}",
            case["mode"],
            case["spawn"]
        );
        assert_eq!(rng.state(), case["rng"].as_u64().unwrap() as u32);
        for update in case["updates"].as_array().unwrap() {
            object.tick = update["tick"].as_u64().unwrap() as u32;
            let base = std::array::from_fn(|i| update["base"][i].as_i64().unwrap() as i32);
            object
                .update(
                    &tables,
                    base,
                    update["delta"].as_i64().unwrap() as i32,
                    &mut rng,
                )
                .unwrap();
            assert_eq!(
                support::snapshot(&object),
                update["expected"],
                "update mode={} spawn={} tick={}",
                case["mode"],
                case["spawn"],
                update["tick"]
            );
            assert_eq!(rng.state(), update["rng"].as_u64().unwrap() as u32);
        }
    }
}
