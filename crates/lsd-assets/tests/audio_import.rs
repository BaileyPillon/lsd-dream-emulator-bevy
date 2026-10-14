use lsd_assets::formats::{Seq, SeqEventKind, Vab};
use lsd_assets::sha256_hex;
use serde_json::Value;

fn owned() -> Value {
    serde_json::from_str(include_str!("fixtures/audio-import.json")).unwrap()
}

fn bytes(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn number(value: &Value) -> u64 {
    value.as_u64().unwrap()
}

fn compare_bank(header: &[u8], body: &[u8], expected: &Value) {
    assert_eq!(sha256_hex(header), expected["header_sha256"]);
    assert_eq!(sha256_hex(body), expected["body_sha256"]);
    let bank = Vab::parse(header, body).unwrap();
    let sdk = &expected["sdk_header"];
    assert_eq!(bank.header.version as u64, number(&sdk[4]));
    for index in [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    ] {
        assert_eq!(u64::from(header[index]), number(&sdk[index]));
    }
    let programs = expected["programs"].as_array().unwrap();
    assert_eq!(bank.programs.len(), programs.len());
    assert_eq!(
        bank.body_prefix.len() as u64,
        number(&expected["body_prefix_bytes"])
    );
    for expected in programs {
        let id = number(&expected["id"]) as u8;
        let program = bank.program(id).unwrap();
        for index in [0, 1, 2, 3, 4, 6, 7] {
            assert_eq!(
                u64::from(program.raw[index]),
                number(&expected["sdk_attributes"][index])
            );
        }
        let tones = expected["tones"].as_array().unwrap();
        assert_eq!(program.tones.len(), tones.len());
        for (index, (tone, row)) in program.tones.iter().zip(tones).enumerate() {
            for byte in (0..14).chain(16..24) {
                assert_eq!(u64::from(tone.raw[byte]), number(&row[byte]));
            }
            assert_eq!(i16::from(id), tone.parent_program());
            assert_eq!(u64::from(tone.center_note()), number(&row[4]));
            assert_eq!(u64::from(tone.pitch_correction()), number(&row[5]));
            assert_eq!(
                tone.adsr(),
                [
                    (number(&row[16]) | number(&row[17]) << 8) as u16,
                    (number(&row[18]) | number(&row[19]) << 8) as u16,
                ]
            );
            assert_eq!(
                bank.tone_for_packed_note(i32::from(id) * 16 + index as i32),
                Some(tone)
            );
            assert!(bank.sample(tone.sample_number() as u16).is_some());
        }
    }
    assert_eq!(
        bank.samples.len(),
        expected["samples"].as_array().unwrap().len()
    );
    for (sample, expected) in bank
        .samples
        .iter()
        .zip(expected["samples"].as_array().unwrap())
    {
        assert_eq!(u64::from(sample.number), number(&expected["number"]));
        assert_eq!(sample.body_offset as u64, number(&expected["offset"]));
        assert_eq!(sample.data.len() as u64, number(&expected["bytes"]));
        assert_eq!(sha256_hex(sample.data), expected["sha256"]);
    }
    assert!(bank.sample(0).is_none());
    assert!(bank.tone_for_packed_note(-1).is_none());
}

fn compare_sequence(data: &[u8], expected: &Value) {
    assert_eq!(sha256_hex(data), expected["sha256"]);
    let seq = Seq::parse(data).unwrap();
    let initial = &expected["initial"];
    if initial["fault"].is_string() {
        assert_eq!(initial["fault"], "zero_tempo_division_trap");
        assert_eq!(seq.header.tempo_microseconds, 0);
        assert_eq!(seq.header.initial_tempo_bpm(), None);
    } else {
        assert_eq!(seq.header.resolution as u64, number(&initial["resolution"]));
        assert_eq!(
            seq.header.initial_tempo_bpm().map(u64::from),
            Some(number(&initial["tempo_bpm"]))
        );
        assert_eq!(seq.events[0].after_delta as u64, number(&initial["cursor"]));
        assert_eq!(
            seq.events[0].sdk_delta() as u64,
            number(&initial["delta_scaled"])
        );
    }
    let events = expected["events"].as_array().unwrap();
    assert_eq!(seq.events.len(), events.len());
    let mut accumulator = 0_u32;
    for (actual, expected) in seq.events.iter().zip(events) {
        assert_eq!(
            actual.delta_offset as u64,
            number(&expected["delta_offset"])
        );
        assert_eq!(actual.after_delta as u64, number(&expected["after_delta"]));
        assert_eq!(actual.delta as u64, number(&expected["delta"]));
        assert_eq!(actual.tick, number(&expected["tick"]));
        assert_eq!(
            actual.sdk_delta() as u64,
            number(&expected["native_delta_scaled"])
        );
        accumulator = accumulator.wrapping_add(actual.sdk_delta());
        assert_eq!(
            u64::from(accumulator),
            number(&expected["native_accumulator"])
        );
        match &actual.kind {
            SeqEventKind::Channel { status, data, len } => {
                assert_eq!(u64::from(*status), number(&expected["status"]));
                let args = expected["data"].as_array().unwrap();
                assert_eq!(usize::from(*len), args.len());
                for (byte, arg) in data.iter().zip(args) {
                    assert_eq!(u64::from(*byte), number(arg));
                }
            }
            SeqEventKind::End => assert_eq!(expected["status"], 255),
        }
    }
}

#[test]
fn owned_sparse_banks_preserve_original_sdk_attributes_and_sample_boundaries() {
    let fixture = owned();
    for row in fixture["banks"].as_array().unwrap() {
        let header = bytes(&row["header_hex"]);
        let body = bytes(&row["body_hex"]);
        compare_bank(&header, &body, row);
        let bank = Vab::parse(&header, &body).unwrap();
        assert_eq!(bank.body_prefix.len(), 16);
        assert_eq!(
            bank.samples
                .iter()
                .map(|s| s.body_offset)
                .collect::<Vec<_>>(),
            [16, 32]
        );
        assert!(bank.program(1).is_none());
    }
}

#[test]
fn owned_sequences_preserve_running_status_order_and_original_delta_counter() {
    let fixture = owned();
    for row in fixture["sequences"].as_array().unwrap() {
        compare_sequence(&bytes(&row["hex"]), row);
    }
    let seq = Seq::parse(&bytes(&fixture["sequences"][0]["hex"])).unwrap();
    assert_eq!(
        seq.events.iter().map(|e| e.delta).collect::<Vec<_>>(),
        [0, 127, 128, 16384, 268435455, 0]
    );
    assert!(matches!(
        seq.events[1].kind,
        SeqEventKind::Channel {
            status: 0x90,
            data: [61, 0],
            len: 2
        }
    ));
}

#[test]
fn vab_rejects_truncated_tables_overflowing_counts_and_invalid_tone_references() {
    let fixture = owned();
    let header = bytes(&fixture["banks"][0]["header_hex"]);
    let body = bytes(&fixture["banks"][0]["body_hex"]);
    for end in 0..header.len() {
        let mut truncated = header[..end].to_vec();
        if end >= 16 {
            truncated[12..16].copy_from_slice(&((end + body.len()) as u32).to_le_bytes());
        }
        assert!(Vab::parse(&truncated, &body).is_err());
    }
    for (at, value) in [
        (18, 129_u16),
        (22, 255),
        (2080 + 20, 3),
        (2080 + 22, 0),
        (3104, 65535),
    ] {
        let mut bad = header.clone();
        bad[at..at + 2].copy_from_slice(&value.to_le_bytes());
        assert!(Vab::parse(&bad, &body).is_err(), "offset {at}");
    }
    let mut bad = header.clone();
    bad[32] = 17;
    assert!(Vab::parse(&bad, &body).is_err());
    for end in 0..body.len() {
        let mut adjusted = header.clone();
        adjusted[12..16].copy_from_slice(&((header.len() + end) as u32).to_le_bytes());
        assert!(Vab::parse(&adjusted, &body[..end]).is_err());
    }
}

#[test]
fn seq_rejects_missing_status_unbounded_deltas_and_incomplete_events() {
    let data = bytes(&owned()["sequences"][0]["hex"]);
    for end in 0..data.len() {
        assert!(Seq::parse(&data[..end]).is_err());
    }
    for suffix in [
        vec![0, 60, 0],
        vec![0x81, 0x80, 0x80, 0x80, 0, 0xff, 0x2f, 0],
        vec![0, 0x90, 60, 0xff],
        vec![0, 0xff, 0x51, 3, 1, 2, 3],
        vec![0, 0xff, 0x2f, 0, 0],
    ] {
        let mut bad = data[..15].to_vec();
        bad.extend(suffix);
        assert!(Seq::parse(&bad).is_err());
    }
    for (at, value) in [(0, 0), (7, 2), (8, 0x80), (9, 0)] {
        let mut bad = data.clone();
        bad[at] = value;
        if at == 9 {
            bad[8] = 0;
        }
        assert!(Seq::parse(&bad).is_err());
    }
}

#[test]
#[ignore = "requires local original sound banks/sequences and original SDK recordings"]
fn original_audio_files_match_identified_sdk_attribute_and_delta_recordings() {
    let files =
        std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("LSD_GAME_FILES"));
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("LSD_NATIVE_AUDIO_IMPORT").expect("LSD_NATIVE_AUDIO_IMPORT"),
        )
        .unwrap(),
    )
    .unwrap();
    let banks = fixture["banks"].as_array().unwrap();
    let sequences = fixture["sequences"].as_array().unwrap();
    assert_eq!(banks.len(), 9);
    assert_eq!(sequences.len(), 70);
    for row in banks {
        let path = files.join(row["path"].as_str().unwrap());
        compare_bank(
            &std::fs::read(&path).unwrap(),
            &std::fs::read(path.with_extension("VB")).unwrap(),
            row,
        );
    }
    let mut events = 0;
    for row in sequences {
        compare_sequence(
            &std::fs::read(files.join(row["path"].as_str().unwrap())).unwrap(),
            row,
        );
        events += row["events"].as_array().unwrap().len();
    }
    assert_eq!(events, 38976);
    assert_eq!(
        sequences
            .iter()
            .filter(|s| s["initial"]["fault"].is_string())
            .count(),
        1
    );
    println!(
        "original_audio_banks=9 programs=102 tones=351 samples=255 sequences=70 delta_observations=38976 original_initializations=69 zero_tempo_faults=1 playback_accepted=false"
    );
}
