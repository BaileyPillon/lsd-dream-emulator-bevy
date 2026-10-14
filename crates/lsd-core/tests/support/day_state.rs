use crate::support;
use lsd_core::{
    DayTables, DreamDay, DreamLinks, FlashbackRecord, LinkContext, LinkExecution, Mood,
    MoodContributors, PlayerControls, PsxRng, SceneGrid, SpawnLocation,
};
use serde_json::Value;

pub fn num(value: &Value, name: &str) -> i32 {
    value[name].as_i64().unwrap() as i32
}
pub fn arr<T: Copy, const N: usize>(value: &Value, read: impl Fn(&Value) -> T) -> [T; N] {
    std::array::from_fn(|i| read(&value[i]))
}
pub fn mood(value: &Value) -> Mood {
    Mood(arr(value, |v| v.as_i64().unwrap() as i8))
}
pub fn contributors(value: &Value) -> MoodContributors {
    MoodContributors {
        last: mood(&value["last"]),
        dynamic: num(value, "dynamic"),
        upper: num(value, "upper"),
        count: num(value, "count"),
    }
}
pub fn record(value: &Value) -> FlashbackRecord {
    FlashbackRecord {
        stage: num(value, "stage"),
        spawn: support::location(&value["spawn"]),
        rotation: arr(&value["rotation"], |v| v.as_u64().unwrap() as u32),
        seconds: num(value, "seconds") as i16,
        kind: num(value, "kind") as i16,
        padding: arr(&value["padding"], |v| v.as_u64().unwrap() as u8),
        day: num(value, "day"),
    }
}
pub fn tables(value: &Value) -> DayTables {
    DayTables {
        grids: arr(&value["grids"], |v| SceneGrid {
            columns: num(v, "columns") as i16,
            rows: num(v, "rows") as i16,
            mode: num(v, "mode"),
        }),
        moods: std::array::from_fn(|i| {
            value["moods"][i]
                .as_array()
                .unwrap()
                .iter()
                .map(mood)
                .collect()
        }),
        special_days: arr(&value["special_days"], |v| v.as_i64().unwrap() as i16),
        special_marker: mood(&value["special_marker"]),
        colors: arr(&value["colors"], |v| v.as_i64().unwrap() as i8),
        time_limits: arr(&value["time_limits"], |v| v.as_i64().unwrap() as i16),
        save_magic: value["save_magic"].as_u64().unwrap() as u32,
        start_rotation: arr(&value["start_rotation"], |v| v.as_u64().unwrap() as u32),
    }
}
pub fn initial(
    fixture: &Value,
    value: &Value,
) -> (DreamDay, DreamLinks, PlayerControls, LinkContext, PsxRng) {
    let day = DreamDay {
        year: num(value, "year"),
        day: num(value, "day"),
        stored_day: num(value, "stored_day"),
        total_unlock_score: num(value, "total_unlock_score"),
        navigation_score: num(value, "navigation_score"),
        instance_score: num(value, "instance_score"),
        history: arr(&fixture["history"], mood),
        chunks: contributors(&value["chunks"]),
        instances: contributors(&value["instances"]),
        cinematic: arr(&value["cinematic"], |v| v.as_i64().unwrap() as i16),
        flashbacks: arr(&fixture["records"], record),
        flashback_count: num(value, "flashback_count"),
        flashback_index: num(value, "flashback_index"),
        is_new_game: num(value, "is_new_game"),
        screen_shake: num(value, "screen_shake") != 0,
        save_magic: value["save_magic"].as_u64().unwrap() as u32,
        reserved_flags: [0xa5; 8],
        reserved67c: 99,
        reserved680: 101,
        reserved684: [0xa5; 500],
    };
    let links = DreamLinks {
        execution: LinkExecution {
            state: num(value, "state"),
            stage: num(value, "stage"),
            dream_timer: value["timer"].as_u64().unwrap() as u32,
            flashback: num(value, "flashback") != 0,
        },
        spawn: SpawnLocation {
            chunk: 1337,
            tile: -1337,
            position: [1, 2, 3],
        },
        source_transform: None,
        target_transform: None,
        auto_action: None,
        auto_step: 0,
        auto_target: SpawnLocation::default(),
        time_limit: num(value, "limit"),
        default_teleport_transform: false,
    };
    let controls = PlayerControls {
        angles: arr(&value["angles"], |v| v.as_i64().unwrap() as i16),
        input_locked: value["input_locked"].as_bool().unwrap(),
        action_pressed: true,
        ..PlayerControls::default()
    };
    let context = LinkContext {
        navigation: arr(&value["navigation"], |v| v.as_u64().unwrap() as u8),
        dynamic_penalty: num(value, "penalty"),
        ..LinkContext::default()
    };
    (
        day,
        links,
        controls,
        context,
        PsxRng::new(value["seed"].as_u64().unwrap() as u32),
    )
}
