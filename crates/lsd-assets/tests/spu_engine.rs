use lsd_assets::sha256_hex;
use lsd_core::{SpuNoise, SpuVoiceEngine};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

fn number(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn owned() -> Value {
    serde_json::from_str(include_str!("fixtures/spu-engine.json")).unwrap()
}

fn snapshot(engine: &SpuVoiceEngine) -> Vec<u32> {
    let mut values = vec![
        u32::from(engine.control),
        engine.noise.divider,
        engine.noise.counter,
        u32::from(engine.noise.level),
        engine.pitch_modulation,
        engine.noise_mode,
        engine.reverb_mode,
        engine.reverb_base,
        engine.reverb_current,
        engine.cd_volume[0] as i32 as u32,
        engine.cd_volume[1] as i32 as u32,
        engine.external_volume[0] as i32 as u32,
        engine.external_volume[1] as i32 as u32,
        engine.reverb_volume[0] as i32 as u32,
        engine.reverb_volume[1] as i32 as u32,
        engine.irq_address,
        engine.transfer_address,
        engine.key_on,
        engine.key_off,
        engine.block_end,
        engine.capture_address,
        u32::from(engine.irq_asserted),
    ];
    for volume in engine.main_volume {
        values.extend([
            u32::from(volume.control),
            u32::from(volume.level),
            volume.divider,
        ]);
    }
    values.extend(engine.registers.map(u32::from));
    values.extend(engine.auxiliary.map(u32::from));
    for voice in &engine.voices {
        values.extend(voice.decoded.map(|value| u32::from(value as u16)));
        values.extend(voice.decoder.history.map(|value| u32::from(value as u16)));
        values.extend([
            voice.play_delay,
            voice.write_position,
            voice.read_position,
            voice.available,
            u32::from(voice.ignore_loop_start),
            u32::from(voice.header & 15),
            u32::from(voice.header >> 4),
            u32::from(voice.flags),
        ]);
        for volume in voice.volume {
            values.extend([
                u32::from(volume.control),
                u32::from(volume.level),
                volume.divider,
            ]);
        }
        values.extend([
            u32::from(voice.pitch),
            voice.phase,
            voice.start_address,
            voice.current_address,
            u32::from(voice.envelope.control[0]) | u32::from(voice.envelope.control[1]) << 16,
            voice.loop_address,
            voice.output as u32,
            u32::from(voice.envelope.level),
            voice.envelope.divider,
            voice.envelope.phase as u32,
            u32::from(voice.sustain_target),
        ]);
    }
    assert_eq!(values.len(), 1716);
    values
}

fn bytes(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn cd_values(clock: u32, mode: u32) -> [i16; 2] {
    if mode == 0 {
        [0; 2]
    } else {
        [
            ((clock * 97 + 12345) & 65535) as i32 - 32768,
            ((clock * 173 + 54321) & 65535) as i32 - 32768,
        ]
        .map(|value| value as i16)
    }
}

fn run_case(engine: &mut SpuVoiceEngine, case: &Value, label: &str) {
    let checkpoints: BTreeMap<_, _> = case["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (number(&row["clock"]), row))
        .collect();
    let mut actions: BTreeMap<u32, Vec<&Value>> = BTreeMap::new();
    for row in case["actions"].as_array().unwrap() {
        actions.entry(number(&row[0])).or_default().push(row);
    }
    let mut reads = Vec::new();
    let mut frames = Vec::new();
    for clock in 0..number(&case["clocks"]) {
        if let Some(operations) = actions.get(&clock) {
            for row in operations {
                let value = number(&row[3]);
                match row[1].as_str().unwrap() {
                    "write" => engine
                        .write_register(number(&row[2]) as u16, value as u16)
                        .unwrap(),
                    "read" => reads.push([
                        i64::from(clock),
                        i64::from(number(&row[2])),
                        i64::from(engine.read_register(number(&row[2]) as u16).unwrap()),
                    ]),
                    "dma_write" => engine.dma_write(value),
                    "dma_read" => reads.push([i64::from(clock), -1, i64::from(engine.dma_read())]),
                    _ => panic!("invalid fixture action"),
                }
            }
        }
        let frame = engine
            .clock_voices(cd_values(clock, number(&case["cd_mode"])))
            .unwrap();
        for value in [
            frame.dry[0],
            frame.dry[1],
            i32::from(frame.reverb_input[0]),
            i32::from(frame.reverb_input[1]),
        ] {
            frames.extend_from_slice(&value.to_le_bytes());
        }
        if let Some(expected) = checkpoints.get(&clock) {
            let actual = snapshot(engine);
            if let Some(expected_words) = expected["state"].as_array() {
                assert_eq!(actual.len(), expected_words.len());
                for (index, (actual, expected)) in actual.iter().zip(expected_words).enumerate() {
                    assert_eq!(
                        *actual,
                        number(expected),
                        "{label} clock {clock} state word {index}"
                    );
                }
            } else {
                assert_eq!(
                    sha256_hex(&bytes(&actual)),
                    expected["sha256"].as_str().unwrap(),
                    "{label} clock {clock}"
                );
            }
        }
    }
    assert_eq!(
        serde_json::to_value(reads).unwrap(),
        case["reads"],
        "{label} reads"
    );
    assert_eq!(
        sha256_hex(&frames),
        case["frame_sha256"].as_str().unwrap(),
        "{label} frame accumulators"
    );
    assert_eq!(
        sha256_hex(&engine.memory.to_bytes()),
        case["ram_sha256"].as_str().unwrap(),
        "{label} sample/capture RAM"
    );
}

#[test]
fn voice_register_memory_key_and_mix_scenarios_match_software_reference() {
    let fixture = owned();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    for case in cases {
        let mut engine = SpuVoiceEngine::default();
        for upload in case["uploads"].as_array().unwrap() {
            let hex = upload["hex"].as_str().unwrap();
            let data: Vec<_> = (0..hex.len())
                .step_by(2)
                .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
                .collect();
            engine
                .memory
                .upload(number(&upload["address"]), &data)
                .unwrap();
        }
        run_case(&mut engine, case, case["name"].as_str().unwrap());
    }
}

#[test]
fn every_noise_rate_matches_supplied_software_clocks() {
    let fixture = owned();
    let mut result = Vec::new();
    for rate in 0..number(&fixture["noise"]["rates"]) {
        let mut noise = SpuNoise::default();
        for _ in 0..number(&fixture["noise"]["clocks_per_rate"]) {
            noise.clock((rate << 8) as u16);
            for value in [noise.divider, noise.counter, u32::from(noise.level)] {
                result.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    assert_eq!(
        sha256_hex(&result),
        fixture["noise"]["sha256"].as_str().unwrap()
    );
}

#[test]
fn immediate_sound_commands_preserve_noise_keyoff_and_failed_queue_state() {
    use lsd_assets::{AudioImmediateCommand, dispatch_audio_commands};
    use lsd_core::{AudioRegisterWrite, SpuEngineFault};
    let mut engine = SpuVoiceEngine::default();
    engine.write_register(0x194, 0xffff).unwrap();
    engine.write_register(0x196, 0xffff).unwrap();
    engine.write_register(0x18c, 0x1234).unwrap();
    let mut commands = vec![
        AudioImmediateCommand::Write(AudioRegisterWrite {
            offset: 4,
            value: 0x1000,
        }),
        AudioImmediateCommand::DisableNoise(0xffffff),
        AudioImmediateCommand::Write(AudioRegisterWrite {
            offset: 1,
            value: 0,
        }),
        AudioImmediateCommand::Write(AudioRegisterWrite {
            offset: 2,
            value: 99,
        }),
    ];
    assert_eq!(
        dispatch_audio_commands(&mut engine, &mut commands),
        Err(SpuEngineFault::Register(1))
    );
    assert_eq!(commands.len(), 2);
    assert_eq!(engine.voices[0].pitch, 0x1000);
    assert_eq!(engine.noise_mode, 0);
    assert_eq!(engine.read_register(0x196).unwrap(), 0xff00);
    assert_eq!(engine.key_off, 0x1234);
    commands.remove(0);
    assert_eq!(dispatch_audio_commands(&mut engine, &mut commands), Ok(1));
    assert!(commands.is_empty());
    assert_eq!(engine.voices[0].volume[1].control, 99);
}

#[test]
#[ignore = "requires original sample banks and private SDK note/voice-engine observations"]
fn all_original_sdk_note_bindings_match_supplied_voice_engine_clocks() {
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("set LSD_GAME_FILES"));
    let reference =
        std::env::var_os("LSD_SPU_ENGINE_REFERENCE").expect("set LSD_SPU_ENGINE_REFERENCE");
    let fixture: Value = serde_json::from_slice(&std::fs::read(reference).unwrap()).unwrap();
    let mut count = 0;
    for bank in fixture["banks"].as_array().unwrap() {
        let path = root.join(bank["path"].as_str().unwrap());
        let header = std::fs::read(&path).unwrap();
        let body = std::fs::read(path.with_extension("VB")).unwrap();
        assert_eq!(sha256_hex(&header), bank["header_sha256"].as_str().unwrap());
        assert_eq!(sha256_hex(&body), bank["body_sha256"].as_str().unwrap());
        for tone in bank["tones"].as_array().unwrap() {
            let mut engine = SpuVoiceEngine::default();
            engine
                .memory
                .upload(number(&bank["spu_base"]), &body)
                .unwrap();
            run_case(
                &mut engine,
                tone,
                &format!(
                    "{} program {} tone {}",
                    bank["path"], tone["program"], tone["tone"]
                ),
            );
            count += 1;
        }
    }
    assert_eq!(count, 351);
}
