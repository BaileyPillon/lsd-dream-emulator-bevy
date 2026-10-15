use lsd_assets::{formats::Vab, load_spu_pitch_table};
use serde_json::Value;

fn bytes(text: &Value) -> Vec<u8> {
    let text = text.as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn imported_owned_tone_uses_its_own_center_and_correction() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/audio-import.json")).unwrap();
    let table = lsd_core::SpuPitchTable {
        values: Box::new(std::array::from_fn(|i| i as u16 + 1000)),
    };
    for row in fixture["banks"].as_array().unwrap() {
        let header = bytes(&row["header_hex"]);
        let body = bytes(&row["body_hex"]);
        let bank = Vab::parse(&header, &body).unwrap();
        for program in bank.programs {
            for tone in program.tones {
                assert_eq!(
                    tone.pitch(&table, i32::from(tone.center_note()), 0)
                        .unwrap(),
                    1000
                );
                assert_eq!(
                    tone.pitch(&table, i32::from(tone.center_note()) + 12, 0)
                        .unwrap(),
                    2000
                );
            }
        }
    }
    assert!(load_spu_pitch_table(&[]).is_err());
}

#[test]
#[ignore = "requires original executable and original pitch recordings"]
fn original_pitch_table_import_and_recordings_match() {
    let exe = std::fs::read(std::env::var_os("LSD_EXECUTABLE").expect("LSD_EXECUTABLE")).unwrap();
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_NATIVE_PITCH").expect("LSD_NATIVE_PITCH")).unwrap(),
    )
    .unwrap();
    let table = load_spu_pitch_table(&exe).unwrap();
    for (value, expected) in table
        .values
        .iter()
        .zip(expected["pitch_table"].as_array().unwrap())
    {
        assert_eq!(u64::from(*value), expected.as_u64().unwrap());
    }
    let rows = expected["pitch"].as_array().unwrap();
    assert_eq!(rows.len(), 3159);
    for row in rows {
        let value = table
            .pitch(
                row["note"].as_i64().unwrap() as i32,
                row["fine"].as_i64().unwrap() as i32,
                row["center"].as_u64().unwrap() as u8,
                row["correction"].as_u64().unwrap() as u8,
            )
            .unwrap();
        assert_eq!(u64::from(value), row["result"].as_u64().unwrap());
    }
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("LSD_GAME_FILES"));
    let mut coupled = 0;
    for record in expected["bank_pitch"].as_array().unwrap() {
        let path = files.join(record["path"].as_str().unwrap());
        let header = std::fs::read(&path).unwrap();
        let body = std::fs::read(path.with_extension("VB")).unwrap();
        assert_eq!(lsd_assets::sha256_hex(&header), record["header_sha256"]);
        let bank = Vab::parse(&header, &body).unwrap();
        for case in record["cases"].as_array().unwrap() {
            let program = bank
                .program(case["program"].as_u64().unwrap() as u8)
                .unwrap();
            let tone = &program.tones[case["tone"].as_u64().unwrap() as usize];
            let result = tone
                .pitch(
                    &table,
                    case["note"].as_i64().unwrap() as i32,
                    case["fine"].as_i64().unwrap() as i32,
                )
                .unwrap();
            assert_eq!(u64::from(result), case["result"].as_u64().unwrap());
            coupled += 1;
        }
    }
    assert_eq!(coupled, 5265);
    println!(
        "original_pitch_values=192 original_pitch_comparisons=3159 original_bound_tone_pitch_comparisons=5265 audio_playback_accepted=false"
    );
}
