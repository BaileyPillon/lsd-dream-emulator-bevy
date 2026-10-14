#[path = "../../lsd-core/tests/support/generation_objects.rs"]
#[allow(dead_code)]
mod support;
use lsd_assets::{GenerationAssets, load_generation_object_tables, load_generation_tables};
use lsd_core::{DreamGeneration, GenerationChild, PsxRng};
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
#[ignore = "Requires local original assets and private generated-object CPU observations"]
fn original_generation_object_content_builds_updates_and_textures_match() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_GENERATION_OBJECT_CASES")
                .expect("Set LSD_NATIVE_GENERATION_OBJECT_CASES"),
        )
        .unwrap(),
    )
    .unwrap();
    let tables = load_generation_tables(&executable).unwrap();
    let objects = load_generation_object_tables(&executable).unwrap();
    assert_eq!(objects, support::tables(&fixture["object_tables"]));
    let assets = GenerationAssets::load(&files, &executable).unwrap();
    let raw =
        lsd_assets::formats::Tmd::parse(&std::fs::read(files.join("CDI/ETC/DREAMER.TMD")).unwrap())
            .unwrap();
    let [x, y] = objects.palette_coordinates;
    let clut = (i32::from(x) / 16 + (i32::from(y) << 6)) as u16;
    for index in objects.model_indices {
        let original = &raw.objects[index as usize].polygons[0];
        let loaded = &assets.model.objects[index as usize].polygons[0];
        assert_eq!(loaded.clut, Some(clut));
        assert_eq!(loaded.texture_page, original.texture_page);
        assert_eq!(loaded.vertices, original.vertices);
        assert_eq!(loaded.uv, original.uv);
        assets
            .vram
            .texture_rgba(loaded.texture_page.unwrap(), loaded.clut.unwrap())
            .unwrap();
    }
    let mut sampled_sprites = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let mut generation = DreamGeneration {
            effect_data_color: tables.palette[0],
            ..Default::default()
        };
        generation
            .start(
                &tables,
                case["stage"].as_i64().unwrap() as i32,
                case["day"].as_i64().unwrap() as i32,
                0,
            )
            .unwrap();
        let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        generation
            .build_structures(&tables, &objects, &mut rng, [-1000, 500, -700], 1234)
            .unwrap();
        let actual = json!({"objects":generation.structures.iter().map(support::snapshot).collect::<Vec<_>>(),"count":generation.structure_count,"rng":rng.state()});
        assert_eq!(
            actual, case["expected"],
            "stage={} day={} seed={}",
            case["stage"], case["day"], case["seed"]
        );
        for object in &generation.structures {
            for child in &object.children {
                if let GenerationChild::Sprite(sprite) = child {
                    let (width, height, pixels) = assets.sprite_rgba(sprite).unwrap();
                    assert_eq!(pixels.len(), (width * height * 4) as usize);
                    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
                    sampled_sprites += 1;
                }
            }
        }
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
                        .map(support::snapshot)
                        .collect::<Vec<_>>()
                ),
                update["objects"]
            );
            assert_eq!(rng.state(), update["rng"].as_u64().unwrap() as u32);
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 168);
    assert!(sampled_sprites > 0);
}
