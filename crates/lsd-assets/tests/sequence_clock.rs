use lsd_assets::{formats::Seq, sha256_hex};
use serde_json::Value;

fn check_frames(player: &mut lsd_assets::SeqEventClock, frames: &Value, original: bool) {
    for (index, row) in frames.as_array().unwrap().iter().enumerate() {
        let mut events = Vec::new();
        player
            .tick(100000, |event| events.push(event.after_delta))
            .unwrap();
        let mut actual = vec![
            i64::from(player.clock.countdown),
            i64::from(player.clock.step),
            i64::from(player.clock.initial_step),
            i64::from(player.clock.remaining),
            player.byte_cursor() as i64,
            i64::from(player.loop_count),
        ];
        if !original {
            actual.push(i64::from(player.stopped));
        }
        let expected: Vec<_> = row[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        assert_eq!(actual, expected, "callback {index}");
        let expected_events: Vec<_> = row[1]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(events, expected_events, "callback {index}");
    }
}

#[test]
fn event_clock_and_repeats_match_original_decoder_for_owned_running_status_and_loop_limits() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/sequence-events.json")).unwrap();
    let hex = fixture["owned_seq_hex"].as_str().unwrap();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let seq = Seq::parse(&bytes).unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let mut player = lsd_assets::SeqEventClock::new(
            seq.clone(),
            row["rate"].as_u64().unwrap() as u32,
            row["limit"].as_i64().unwrap() as i16,
        )
        .unwrap();
        player.loop_count = row["initial_loops"].as_u64().unwrap() as u16;
        check_frames(&mut player, &row["frames"], false);
    }
}

#[test]
#[ignore = "requires original SEQ files and private full-loop original SDK clock/decoder traces"]
fn every_playable_original_sequence_matches_native_event_positions_through_its_first_repeat() {
    let root = std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap());
    let reference: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_SEQUENCE_EVENT_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    for row in reference.as_array().unwrap() {
        let bytes = std::fs::read(root.join(row["path"].as_str().unwrap())).unwrap();
        assert_eq!(sha256_hex(&bytes), row["sha256"]);
        let seq = Seq::parse(&bytes).unwrap();
        let mut player = lsd_assets::SeqEventClock::new(seq, 60, 0).unwrap();
        check_frames(&mut player, &row["frames"], true);
        assert_eq!(player.loop_count, 1);
        assert!(!player.stopped);
    }
    assert_eq!(reference.as_array().unwrap().len(), 69);
}
#[test]
#[ignore = "requires original SEQ files and private original SDK timing observations"]
fn all_original_sequences_retain_original_initial_clock_or_original_zero_tempo_fault() {
    let root = std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap());
    let reference: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_SEQUENCE_CLOCK_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    let mut initialized = 0;
    let mut faults = 0;
    for row in reference.as_array().unwrap() {
        let bytes = std::fs::read(root.join(row["path"].as_str().unwrap())).unwrap();
        assert_eq!(sha256_hex(&bytes), row["sha256"]);
        let seq = Seq::parse(&bytes).unwrap();
        for native in row["clocks"].as_array().unwrap() {
            let actual = seq
                .header
                .initial_clock(native["rate"].as_u64().unwrap() as u32, seq.events[0].delta);
            if native.get("fault_pc").is_some() {
                assert_eq!(actual, Err(lsd_core::SequenceClockFault::ZeroTempo));
                faults += 1;
            } else {
                let actual = actual.unwrap();
                let expected = &native["clock"];
                assert_eq!(
                    [
                        i64::from(actual.countdown),
                        i64::from(actual.step),
                        i64::from(actual.initial_step),
                        i64::from(actual.remaining)
                    ],
                    [
                        expected["countdown"].as_i64().unwrap(),
                        expected["step"].as_i64().unwrap(),
                        expected["initial_step"].as_i64().unwrap(),
                        expected["remaining"].as_i64().unwrap()
                    ]
                );
                assert_eq!(
                    u64::from(seq.header.initial_tempo_bpm().unwrap()),
                    native["bpm"].as_u64().unwrap()
                );
                initialized += 1;
            }
        }
    }
    assert_eq!((initialized, faults), (276, 4));
}
