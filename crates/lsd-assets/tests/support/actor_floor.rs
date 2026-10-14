use lsd_core::{
    CollisionBounds, CollisionChunk, CollisionFace, CollisionModel, CollisionPolicy, CollisionTile,
    SceneCollision, SceneGrid, SceneSlot, SceneSlots,
};
use serde_json::Value;
pub fn array<const N: usize>(value: &Value) -> [i32; N] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i32)
}
pub fn scene(value: &Value) -> SceneCollision {
    let layout = array::<3>(&value["layout"]);
    let slots = &value["slots"];
    SceneCollision {
        slots: SceneSlots {
            grid: SceneGrid {
                columns: layout[0] as i16,
                rows: layout[1] as i16,
                mode: layout[2],
            },
            slots: std::array::from_fn(|i| SceneSlot {
                label: slots[i]["label"].as_i64().unwrap() as i16,
                map_index: slots[i]["map"].as_i64().unwrap() as i16,
                origin: array(&slots[i]["origin"]),
                ready: slots[i]["ready"].as_bool().unwrap(),
            }),
        },
        chunks: std::array::from_fn(|slot| {
            let mut models = vec![];
            let mut cells = vec![vec![]; 400];
            for (cell, records) in slots[slot]["cells"].as_object().unwrap() {
                for record in records.as_array().unwrap() {
                    let points: Vec<[i16; 3]> = record["points"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|point| array::<3>(point).map(|v| v as i16))
                        .collect();
                    let face = match points.len() {
                        3 => CollisionFace::Triangle(points.as_slice().try_into().unwrap()),
                        4 => CollisionFace::Quad(points.as_slice().try_into().unwrap()),
                        _ => panic!("invalid face"),
                    };
                    let model = models.len();
                    models.push(CollisionModel {
                        bounds: CollisionBounds::from_points(&points),
                        faces: vec![face],
                    });
                    cells[cell.parse::<usize>().unwrap()].push(CollisionTile {
                        model,
                        translation: array(&record["translation"]),
                        heading: record["heading"].as_i64().unwrap() as i16,
                        category: 0,
                        flags: record["flags"].as_u64().unwrap() as u8,
                    });
                }
            }
            Some(CollisionChunk {
                policy: CollisionPolicy::for_stage(0),
                models,
                cells,
            })
        }),
    }
}
