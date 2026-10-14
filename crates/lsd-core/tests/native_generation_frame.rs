#[path = "support/generation.rs"]
mod generation_support;
#[path = "support/generation_layers.rs"]
mod layer_support;
#[path = "support/generation_objects.rs"]
#[allow(dead_code)]
mod object_support;
use lsd_core::{
    DreamGeneration, GenerationFrame, GenerationSound, GenerationStyle, PsxRng, SceneGrid,
    SpawnLocation,
};
use serde_json::{Value, json};

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
fn original_combined_generation_frames_match_layers_objects_rng_palette_and_sound_order() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/generation-frame.json")).unwrap();
    let tables = generation_support::tables(&fixture["tables"]);
    let objects = object_support::tables(&fixture["object_tables"]);
    let geometry = layer_support::geometry(&fixture["geometry"]);
    let color = layer_support::arr(&fixture["scene_color"], |v| v.as_u64().unwrap() as u8);
    for case in fixture["cases"].as_array().unwrap() {
        let mut generation = DreamGeneration {
            owner_active: true,
            location: case["stage"].as_i64().unwrap() as i32,
            day: 21,
            generation_type: case["kind"].as_i64().unwrap() as i32,
            flags: case["flags"].as_i64().unwrap() as i32,
            effect_palette: case["palette"].as_u64().unwrap() as usize,
            effect_data_color: [249, 3, 254],
            environment_effect: case["environment"]
                .as_bool()
                .unwrap()
                .then_some([33, 66, 99]),
            style: Some(GenerationStyle {
                kind: 0,
                height: 0,
                scene_color: color,
                environment_color: [33, 66, 99],
            }),
            actor_entries: case["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(generation_support::actor_entry)
                .collect(),
            ..Default::default()
        };
        let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        let mut sound = Sound {
            next: 100,
            ..Default::default()
        };
        let grid = SceneGrid {
            columns: 3,
            rows: 4,
            mode: 0,
        };
        for step in case["steps"].as_array().unwrap() {
            let key: [i8; 4] = generation_support::arr(&step["key"], |v| v.as_i64().unwrap() as i8);
            let spawn = SpawnLocation {
                chunk: i16::from_le_bytes([key[0] as u8, key[1] as u8]),
                tile: i16::from_le_bytes([key[2] as u8, key[3] as u8]),
                position: generation_support::arr(&step["offset"], |v| v.as_i64().unwrap() as i16),
            };
            let position = grid.place_spawn([100, -500, -200], spawn).position;
            let view = generation_support::arr(&step["view"], |v| v.as_i64().unwrap() as i32);
            let moves = generation
                .tick_objects_and_layers(
                    &tables,
                    &objects,
                    &geometry,
                    &mut rng,
                    GenerationFrame { position, view },
                )
                .unwrap();
            let mut trace = Vec::new();
            if generation.flags != 0 && (view[0].wrapping_sub(view[1]) / 600).wrapping_mul(3) > 0 {
                trace.push(json!(["color", generation.layers.stage_color.unwrap()]));
            }
            for command in moves {
                trace.push(json!([
                    "move",
                    [
                        i32::from(command.source.x),
                        i32::from(command.source.y),
                        command.source.width,
                        command.source.height
                    ],
                    command.destination
                ]));
            }
            sound.trace.clear();
            let result = generation
                .tick_actors(
                    &tables,
                    grid,
                    [100, -500, -200],
                    Some(position),
                    14,
                    &mut sound,
                )
                .unwrap();
            trace.extend(sound.trace.iter().cloned());
            let actors:Vec<_>=generation.actors.iter().map(|actor| {
                let mut context=actor.context;context[2]=0;context[3]=0;
                json!({"active":actor.active,"entry":actor.active.then_some(actor.entry),"position":actor.position,"distance":actor.distance,"context":context})
            }).collect();
            let actual = json!({"objects":generation.structures.iter().map(object_support::snapshot).collect::<Vec<_>>(),
                "count":generation.structure_count,"rng":rng.state(),"layers":layer_support::snapshot(&generation.layers),
                "actors":actors,"result":result,"trace":trace,"entries":generation.actor_entries.iter().map(|entry|entry.effect).collect::<Vec<_>>(),
                "timer":generation.timer,"index":generation.entry_index});
            assert_eq!(
                actual, step["expected"],
                "kind={} flags={} stage={} seed={} timer={}",
                case["kind"], case["flags"], case["stage"], case["seed"], generation.timer
            );
        }
        generation.stop_with_sound(&mut sound);
        assert!(generation.layers.gradient.is_empty());
        assert!(generation.layers.environment.is_none());
        assert!(generation.structures.is_empty());
        assert!(!generation.owner_active);
    }
}
