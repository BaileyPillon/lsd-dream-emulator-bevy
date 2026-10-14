#[path = "support/generation.rs"]
mod support;
use lsd_core::{
    DreamGeneration, GenerationSound, PsxRng, SceneGrid, SpawnLocation, choose_music,
    choose_texture, generation_effect,
};
use serde_json::{Value, json};
use support::*;

#[derive(Default)]
struct Sound {
    next: i32,
    trace: Vec<Value>,
}
impl GenerationSound for Sound {
    fn stop_note(&mut self, handle: i32) -> i32 {
        self.trace.push(json!(["stop", handle]));
        -1
    }
    fn volume_offset(&mut self, offset: i32) {
        self.trace.push(json!(["volume", offset]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        let handle = self.next;
        self.next += 1;
        self.trace
            .push(json!(["play", note, period, counter, handle]));
        handle
    }
}

#[test]
fn original_generation_choices_placement_effects_and_sound_scheduling_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/generation.json")).unwrap();
    let tables = tables(&fixture["tables"]);
    for case in fixture["cases"].as_array().unwrap() {
        let operation = case["operation"].as_str().unwrap();
        let expected = &case["expected"];
        let mut generation = DreamGeneration::default();
        let actual = match operation {
            "start" => {
                generation.index = 77;
                generation.flags = num(&case["flags"]);
                generation.effect_data_color = tables.palette[0];
                generation
                    .start(&tables, num(&case["stage"]), num(&case["day"]), 2)
                    .unwrap();
                let style = generation.style.unwrap();
                json!({"owner":generation.owner_active,"location":generation.location,"day":generation.day,
                    "submode":generation.submode,"timer":generation.timer,"kind":generation.generation_type,
                    "index":generation.index,"flags":generation.flags,"palette":generation.effect_palette,
                    "data_color":generation.effect_data_color,"environment":generation.environment_effect,
                    "style":{"kind":style.kind,"height":style.height,"scene_color":style.scene_color,"environment_color":style.environment_color}})
            }
            "random" | "daily" => {
                let mut rng = PsxRng::new(num(&case["seed"]) as u32);
                generation.day = num(&case["day"]);
                if operation == "random" {
                    generation.seed_random(&tables, &mut rng, num(&case["sample"]));
                } else {
                    generation.seed_daily(&tables, &mut rng);
                }
                json!({"position":generation.spawn.position,"rotation":generation.spawn.rotation,
                    "index":generation.spawn.index,"rng":rng.state()})
            }
            "effect" => {
                let mut context: [i32; 26] = arr(&case["initial"], num);
                generation_effect(
                    num(&case["effect"]) as i8,
                    &mut context,
                    num(&case["distance"]),
                    num(&case["range"]),
                    num(&case["divisor"]),
                )
                .unwrap();
                json!(context)
            }
            "paths" => {
                let mut rng = PsxRng::new(num(&case["seed"]) as u32);
                let mut track = num(&case["override"]);
                let music = choose_music(num(&case["stage"]), &mut track, &mut rng);
                let texture = choose_texture(num(&case["day"]), &mut rng).unwrap();
                json!({"music":music,"texture":texture,"override":track,"rng":rng.state()})
            }
            "find" => {
                generation.actor_entries = tables.actor_entries[0].clone();
                generation.entry_index = num(&case["start"]);
                let mut position = [17, 19, 23];
                let mut distance = 29;
                let input = if case["position"].is_null() {
                    None
                } else {
                    Some(arr(&case["position"], num))
                };
                let entry = generation
                    .find_actor(
                        &tables,
                        SceneGrid {
                            columns: 3,
                            rows: 4,
                            mode: num(&case["mode"]),
                        },
                        [100, -500, -200],
                        input,
                        &mut position,
                        &mut distance,
                    )
                    .unwrap();
                json!({"entry":entry,"position":position,"distance":distance,"index":generation.entry_index})
            }
            "actors" => {
                generation.timer = 1;
                generation.actor_entries = case["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(actor_entry)
                    .collect();
                let mut sound = Sound {
                    next: 100,
                    ..Default::default()
                };
                for step in case["steps"].as_array().unwrap() {
                    let grid = SceneGrid {
                        columns: 3,
                        rows: 4,
                        mode: 0,
                    };
                    let key: [i8; 4] = arr(&step["key"], |v| num(v) as i8);
                    let spawn = SpawnLocation {
                        chunk: i16::from_le_bytes([key[0] as u8, key[1] as u8]),
                        tile: i16::from_le_bytes([key[2] as u8, key[3] as u8]),
                        position: arr(&step["offset"], |v| num(v) as i16),
                    };
                    let position = step["has_position"]
                        .as_bool()
                        .unwrap()
                        .then(|| grid.place_spawn([100, -500, -200], spawn).position);
                    sound.trace.clear();
                    generation.timer = generation.timer.wrapping_add(1);
                    let result = generation
                        .tick_actors(
                            &tables,
                            grid,
                            [100, -500, -200],
                            position,
                            num(&step["wanted"]),
                            &mut sound,
                        )
                        .unwrap();
                    let actors:Vec<_>=generation.actors.iter().map(|actor| {
                        let mut context=actor.context;context[2]=0;context[3]=0;
                        json!({"active":actor.active,"entry":actor.active.then_some(actor.entry),
                            "position":actor.position,"distance":actor.distance,"context":context})
                    }).collect();
                    let actual = json!({"result":result,"actors":actors,"trace":sound.trace,
                        "entries":generation.actor_entries.iter().map(|v|v.effect).collect::<Vec<_>>(),"timer":generation.timer,"index":generation.entry_index});
                    assert_eq!(
                        actual, step["expected"],
                        "effect={} step={step}",
                        case["entries"][0]["effect"]
                    );
                }
                continue;
            }
            _ => panic!("Unknown native generation operation {operation}"),
        };
        assert_eq!(&actual, expected, "operation={operation} case={case}");
    }
}

#[test]
fn generation_reentry_preserves_active_scene_and_retained_slot_storage() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/generation.json")).unwrap();
    let tables = tables(&fixture["tables"]);
    let mut generation = DreamGeneration::default();
    generation.actors[0].context[14] = 12345;
    assert!(generation.start(&tables, 0, 1, 0).unwrap());
    let before = format!("{generation:?}");
    assert!(!generation.start(&tables, 1, 20, 0).unwrap());
    assert_eq!(before, format!("{generation:?}"));
    generation.stop();
    assert!(generation.start(&tables, 1, 20, 0).unwrap());
    assert_eq!(generation.actors[0].context[14], 12345);
}
