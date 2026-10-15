use lsd_assets::{SoftwareSound, SoundBankBinding, formats::Vab, load_spu_pitch_table};
use lsd_core::{
    AudioNoteDriver, AudioRegisterWrite, AudioVolumeContext, AudioVolumeFault, GenerationSound,
    SpuNoiseMode, SpuPitchTable,
};
use serde_json::Value;

fn owned() -> Value {
    serde_json::from_str(include_str!("fixtures/audio-clock.json")).unwrap()
}
fn num(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn bytes(v: &Value) -> Vec<u8> {
    let s = v.as_str().unwrap();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn driver(state: &Value) -> AudioNoteDriver {
    let mut driver = AudioNoteDriver::new(num(&state["capacity"]) as i32);
    for (slot, row) in driver
        .pool
        .slots
        .iter_mut()
        .zip(state["slots"].as_array().unwrap())
    {
        slot.record = bytes(row).try_into().unwrap();
    }
    driver.pool.lock = num(&state["lock"]) as i32;
    driver.pool.current = num(&state["current"]) as u16;
    driver.pool.key_on = num(&state["key_on"]);
    driver.pool.key_off = num(&state["key_off"]);
    driver.reverb = num(&state["reverb"]);
    driver.registers =
        std::array::from_fn(|v| std::array::from_fn(|r| num(&state["registers"][v][r]) as u16));
    driver.dirty = std::array::from_fn(|i| num(&state["dirty"][i]) as u8);
    driver.automation = std::array::from_fn(|i| num(&state["automation"][i]));
    let c = &state["volume_context"];
    driver.volume_context = AudioVolumeContext {
        bank_volume: num(&c[0]) as u8,
        program_volume: num(&c[1]) as u8,
        tone_volume: num(&c[2]) as u8,
        tone_pan: num(&c[3]) as u8,
        program_pan: num(&c[4]) as u8,
        request_pan: num(&c[5]) as u8,
        request_volume: num(&c[6]) as u8,
    };
    driver.history_index = num(&state["history_index"]);
    driver.reclaim_disabled = num(&state["reclaim_disabled"]) as u8;
    driver
}
fn compare(actual: &AudioNoteDriver, expected: &Value, label: &str) {
    assert_eq!(
        u32::from(actual.pool.capacity()),
        num(&expected["capacity"]),
        "{label}"
    );
    assert_eq!(
        actual.pool.lock as u32,
        num(&expected["lock"]),
        "{label} lock"
    );
    for (value, key) in [
        (u32::from(actual.pool.current), "current"),
        (actual.pool.key_on, "key_on"),
        (actual.pool.key_off, "key_off"),
        (actual.reverb, "reverb"),
        (actual.history_index, "history_index"),
        (u32::from(actual.reclaim_disabled), "reclaim_disabled"),
    ] {
        assert_eq!(value, num(&expected[key]), "{label} {key}");
    }
    for i in 0..24 {
        assert_eq!(
            actual.pool.slots[i].record.as_slice(),
            bytes(&expected["slots"][i]),
            "{label} slot {i}"
        );
        assert_eq!(
            actual.registers[i],
            std::array::from_fn(|j| num(&expected["registers"][i][j]) as u16),
            "{label} cache {i}"
        );
        assert_eq!(
            u32::from(actual.dirty[i]),
            num(&expected["dirty"][i]),
            "{label} dirty {i}"
        );
    }
    for i in 0..16 {
        assert_eq!(
            actual.automation[i],
            num(&expected["automation"][i]),
            "{label} automation {i}"
        );
    }
    let c = actual.volume_context;
    for (i, value) in [
        c.bank_volume,
        c.program_volume,
        c.tone_volume,
        c.tone_pan,
        c.program_pan,
        c.request_pan,
        c.request_volume,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            u32::from(value),
            num(&expected["volume_context"][i]),
            "{label} context {i}"
        );
    }
}
fn writes(actual: &[AudioRegisterWrite], expected: &Value, label: &str) {
    let actual: Vec<_> = actual
        .iter()
        .map(|w| serde_json::json!([w.offset, w.value]))
        .collect();
    assert_eq!(
        &actual,
        expected.as_array().unwrap(),
        "{label} device writes"
    );
}
fn volumes(cases: &[Value]) {
    for (i, case) in cases.iter().enumerate() {
        let mut state = driver(&case["before"]);
        let result = state.auto_volume(
            case["voice"].as_i64().unwrap() as i32,
            case["first"].as_i64().unwrap() as i32,
            case["target"].as_i64().unwrap() as i32,
            case["duration"].as_i64().unwrap() as i32,
        );
        let trace = case["trace"].as_array().unwrap();
        if trace[0]["fault"].is_string() {
            assert_eq!(result, Err(AudioVolumeFault::DivisionByZero));
        } else {
            assert_eq!(
                i64::from(result.unwrap()),
                trace[0]["result"].as_i64().unwrap()
            );
        }
        compare(&state, &trace[0]["after"], &format!("volume {i} setup"));
        for (tick, expected) in trace.iter().enumerate().skip(1) {
            if num(&expected["active_before"]) != 0 {
                state
                    .advance_volume(case["voice"].as_i64().unwrap() as u16 as usize)
                    .unwrap();
            }
            compare(
                &state,
                &expected["after"],
                &format!("volume {i} tick {tick}"),
            );
        }
    }
}
#[test]
fn noise_commands_write_non_registers_and_leave_key_off_unchanged() {
    for case in owned()["noise"].as_array().unwrap() {
        let mut noise = SpuNoiseMode {
            registers: std::array::from_fn(|i| num(&case["before"]["noise_registers"][i]) as u16),
        };
        let (result, actual) = noise.set(case["mode"].as_i64().unwrap() as i32, num(&case["mask"]));
        assert_eq!(result, num(&case["result"]));
        writes(&actual, &case["writes"], "noise");
        assert_eq!(
            noise.registers,
            std::array::from_fn(|i| num(&case["after"]["noise_registers"][i]) as u16)
        );
        assert_eq!(
            case["before"]["key_off_registers"],
            case["after"]["key_off_registers"]
        );
        assert!(actual.iter().all(|w| matches!(w.offset, 0x194 | 0x196)));
    }
}
#[test]
fn owned_volume_updates_match_original_halfword_parameters_and_cache() {
    volumes(owned()["volumes"].as_array().unwrap());
}
#[test]
fn complete_sdk_tick_matches_shared_volume_context_idle_ring_and_ordered_flush() {
    for case in owned()["ticks"].as_array().unwrap() {
        let mut state = driver(&case["before"]);
        for (i, tick) in case["trace"].as_array().unwrap().iter().enumerate() {
            let levels = std::array::from_fn(|i| num(&tick["levels"][i]) as u16);
            let noise = std::array::from_fn(|i| num(&tick["noise_before"][i]) as u16);
            let actual = state.tick(levels, noise).unwrap();
            writes(&actual, &tick["writes"], &format!("tick {i}"));
            compare(
                &state,
                &tick["after"],
                &format!("scenario {} tick {i}", case["scenario"]),
            );
        }
    }
}
#[test]
fn stop_all_matches_original_records_and_direct_device_writes() {
    for case in owned()["stops"].as_array().unwrap() {
        let mut state = driver(&case["before"]);
        let actual = state.stop_all();
        writes(&actual, &case["writes"], "stop all");
        compare(&state, &case["after"], "stop all");
    }
}
fn sound_object(case: &Value, header: &[u8], body: &[u8], table: &SpuPitchTable) {
    let vab = Vab::parse(header, body).unwrap();
    let mut bank = SoundBankBinding::from_vab(
        &vab,
        case["bank_id"].as_i64().unwrap() as i16,
        num(&case["spu_base"]),
    )
    .unwrap();
    let mut state = driver(&case["before"]);
    let mut commands = Vec::new();
    let mut faults = Vec::new();
    let mut sound = SoftwareSound {
        bank: &mut bank,
        driver: &mut state,
        pitch: table,
        commands: &mut commands,
        faults: &mut faults,
    };
    sound.volume_offset(case["offset_level"].as_i64().unwrap() as i32);
    assert_eq!(
        sound.bank.pitch_offset as u16 as u32,
        num(&case["stored_offset"])
    );
    assert_eq!(
        i64::from(sound.play_note(num(&case["packed_note"]) as i32, 100, 0)),
        case["result"].as_i64().unwrap()
    );
    assert!(commands.is_empty());
    assert!(faults.is_empty());
    compare(&state, &case["after"], "sound object");
}
#[test]
fn owned_sound_object_and_generation_sound_adapter_match_original_calls() {
    let data = owned();
    let table = SpuPitchTable {
        values: Box::new(std::array::from_fn(|i| num(&data["pitch_table"][i]) as u16)),
    };
    for case in data["sound_objects"].as_array().unwrap() {
        sound_object(
            case,
            &bytes(&case["header_hex"]),
            &bytes(&case["body_hex"]),
            &table,
        );
    }
    let case = &data["sound_objects"][0];
    let header = bytes(&case["header_hex"]);
    let body = bytes(&case["body_hex"]);
    let vab = Vab::parse(&header, &body).unwrap();
    let bank = SoundBankBinding::from_vab(&vab, 7, 0x2000).unwrap();
    let mut state = driver(&case["before"]);
    let before = state.clone();
    assert_eq!(
        bank.play_note(&mut state, &table, -1, 100, 0)
            .unwrap()
            .handle(),
        -1
    );
    assert_eq!(state, before);
    assert!(vab.program(7).is_some());
    assert_eq!(
        bank.play_note(&mut state, &table, 0x70, 100, 0),
        Err(lsd_assets::SoundBankFault::MissingTone)
    );
}
#[test]
#[ignore = "requires original audio/executable and original volume/sound-object recordings"]
fn original_volume_and_sound_object_calls_match_all_recorded_game_tones() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("LSD_GAME_FILES"));
    let table = load_spu_pitch_table(
        &std::fs::read(std::env::var_os("LSD_EXECUTABLE").expect("LSD_EXECUTABLE")).unwrap(),
    )
    .unwrap();
    let data: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_NATIVE_AUDIO_CLOCK").expect("LSD_NATIVE_AUDIO_CLOCK"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(data["volumes"].as_array().unwrap().len(), 702);
    volumes(data["volumes"].as_array().unwrap());
    let mut cached = std::collections::BTreeMap::new();
    for case in data["sound_objects"].as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        let (header, body) = cached.entry(path).or_insert_with(|| {
            let path = files.join(path);
            (
                std::fs::read(&path).unwrap(),
                std::fs::read(path.with_extension("VB")).unwrap(),
            )
        });
        sound_object(case, header, body, &table);
    }
    assert_eq!(data["sound_objects"].as_array().unwrap().len(), 696);
    println!(
        "original_volume_cases=702 original_sound_object_cases=696 full_sdk_tick_traces=12 playback_accepted=false"
    );
}
