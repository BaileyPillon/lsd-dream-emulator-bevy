use lsd_assets::{formats::Vab, load_spu_pitch_table};
use lsd_core::{AudioNoteDriver, SpuPitchTable};
use serde_json::Value;

fn bytes(text: &Value) -> Vec<u8> {
    let text = text.as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn number(value: &Value) -> u32 {
    value.as_u64().unwrap() as u32
}
fn driver(state: &Value, case: &Value) -> AudioNoteDriver {
    let mut driver = AudioNoteDriver::new(24);
    for (slot, raw) in driver
        .pool
        .slots
        .iter_mut()
        .zip(state["slots"].as_array().unwrap())
    {
        slot.record = bytes(raw).try_into().unwrap();
    }
    driver.pool.lock = state["lock"].as_i64().unwrap() as i32;
    driver.pool.current = number(&state["current"]) as u16;
    driver.pool.key_on = number(&state["key_on"]);
    driver.pool.key_off = number(&state["key_off"]);
    driver.reverb = number(&state["reverb"]);
    driver.registers = std::array::from_fn(|voice| {
        std::array::from_fn(|word| number(&state["registers"][voice][word]) as u16)
    });
    driver.dirty = std::array::from_fn(|i| number(&state["dirty"][i]) as u8);
    driver.automation = std::array::from_fn(|i| number(&state["automation"][i]));
    driver.mono = case["mono"].as_i64().unwrap() as i16;
    driver.adsr_release_offset = number(&case["adsr_release_offset"]) as u16;
    driver
}
fn compare(actual: &AudioNoteDriver, expected: &Value, label: &str) {
    assert_eq!(
        i64::from(actual.pool.lock),
        expected["lock"].as_i64().unwrap(),
        "{label} lock"
    );
    for (actual, key) in [
        (u32::from(actual.pool.current), "current"),
        (actual.pool.key_on, "key_on"),
        (actual.pool.key_off, "key_off"),
        (actual.reverb, "reverb"),
    ] {
        assert_eq!(actual, number(&expected[key]), "{label} {key}");
    }
    for voice in 0..24 {
        assert_eq!(
            actual.pool.slots[voice].record.as_slice(),
            bytes(&expected["slots"][voice]),
            "{label} slot {voice}"
        );
        let words: [u16; 8] =
            std::array::from_fn(|i| number(&expected["registers"][voice][i]) as u16);
        assert_eq!(actual.registers[voice], words, "{label} registers {voice}");
        assert_eq!(
            u32::from(actual.dirty[voice]),
            number(&expected["dirty"][voice]),
            "{label} dirty {voice}"
        );
    }
    for i in 0..16 {
        assert_eq!(
            actual.automation[i],
            number(&expected["automation"][i]),
            "{label} automation {i}"
        );
    }
}
fn replay(bank: Vab<'_>, expected: &Value, table: &SpuPitchTable) {
    for (index, case) in expected["cases"].as_array().unwrap().iter().enumerate() {
        let definition = bank
            .prepare_voice_note(
                expected["bank_id"].as_i64().unwrap() as i16,
                number(&case["program"]) as u8,
                number(&case["tone"]) as usize,
                number(&expected["spu_base"]),
            )
            .unwrap();
        let mut driver = driver(&case["before"], case);
        let result = driver
            .key_on(
                &definition,
                table,
                case["note"].as_i64().unwrap() as i32,
                case["fine"].as_i64().unwrap() as i32,
                case["left"].as_i64().unwrap() as i32,
                case["right"].as_i64().unwrap() as i32,
            )
            .unwrap();
        let label = format!("{} case {index}", expected["path"].as_str().unwrap());
        assert_eq!(
            i64::from(result.handle()),
            case["result"].as_i64().unwrap(),
            "{label}"
        );
        let calls = result
            .immediate_key_off
            .map(|mask| vec![serde_json::json!([0, mask])])
            .unwrap_or_default();
        assert_eq!(
            &calls,
            case["force_key_off"].as_array().unwrap(),
            "{label} forced key off"
        );
        compare(&driver, &case["after"], &label);
    }
}
fn owned() -> Value {
    serde_json::from_str(include_str!("fixtures/note-setup.json")).unwrap()
}

#[test]
fn owned_complete_note_setup_matches_original_register_cache_and_voice_records() {
    let fixture = owned();
    let table = SpuPitchTable {
        values: Box::new(std::array::from_fn(|i| {
            number(&fixture["pitch_table"][i]) as u16
        })),
    };
    for expected in fixture["banks"].as_array().unwrap() {
        let header = bytes(&expected["header_hex"]);
        let body = bytes(&expected["body_hex"]);
        let bank = Vab::parse(&header, &body).unwrap();
        replay(bank, expected, &table);
    }
}

#[test]
fn prepared_note_addresses_preserve_body_prefix_and_enforce_ram_and_binding_bounds() {
    let fixture = owned();
    let expected = &fixture["banks"][0];
    let header = bytes(&expected["header_hex"]);
    let body = bytes(&expected["body_hex"]);
    let bank = Vab::parse(&header, &body).unwrap();
    let definition = bank.prepare_voice_note(7, 0, 0, 0x2000).unwrap();
    assert_eq!(definition.sample_address, (0x2000 + 16) / 8);
    assert_eq!(
        bank.prepare_voice_note(7, 7, 1, 0x2000)
            .unwrap()
            .sample_address,
        (0x2000 + 32) / 8
    );
    assert!(bank.prepare_voice_note(16, 0, 0, 0x2000).is_err());
    assert!(bank.prepare_voice_note(7, 1, 0, 0x2000).is_err());
    assert!(bank.prepare_voice_note(7, 0, 1, 0x2000).is_err());
    assert!(bank.prepare_voice_note(7, 0, 0, 0x80000).is_err());
    assert!(bank.prepare_voice_note(7, 0, 0, u32::MAX).is_err());
}

#[test]
#[ignore = "requires local original sound banks, executable and original note-setup recordings"]
fn original_imported_tones_match_complete_sdk_note_setup() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("LSD_GAME_FILES"));
    let exe = std::fs::read(std::env::var_os("LSD_EXECUTABLE").expect("LSD_EXECUTABLE")).unwrap();
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_NATIVE_NOTE_SETUP").expect("LSD_NATIVE_NOTE_SETUP"))
            .unwrap(),
    )
    .unwrap();
    let table = load_spu_pitch_table(&exe).unwrap();
    let mut cases = 0;
    assert_eq!(expected["banks"].as_array().unwrap().len(), 9);
    for expected in expected["banks"].as_array().unwrap() {
        let path = files.join(expected["path"].as_str().unwrap());
        let header = std::fs::read(&path).unwrap();
        let body = std::fs::read(path.with_extension("VB")).unwrap();
        assert_eq!(lsd_assets::sha256_hex(&header), expected["header_sha256"]);
        assert_eq!(lsd_assets::sha256_hex(&body), expected["body_sha256"]);
        cases += expected["cases"].as_array().unwrap().len();
        replay(Vab::parse(&header, &body).unwrap(), expected, &table);
    }
    assert_eq!(cases, 1404);
    println!(
        "original_note_setup_banks=9 tones=351 cases=1404 register_cache_and_all_voice_records=true audio_playback_accepted=false"
    );
}
