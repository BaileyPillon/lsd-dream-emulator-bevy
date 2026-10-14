use lsd_core::{
    AnimationClip, AnimationFrame, AnimationRecord, AnimationWord, EncounterChunk, EncounterFrame,
    EncounterHost, EncounterObject, EncounterSpawn, EncounterTables, EncounterVariant, ModelActor,
    PsxRng, SceneGrid, transform_number,
};
use serde_json::{Value, json};

fn record(part: u8, kind: u8, flags: u8, payload: &[u32]) -> AnimationRecord {
    AnimationRecord {
        header: AnimationWord {
            part_type: part,
            kind,
            flags,
            word_count: payload.len() as u8 + 1,
        },
        payload: payload.to_vec(),
    }
}
fn owned_clips() -> Vec<AnimationClip> {
    vec![AnimationClip {
        tag: 0,
        header_halfword: 0,
        frames: vec![
            AnimationFrame {
                tag: 0,
                metadata: 0,
                records: vec![
                    record(9, 8, 0, &[]),
                    record(5, 8, 0, &[]),
                    record(9, 2, 0, &[1]),
                    record(5, 2, 0, &[2]),
                    record(9, 3, 0, &[0]),
                    record(5, 3, 0, &[0]),
                ],
            },
            AnimationFrame {
                tag: 1,
                metadata: 0,
                records: vec![record(5, 1, 9, &[1, (-2i32) as u32, 3])],
            },
        ],
    }]
}
struct Host {
    grid: SceneGrid,
    origin: [i32; 3],
    events: Vec<Value>,
    actors: Vec<Value>,
}
impl EncounterHost for Host {
    fn prepare_library(&mut self) -> bool {
        self.events.push(json!(["prepare"]));
        true
    }
    fn enable_teleports(&mut self) {
        self.events.push(json!(["teleport"]));
    }
    fn select_model(&mut self, index: i8) -> bool {
        self.events.push(json!(["model", index]));
        index == 0
    }
    fn create_actor(&mut self, spawn: EncounterSpawn) -> bool {
        let mut actor = ModelActor::construct(&owned_clips(), &[9, 5], 2).unwrap();
        actor.pose.position = self.grid.place_spawn(self.origin, spawn.spawn).position;
        actor.pose.angles = spawn
            .rotation
            .map(|word| (transform_number(word).unwrap() / 360) as i16);
        self.events.push(json!(["actor", self.actors.len()]));
        self.actors.push(json!({"entity":spawn.entity,"variant":spawn.variant,
            "pose":{"position":actor.pose.position,"angles":actor.pose.angles,"scale":actor.pose.scale},
            "parts":actor.animation.parts.len()}));
        true
    }
    fn activate_gray_man(&mut self) {
        self.events.push(json!(["gray"]));
    }
}
#[test]
fn original_coupled_chunk_library_entity_spawn_order_pose_and_rng_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/encounter-program.json")).unwrap();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let objects = case["objects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|object| EncounterObject {
                used: object["used"].as_i64().unwrap() as i8,
                gate: object["gate"].as_i64().unwrap() as i8,
                model: object["model"].as_i64().unwrap() as i8,
                entity: object["entity"].as_u64().unwrap() as u8,
                variants: std::array::from_fn(|i| object["variants"][i].as_i64().unwrap() as i8),
            })
            .collect::<Vec<_>>();
        let mut tables = EncounterTables {
            chunks: std::array::from_fn(|_| {
                vec![EncounterChunk {
                    chunk: input["chunk"].as_i64().unwrap() as i16,
                    day_gate: input["day_gate"].as_i64().unwrap() as i8,
                    objects: [0, -1, -1],
                }]
            }),
            objects: std::array::from_fn(|_| objects.clone()),
            color_lookup: fixture["colors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i8)
                .collect(),
            variants: fixture["variants"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| EncounterVariant {
                    tile: v["tile"].as_i64().unwrap() as i16,
                    rotation: v["rotation"].as_i64().unwrap() as i8,
                    position: v["position"].as_i64().unwrap() as i8,
                })
                .collect(),
            positions: fixture["positions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| std::array::from_fn(|i| v[i].as_i64().unwrap() as i16))
                .collect(),
            rotations: fixture["rotations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| std::array::from_fn(|i| v[i].as_u64().unwrap() as u32))
                .collect(),
        };
        let mut host = Host {
            grid: SceneGrid {
                columns: input["grid"][0].as_i64().unwrap() as i16,
                rows: input["grid"][1].as_i64().unwrap() as i16,
                mode: input["grid"][2].as_i64().unwrap() as i32,
            },
            origin: std::array::from_fn(|i| input["origin"][i].as_i64().unwrap() as i32),
            events: vec![],
            actors: vec![],
        };
        let frame = EncounterFrame {
            stage: input["stage"].as_i64().unwrap() as i32,
            day: input["day"].as_i64().unwrap() as i32,
            palette: input["palette"].as_i64().unwrap() as i32,
            generation_type: input["kind"].as_i64().unwrap() as i32,
            chunk: if input["key_missing"].as_bool().unwrap() {
                -1
            } else {
                input["chunk"].as_i64().unwrap() as i16
            },
        };
        let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
        let prepared = tables.spawn_chunk(frame, &mut rng, &mut host).unwrap();
        assert_eq!(
            json!({"prepared":prepared,"events":host.events,"actors":host.actors,
            "used":tables.objects[frame.stage as usize].iter().map(|v|v.used).collect::<Vec<_>>(),"rng":rng.state()}),
            case["expected"],
            "coupled native case {number}: {input}"
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 161);
}
