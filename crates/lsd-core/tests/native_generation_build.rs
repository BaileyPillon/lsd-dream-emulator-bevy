#[path = "support/generation.rs"]
mod generation_support;
#[path = "support/generation_objects.rs"]
mod object_support;
use lsd_core::{DreamGeneration, GenerationStyle, PsxRng};
use serde_json::{Value, json};

#[test]
fn original_generation_build_and_immediate_update_rng_history_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/generation-build.json")).unwrap();
    let tables = generation_support::tables(&fixture["tables"]);
    let objects = object_support::tables(&fixture["object_tables"]);
    let color = generation_support::arr(&fixture["scene_color"], |v| {
        generation_support::num(v) as u8
    });
    for case in fixture["cases"].as_array().unwrap() {
        let mut generation = DreamGeneration {
            generation_type: generation_support::num(&case["kind"]),
            day: generation_support::num(&case["day"]),
            flags: generation_support::num(&case["flags"]),
            effect_palette: generation_support::num(&case["palette"]) as usize,
            style: Some(GenerationStyle {
                kind: 0,
                height: 0,
                scene_color: color,
                environment_color: color,
            }),
            ..Default::default()
        };
        let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        generation
            .build_structures(&tables, &objects, &mut rng, [-1000, 500, -700], 1234)
            .unwrap();
        assert_eq!(
            generation.spawn,
            object_support::spawn(&case["expected"]["spawn"])
        );
        let actual = json!({"objects":generation.structures.iter().map(object_support::snapshot).collect::<Vec<_>>(),
            "count":generation.structure_count,"rng":rng.state(),"spawn":{"position":generation.spawn.position,
            "rotation":generation.spawn.rotation,"scale":generation.spawn.scale,"index":generation.spawn.index,
            "random":generation.spawn.random,"color":generation.spawn.color,"alternate_color":generation.spawn.alternate_color}});
        assert_eq!(
            actual, case["expected"],
            "kind={} day={} seed={} flags={} palette={}",
            case["kind"], case["day"], case["seed"], case["flags"], case["palette"]
        );
        for update in case["updates"].as_array().unwrap() {
            let base = std::array::from_fn(|i| update["base"][i].as_i64().unwrap() as i32);
            for object in &mut generation.structures {
                object
                    .update(
                        &objects,
                        base,
                        update["delta"].as_i64().unwrap() as i32,
                        &mut rng,
                    )
                    .unwrap();
            }
            assert_eq!(
                json!(
                    generation
                        .structures
                        .iter()
                        .map(object_support::snapshot)
                        .collect::<Vec<_>>()
                ),
                update["objects"]
            );
            assert_eq!(rng.state(), update["rng"].as_u64().unwrap() as u32);
        }
        // The public first-frame path must construct and immediately perform
        // exactly the same first update, rather than defer the constructor RNG.
        let mut first = DreamGeneration {
            generation_type: generation.generation_type,
            day: generation.day,
            flags: generation.flags,
            effect_palette: generation.effect_palette,
            style: generation.style,
            ..Default::default()
        };
        let mut first_rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        first
            .tick_structures(&tables, &objects, &mut first_rng, [-1000, 500, -700], 1234)
            .unwrap();
        assert_eq!(first.timer, 1);
        assert_eq!(
            json!(
                first
                    .structures
                    .iter()
                    .map(object_support::snapshot)
                    .collect::<Vec<_>>()
            ),
            case["updates"][0]["objects"]
        );
        assert_eq!(
            first_rng.state(),
            case["updates"][0]["rng"].as_u64().unwrap() as u32
        );
    }
}
