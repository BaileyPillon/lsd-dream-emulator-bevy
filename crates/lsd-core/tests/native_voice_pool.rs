use lsd_core::{AudioVoicePool, SpuPitchTable};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/voice-pool.json")).unwrap()
}
fn number(value: &Value) -> u32 {
    value.as_u64().unwrap() as u32
}
fn bytes(text: &Value) -> [u8; 52] {
    let text = text.as_str().unwrap();
    std::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}
fn state(value: &Value) -> AudioVoicePool {
    let mut pool = AudioVoicePool::new(number(&value["capacity"]) as i32);
    for (slot, row) in pool
        .slots
        .iter_mut()
        .zip(value["slots"].as_array().unwrap())
    {
        slot.record = bytes(row);
    }
    pool.lock = value["lock"].as_i64().unwrap() as i32;
    pool.current = number(&value["current"]) as u16;
    pool.key_off = number(&value["key_off"]);
    pool.key_on = number(&value["key_on"]);
    pool
}
fn compare(actual: &AudioVoicePool, expected: &Value, label: &str) {
    assert_eq!(
        u32::from(actual.capacity()),
        number(&expected["capacity"]),
        "{label}"
    );
    assert_eq!(
        i64::from(actual.lock),
        expected["lock"].as_i64().unwrap(),
        "{label}"
    );
    assert_eq!(
        u32::from(actual.current),
        number(&expected["current"]),
        "{label}"
    );
    assert_eq!(
        actual.key_off,
        number(&expected["key_off"]),
        "{label} key off"
    );
    assert_eq!(actual.key_on, number(&expected["key_on"]), "{label} key on");
    for (i, (actual, expected)) in actual
        .slots
        .iter()
        .zip(expected["slots"].as_array().unwrap())
        .enumerate()
    {
        assert_eq!(actual.record, bytes(expected), "{label} slot {i}");
    }
}

#[test]
fn original_software_startup_and_voice_selection_match_all_record_bytes() {
    let data = fixture();
    for row in data["startup"].as_array().unwrap() {
        let requested = row["requested"].as_i64().unwrap() as i32;
        compare(
            &AudioVoicePool::new(requested),
            &row["after"],
            &format!("startup {requested}"),
        );
    }
    for (i, row) in data["allocations"].as_array().unwrap().iter().enumerate() {
        let mut pool = state(&row["before"]);
        let allocation = pool.allocate(number(&row["priority"]) as u8);
        assert_eq!(
            u32::from(allocation.index),
            number(&row["result"]),
            "allocation {i}"
        );
        let expected = row["noise_calls"].as_array().unwrap();
        let calls = allocation
            .disable_noise
            .map(|mask| vec![serde_json::json!([0, mask])])
            .unwrap_or_default();
        assert_eq!(&calls, expected, "allocation {i} noise disable");
        compare(&pool, &row["after"], &format!("allocation {i}"));
    }
}

#[test]
fn original_release_preserves_halfword_handles_masks_and_busy_state() {
    for (i, row) in fixture()["releases"].as_array().unwrap().iter().enumerate() {
        let mut pool = state(&row["before"]);
        let result = pool.release(row["handle"].as_i64().unwrap() as i32);
        assert_eq!(
            i64::from(result),
            row["result"].as_i64().unwrap(),
            "release {i}"
        );
        compare(&pool, &row["after"], &format!("release {i}"));
    }
}

#[test]
fn original_pitch_conversion_matches_owned_table_and_reports_unbounded_indices() {
    let data = fixture();
    let table = SpuPitchTable {
        values: Box::new(std::array::from_fn(|i| {
            number(&data["owned_pitch_table"][i]) as u16
        })),
    };
    for (i, row) in data["pitch"].as_array().unwrap().iter().enumerate() {
        let pitch = table
            .pitch(
                row["note"].as_i64().unwrap() as i32,
                number(&row["fine"]) as i32,
                number(&row["center"]) as u8,
                number(&row["correction"]) as u8,
            )
            .unwrap();
        assert_eq!(u32::from(pitch), number(&row["result"]), "pitch {i}");
    }
    assert!(table.pitch(0, 0, 127, 0).is_err());
    assert!(table.pitch(60, -1, 60, 127).is_err());
}
