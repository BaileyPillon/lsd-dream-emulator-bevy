use lsd_assets::{
    AudioImmediateCommand, SeqMusicTrack, SequenceBankBinding,
    formats::{Seq, Vab},
    sha256_hex,
};
use lsd_core::{AudioNoteDriver, SequencePitchTable, SpuNoiseMode};
use serde_json::{Value, json};

fn driver_hash(driver: &AudioNoteDriver) -> String {
    let mut bytes = vec![driver.pool.capacity()];
    bytes.extend_from_slice(&driver.pool.lock.to_le_bytes());
    bytes.extend_from_slice(&driver.pool.current.to_le_bytes());
    bytes.extend_from_slice(&driver.pool.key_on.to_le_bytes());
    bytes.extend_from_slice(&driver.pool.key_off.to_le_bytes());
    for slot in &driver.pool.slots {
        bytes.extend_from_slice(&slot.record);
    }
    for registers in &driver.registers {
        for value in registers {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&driver.dirty);
    for value in driver.automation {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&driver.reverb.to_le_bytes());
    bytes.extend_from_slice(&driver.history_index.to_le_bytes());
    bytes.push(driver.reclaim_disabled);
    let c = driver.volume_context;
    bytes.extend_from_slice(&[
        c.bank_volume,
        c.program_volume,
        c.tone_volume,
        c.tone_pan,
        c.program_pan,
        c.request_pan,
        c.request_volume,
    ]);
    bytes.extend_from_slice(&driver.mono.to_le_bytes());
    bytes.extend_from_slice(&driver.adsr_release_offset.to_le_bytes());
    sha256_hex(&bytes)
}

fn channel_hash(track: &SeqMusicTrack) -> String {
    let c = &track.channels;
    let mut bytes = vec![c.current_channel];
    bytes.extend_from_slice(&c.pans);
    bytes.extend_from_slice(&c.programs);
    for value in c.channel_volumes {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in c.sequence_volume {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&c.last_velocity.to_le_bytes());
    sha256_hex(&bytes)
}

fn state(
    track: &SeqMusicTrack,
    driver: &AudioNoteDriver,
    bank: &SequenceBankBinding,
) -> Vec<Value> {
    let c = track.events.clock;
    vec![
        json!([
            c.countdown,
            c.step,
            c.initial_step,
            c.remaining,
            track.events.byte_cursor(),
            track.events.loop_count
        ]),
        json!(driver_hash(driver)),
        json!(channel_hash(track)),
        json!(sha256_hex(
            &bank
                .volume_definition()
                .programs
                .iter()
                .map(|p| p[1])
                .collect::<Vec<_>>()
        )),
    ]
}

fn decode(value: &Value) -> Vec<u8> {
    let s = value.as_str().unwrap();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn check(row: &Value, header: &[u8], body: &[u8], seq: &[u8], pitch: &SequencePitchTable) -> usize {
    assert_eq!(sha256_hex(header), row["header_sha256"]);
    assert_eq!(sha256_hex(body), row["body_sha256"]);
    assert_eq!(sha256_hex(seq), row["seq_sha256"]);
    let vab = Vab::parse(header, body).unwrap();
    let mut bank = SequenceBankBinding::from_vab(&vab, 0, 4096).unwrap();
    let mut track = SeqMusicTrack::new(
        Seq::parse(seq).unwrap(),
        row["rate"].as_u64().unwrap() as u32,
        0,
    )
    .unwrap();
    track.channels.sequence_volume = [52; 2];
    let mut driver = AudioNoteDriver::new(24);
    driver.tick([0; 24], [0; 2]).unwrap();
    assert_eq!(
        state(&track, &driver, &bank),
        *row["initial"].as_array().unwrap(),
        "{} initial state",
        row["path"]
    );
    let mut noise = SpuNoiseMode::default();
    let mut commands = Vec::new();
    for (frame, expected) in row["frames"].as_array().unwrap().iter().enumerate() {
        driver.pool.lock = 1;
        let mut writes = driver.tick([0; 24], noise.registers).unwrap();
        for w in &writes {
            if w.offset == 0x194 {
                noise.registers[0] = w.value;
            }
            if w.offset == 0x196 {
                noise.registers[1] = w.value;
            }
        }
        track
            .tick(100000, &mut bank, &mut driver, pitch, &mut commands)
            .unwrap();
        for command in commands.drain(..) {
            match command {
                AudioImmediateCommand::Write(w) => writes.push(w),
                AudioImmediateCommand::DisableNoise(mask) => writes.extend(noise.set(0, mask).1),
            }
        }
        driver.pool.lock = 0;
        let mut actual = state(&track, &driver, &bank);
        let mut write_bytes = Vec::new();
        for write in writes {
            write_bytes.extend_from_slice(&write.offset.to_le_bytes());
            write_bytes.extend_from_slice(&write.value.to_le_bytes());
        }
        actual.push(json!(sha256_hex(&write_bytes)));
        actual.push(json!(track.events_dispatched));
        assert_eq!(
            actual,
            *expected.as_array().unwrap(),
            "{} callback {frame}",
            row["path"]
        );
    }
    assert_eq!(track.events.loop_count, 1);
    row["frames"].as_array().unwrap().len()
}

fn pitch(fixture: &Value) -> SequencePitchTable {
    SequencePitchTable {
        values: Box::new(std::array::from_fn(|i| {
            fixture["sequence_pitch_window"][i].as_u64().unwrap() as u16
        })),
    }
}

#[test]
fn music_track_matches_original_global_callback_for_owned_banks_sequences_and_three_rates() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/music-track.json")).unwrap();
    let pitch = pitch(&fixture);
    let count: usize = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            check(
                row,
                &decode(&row["header_hex"]),
                &decode(&row["body_hex"]),
                &decode(&row["seq_hex"]),
                &pitch,
            )
        })
        .sum();
    assert_eq!(count, 201);
}

#[test]
#[ignore = "requires original bank/sequence files and private full-loop original global callback observations"]
fn every_playable_original_sequence_matches_sdk_driver_channels_and_register_write_order() {
    let root = std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap());
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_MUSIC_TRACK_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    let pitch = pitch(&fixture);
    let mut count = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let bank = root.join(row["bank_path"].as_str().unwrap());
        count += check(
            row,
            &std::fs::read(&bank).unwrap(),
            &std::fs::read(bank.with_extension("VB")).unwrap(),
            &std::fs::read(root.join(row["path"].as_str().unwrap())).unwrap(),
            &pitch,
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 69);
    assert_eq!(count, 37859);
}
