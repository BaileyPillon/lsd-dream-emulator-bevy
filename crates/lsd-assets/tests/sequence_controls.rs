#[path = "support/link_audio.rs"]
#[allow(dead_code)]
mod support;

use lsd_assets::{SeqVoiceDispatch, SequenceBankBinding, formats::Vab, sha256_hex};
use lsd_core::SequencePitchTable;
use serde_json::Value;

fn channels(row: &Value) -> SeqVoiceDispatch {
    SeqVoiceDispatch {
        sequence: 0,
        current_channel: row["current_channel"].as_u64().unwrap() as u8,
        pans: std::array::from_fn(|i| row["pans"][i].as_u64().unwrap() as u8),
        programs: std::array::from_fn(|i| row["programs"][i].as_u64().unwrap() as u8),
        channel_volumes: std::array::from_fn(|i| {
            row["channel_volumes"][i].as_i64().unwrap() as i16
        }),
        sequence_volume: std::array::from_fn(|i| {
            row["sequence_volume"][i].as_u64().unwrap() as u16
        }),
        last_velocity: row["last_velocity"].as_u64().unwrap() as u16,
    }
}

fn check(bank: &Value, header: &[u8], body: &[u8], pitch: &SequencePitchTable) -> usize {
    assert_eq!(sha256_hex(header), bank["header_sha256"]);
    assert_eq!(sha256_hex(body), bank["body_sha256"]);
    let vab = Vab::parse(header, body).unwrap();
    let bank_id = bank["bank_id"].as_i64().unwrap() as i16;
    let mut operations = 0;
    for case in bank["cases"].as_array().unwrap() {
        let mut binding =
            SequenceBankBinding::from_vab(&vab, bank_id, bank["spu_base"].as_u64().unwrap() as u32)
                .unwrap();
        let mut driver = support::driver(&case["before"]);
        driver.mono = case["mono"].as_i64().unwrap() as i16;
        let mut dispatch = channels(&case["initial_channels"]);
        for (index, row) in case["operations"].as_array().unwrap().iter().enumerate() {
            let controller = row["controller"].as_u64().unwrap() as u8;
            let value = row["value"].as_u64().unwrap() as u8;
            let label = format!(
                "{} program {} operation {index}",
                bank["path"].as_str().unwrap(),
                case["program"]
            );
            let (status, data) = if controller == 0xe0 {
                (0xe0, [0, value])
            } else {
                (0xb0, [controller, value])
            };
            dispatch
                .channel_event(status, data, &mut binding, &mut driver, pitch)
                .unwrap_or_else(|error| panic!("{label}: {error:?}"));
            assert_eq!(
                dispatch,
                channels(&row["channels"]),
                "{label} channel state"
            );
            let mut expected = support::driver(&row["after"]);
            expected.mono = driver.mono;
            assert_eq!(driver, expected, "{label} driver state");
            for id in 0..128 {
                assert_eq!(
                    u64::from(binding.volume_definition().programs[id][1]),
                    row["program_volumes"][id].as_u64().unwrap(),
                    "{label} shared program {id} volume"
                );
                assert_eq!(
                    binding.program(id as u8).unwrap().program_volume,
                    binding.volume_definition().programs[id][1]
                );
            }
            operations += 1;
        }
    }
    operations
}

fn pitch(fixture: &Value) -> SequencePitchTable {
    SequencePitchTable {
        values: Box::new(std::array::from_fn(|i| {
            fixture["sequence_pitch_window"][i].as_u64().unwrap() as u16
        })),
    }
}

#[test]
fn music_controllers_match_original_for_owned_sparse_programs_overlaps_and_released_voices() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/sequence-controls.json")).unwrap();
    let pitch = pitch(&fixture);
    let operations: usize = fixture["banks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bank| {
            check(
                bank,
                &support::decode(bank["header_hex"].as_str().unwrap()),
                &support::decode(bank["body_hex"].as_str().unwrap()),
                &pitch,
            )
        })
        .sum();
    assert_eq!(operations, 90);
}

#[test]
#[ignore = "requires original local banks and private original B0/E0 handler observations"]
fn all_original_programs_match_volume_pan_expression_and_bend_handlers() {
    let root = std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap());
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_SEQUENCE_CONTROL_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    let pitch = pitch(&fixture);
    let operations: usize = fixture["banks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bank| {
            let path = root.join(bank["path"].as_str().unwrap());
            check(
                bank,
                &std::fs::read(&path).unwrap(),
                &std::fs::read(path.with_extension("VB")).unwrap(),
                &pitch,
            )
        })
        .sum();
    assert_eq!(operations, 1836);
}
