use lsd_core::{
    EncounterChunk, EncounterFrame, EncounterObject, EncounterTables, encounter_object_gate,
};
use serde_json::{Value, json};
#[test]
fn original_encounter_signed_gates_used_flags_palette_and_chunk_selection_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/encounter-gates.json")).unwrap();
    let colors: Vec<i8> = fixture["colors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap() as i8)
        .collect();
    for case in fixture["cases"].as_array().unwrap() {
        let mut object = EncounterObject {
            used: case["used"].as_i64().unwrap() as i8,
            gate: case["gate"].as_i64().unwrap() as i8,
            model: 0,
            entity: 0,
            variants: [-1; 4],
        };
        let result = encounter_object_gate(
            &mut object,
            case["day"].as_i64().unwrap() as i32,
            case["palette"].as_i64().unwrap() as i32,
            case["kind"].as_i64().unwrap() as i32,
            &colors,
        )
        .unwrap();
        assert_eq!(
            json!({"result":result,"used":object.used}),
            case["expected"],
            "case={case}"
        );
    }
    let chunks = fixture["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| EncounterChunk {
            chunk: v["chunk"].as_i64().unwrap() as i16,
            day_gate: v["day_gate"].as_i64().unwrap() as i8,
            objects: [-1; 3],
        })
        .collect::<Vec<_>>();
    let tables = EncounterTables {
        chunks: std::array::from_fn(|_| chunks.clone()),
        objects: std::array::from_fn(|_| Vec::new()),
        color_lookup: colors,
        variants: Vec::new(),
        positions: Vec::new(),
        rotations: Vec::new(),
    };
    for case in fixture["find"].as_array().unwrap() {
        let frame = EncounterFrame {
            stage: case["stage"].as_i64().unwrap() as i32,
            day: 1,
            palette: case["palette"].as_i64().unwrap() as i32,
            generation_type: 0,
            chunk: case["chunk"].as_i64().unwrap() as i16,
        };
        assert_eq!(json!(tables.find_chunk(frame).unwrap()), case["expected"]);
    }
}
