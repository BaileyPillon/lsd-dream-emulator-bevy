#[path = "support/link_audio.rs"]
#[allow(dead_code)]
mod support;

use lsd_assets::{SequenceBankBinding, formats::Vab, sha256_hex};
use lsd_core::{SequenceNoteRequest, SequencePitchTable};
use serde_json::Value;

fn expected_driver(state: &Value, case: &Value) -> lsd_core::AudioNoteDriver {
    let mut driver = support::driver(state);
    driver.mono = case["mono"].as_i64().unwrap() as i16;
    driver.adsr_release_offset = case["adsr_release_offset"].as_u64().unwrap() as u16;
    driver
}

fn check_bank(bank: &Value, header: &[u8], body: &[u8], pitch: &SequencePitchTable) -> usize {
    assert_eq!(sha256_hex(header), bank["header_sha256"]);
    assert_eq!(sha256_hex(body), bank["body_sha256"]);
    let vab = Vab::parse(header, body).unwrap();
    let bank_id = bank["bank_id"].as_i64().unwrap() as i16;
    let binding =
        SequenceBankBinding::from_vab(&vab, bank_id, bank["spu_base"].as_u64().unwrap() as u32)
            .unwrap();
    for (index, case) in bank["cases"].as_array().unwrap().iter().enumerate() {
        let label = format!("{} case {index}", bank["path"].as_str().unwrap());
        let mut driver = expected_driver(&case["before"], case);
        let program = case["program"].as_u64().unwrap() as u8;
        let request = SequenceNoteRequest {
            sequence: case["sequence"].as_i64().unwrap() as i16,
            key: case["key"].as_u64().unwrap() as u16,
            velocity: case["velocity"].as_u64().unwrap() as u16,
            pan: case["pan"].as_u64().unwrap() as u16,
            channel_volume: case["channel_volume"].as_i64().unwrap() as i16,
            sequence_volume: std::array::from_fn(|i| {
                case["sequence_volume"][i].as_u64().unwrap() as u16
            }),
        };
        let started = driver
            .sequence_key_on(binding.program(program).unwrap(), pitch, request)
            .unwrap_or_else(|error| panic!("{label}: {error:?}"));
        assert_eq!(
            i64::from(started.result),
            case["result"].as_i64().unwrap(),
            "{label} result"
        );
        let calls: Vec<_> = started
            .disable_noise
            .iter()
            .map(|mask| serde_json::json!([0, mask]))
            .collect();
        assert_eq!(
            calls,
            *case["noise_calls"].as_array().unwrap(),
            "{label} noise calls"
        );
        assert_eq!(
            driver,
            expected_driver(&case["after"], case),
            "{label} after key on"
        );
        for (count, state) in [("released", "stopped"), ("released_again", "stopped_again")] {
            let released = driver
                .sequence_key_off(request.sequence, bank_id, i16::from(program), request.key)
                .unwrap();
            assert_eq!(
                u64::from(released),
                case[count].as_u64().unwrap(),
                "{label} {count}"
            );
            assert_eq!(
                driver,
                expected_driver(&case[state], case),
                "{label} {state}"
            );
        }
    }
    bank["cases"].as_array().unwrap().len()
}

fn pitch(fixture: &Value) -> SequencePitchTable {
    SequencePitchTable {
        values: Box::new(std::array::from_fn(|i| {
            fixture["sequence_pitch_window"][i].as_u64().unwrap() as u16
        })),
    }
}

#[test]
fn owned_music_notes_match_original_overlaps_sparse_programs_lock_and_repeated_release() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/sequence-notes.json")).unwrap();
    let pitch = pitch(&fixture);
    let count: usize = fixture["banks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bank| {
            check_bank(
                bank,
                &support::decode(bank["header_hex"].as_str().unwrap()),
                &support::decode(bank["body_hex"].as_str().unwrap()),
                &pitch,
            )
        })
        .sum();
    assert_eq!(count, 138);
}

#[test]
#[ignore = "requires original local banks and private original SDK music note observations"]
fn all_nine_original_banks_match_music_note_and_release_state() {
    let root = std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap());
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_SEQUENCE_NOTE_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    let pitch = pitch(&fixture);
    let banks = fixture["banks"].as_array().unwrap();
    let count: usize = banks
        .iter()
        .map(|bank| {
            let path = root.join(bank["path"].as_str().unwrap());
            check_bank(
                bank,
                &std::fs::read(&path).unwrap(),
                &std::fs::read(path.with_extension("VB")).unwrap(),
                &pitch,
            )
        })
        .sum();
    assert_eq!((banks.len(), count), (9, 932));
}
