use lsd_core::{
    LinkCatalog, LinkContext, LinkKind, LinkSet, LinkTrigger, PsxRng, SpawnEntry, SpawnLocation,
};
use serde_json::Value;

pub fn kind(value: &Value) -> LinkKind {
    match value.as_str().unwrap() {
        "wall" => LinkKind::Wall,
        "tunnel" => LinkKind::Tunnel,
        "teleport" => LinkKind::Teleport,
        "staircase" => LinkKind::Staircase,
        _ => panic!("Unknown native fixture link kind"),
    }
}

pub fn location(value: &Value) -> SpawnLocation {
    SpawnLocation {
        chunk: value[0].as_i64().unwrap() as i16,
        tile: value[1].as_i64().unwrap() as i16,
        position: std::array::from_fn(|axis| value[axis + 2].as_i64().unwrap() as i16),
    }
}

fn entry(value: &Value) -> SpawnEntry {
    SpawnEntry {
        chunk: value["chunk"].as_i64().unwrap() as i16,
        tile: value["tile"].as_i64().unwrap() as i16,
        adjustment: value["adjustment"].as_u64().unwrap() as u8,
        extra: value["extra"].as_i64().unwrap() as i8,
    }
}

fn set(value: &Value) -> LinkSet {
    let indices = |field: &str| {
        value[field].as_array().map(|rows| {
            std::array::from_fn(|stage| {
                rows[stage]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|index| index.as_u64().unwrap() as u8)
                    .collect()
            })
        })
    };
    LinkSet {
        triggers: std::array::from_fn(|stage| {
            value["triggers"][stage]
                .as_array()
                .unwrap()
                .iter()
                .map(|record| LinkTrigger {
                    chunk: record["chunk"].as_i64().unwrap() as i16,
                    tile: record["tile"].as_i64().unwrap() as i16,
                    target_stage: record["target_stage"].as_i64().unwrap() as i8,
                    target_spawn: record["target_spawn"].as_u64().unwrap() as u8,
                })
                .collect()
        }),
        spawns: std::array::from_fn(|stage| {
            value["spawns"][stage]
                .as_array()
                .unwrap()
                .iter()
                .map(entry)
                .collect()
        }),
        trigger_transforms: indices("trigger_transforms"),
        spawn_transforms: indices("spawn_transforms"),
    }
}

pub fn catalog(value: &Value) -> LinkCatalog {
    LinkCatalog {
        adjustments: value["adjustments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|position| std::array::from_fn(|axis| position[axis].as_i64().unwrap() as i16))
            .collect(),
        random_spawns: std::array::from_fn(|stage| {
            value["random_spawns"][stage]
                .as_array()
                .unwrap()
                .iter()
                .map(entry)
                .collect()
        }),
        wall: set(&value["families"]["wall"]),
        tunnel: set(&value["families"]["tunnel"]),
        teleport: set(&value["families"]["teleport"]),
        staircase: set(&value["families"]["staircase"]),
        transforms: value["transforms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| std::array::from_fn(|axis| record[axis].as_u64().unwrap() as u32))
            .collect(),
        facing_angles: value["facing_angles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|angle| angle.as_u64().unwrap() as u16)
            .collect(),
        violence_key: value["violence_key"].as_u64().unwrap() as u32,
    }
}

pub fn initial_context(enabled: i32) -> LinkContext {
    LinkContext {
        source_stage: 13,
        source_index: 7,
        target_stage: 11,
        target_index: 9,
        teleports_enabled: enabled,
        navigation: [2; 30],
        dynamic_penalty: i32::MAX,
    }
}

fn check_output(output: SpawnLocation, context: &LinkContext, case: &Value, label: &str) {
    assert_eq!(output, location(&case["output"]), "location {label}");
    assert_eq!(
        [
            context.source_stage,
            context.source_index,
            context.target_stage,
            context.target_index
        ],
        std::array::from_fn(|index| case["globals"][index].as_i64().unwrap() as i32),
        "globals {label}"
    );
    assert_eq!(
        context.navigation,
        std::array::from_fn(|index| case["navigation"][index].as_u64().unwrap() as u8),
        "navigation {label}"
    );
    assert_eq!(
        context.dynamic_penalty,
        case["penalty"].as_i64().unwrap() as i32,
        "penalty {label}"
    );
}

pub fn static_cases(catalog: &LinkCatalog, cases: &[Value]) {
    for (index, case) in cases.iter().enumerate() {
        let mut output = SpawnLocation {
            chunk: 1337,
            tile: -1337,
            position: [1, 2, 3],
        };
        let mut context = initial_context(case["enabled"].as_i64().unwrap() as i32);
        let result = catalog
            .static_spawn(
                kind(&case["kind"]),
                location(&case["key"]),
                case["stage"].as_i64().unwrap() as i32,
                &mut output,
                &mut context,
            )
            .unwrap();
        assert_eq!(
            result.unwrap_or(-1),
            case["result"].as_i64().unwrap() as i32,
            "static result {index}"
        );
        check_output(output, &context, case, &format!("static {index}"));
    }
}

pub fn spawn_cases(catalog: &LinkCatalog, cases: &[Value], dynamic: bool) {
    for (index, case) in cases.iter().enumerate() {
        let mut output = SpawnLocation {
            chunk: 1337,
            tile: -1337,
            position: [1, 2, 3],
        };
        let mut context = initial_context(1);
        let mut rng = PsxRng::new(case["seed"].as_u64().unwrap() as u32);
        let stage = case["stage"].as_i64().unwrap() as i32;
        let result = if dynamic {
            catalog
                .dynamic_spawn(
                    stage,
                    location(&case["key"]),
                    case["tick"].as_u64().unwrap() as u32,
                    &mut output,
                    &mut context,
                    &mut rng,
                )
                .unwrap()
                .unwrap_or(-1)
        } else {
            catalog
                .random_spawn(stage, &mut output, &mut context, &mut rng)
                .unwrap()
        };
        assert_eq!(
            result,
            case["result"].as_i64().unwrap() as i32,
            "spawn result {index}"
        );
        check_output(output, &context, case, &format!("spawn {index}"));
        assert_eq!(
            rng.state(),
            case["rng_state"].as_u64().unwrap() as u32,
            "RNG {index}"
        );
    }
}
