use lsd_assets::{formats::Vab, sha256_hex};
use lsd_core::{SpuEnvelope, SpuEnvelopePhase, SpuVolume};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

fn owned() -> Value {
    serde_json::from_str(include_str!("fixtures/spu-envelopes.json")).unwrap()
}

fn number(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}

fn phase(value: u32) -> SpuEnvelopePhase {
    match value {
        0 => SpuEnvelopePhase::Attack,
        1 => SpuEnvelopePhase::Decay,
        2 => SpuEnvelopePhase::Sustain,
        3 => SpuEnvelopePhase::Release,
        _ => panic!("invalid fixture phase"),
    }
}

fn append(output: &mut Vec<u8>, values: [u32; 3]) {
    for value in values {
        output.extend_from_slice(&value.to_le_bytes());
    }
}

fn envelope_state(envelope: &SpuEnvelope) -> [u32; 3] {
    [
        u32::from(envelope.level),
        envelope.divider,
        envelope.phase as u32,
    ]
}

fn volume_state(volume: &SpuVolume) -> [u32; 3] {
    [
        u32::from(volume.control),
        u32::from(volume.level),
        volume.divider,
    ]
}

fn control(value: u32) -> [u16; 2] {
    [value as u16, (value >> 16) as u16]
}

#[test]
fn every_envelope_rate_matches_software_clock_reference() {
    let fixture = owned();
    let matrix = &fixture["envelope_matrix"];
    let configs = matrix["configs"].as_array().unwrap();
    assert_eq!(configs.len(), 1088);
    let mut output = Vec::new();
    for config in configs {
        for level in matrix["levels"].as_array().unwrap() {
            for divider in matrix["dividers"].as_array().unwrap() {
                let mut envelope = SpuEnvelope {
                    control: [number(&config[0]) as u16, number(&config[1]) as u16],
                    level: number(level) as u16,
                    divider: number(divider),
                    phase: phase(number(&config[2])),
                };
                for _ in 0..number(&matrix["clocks"]) {
                    envelope.clock();
                    append(&mut output, envelope_state(&envelope));
                }
            }
        }
    }
    assert_eq!(sha256_hex(&output), matrix["sha256"].as_str().unwrap());
}

#[test]
fn every_volume_control_matches_software_clock_reference() {
    let fixture = owned();
    let matrix = &fixture["volume_matrix"];
    assert_eq!(number(&matrix["controls"]), 65536);
    let mut output = Vec::new();
    for control in 0..=65535 {
        for level in matrix["levels"].as_array().unwrap() {
            for divider in matrix["dividers"].as_array().unwrap() {
                let mut volume = SpuVolume {
                    control,
                    level: number(level) as u16,
                    divider: number(divider),
                };
                for _ in 0..number(&matrix["clocks"]) {
                    volume.clock();
                    append(&mut output, volume_state(&volume));
                }
            }
        }
    }
    assert_eq!(sha256_hex(&output), matrix["sha256"].as_str().unwrap());
}

fn observed(output: &[u8], case: &Value) {
    for row in case["observed"].as_array().unwrap() {
        let clock = number(&row[0]) as usize;
        let actual: [u32; 3] = std::array::from_fn(|index| {
            let at = clock * 12 + index * 4;
            u32::from_le_bytes(output[at..at + 4].try_into().unwrap())
        });
        let expected = std::array::from_fn(|index| number(&row[index + 1]));
        assert_eq!(actual, expected, "clock {clock}, initial {}", case["state"]);
    }
    assert_eq!(sha256_hex(output), case["sha256"].as_str().unwrap());
}

#[test]
fn envelope_register_writes_restarts_and_releases_match() {
    for case in owned()["traces"]["envelope"].as_array().unwrap() {
        let state = &case["state"];
        let mut envelope = SpuEnvelope {
            control: control(number(&state[0])),
            level: number(&state[1]) as u16,
            divider: number(&state[2]),
            phase: phase(number(&state[3])),
        };
        let mut output = Vec::new();
        for clock in 0..number(&case["clocks"]) {
            for action in case["actions"].as_array().unwrap() {
                if number(&action[0]) != clock {
                    continue;
                }
                match number(&action[1]) {
                    1 => envelope.restart_attack(),
                    2 => envelope.begin_release(),
                    3 => envelope.control = control(number(&action[2])),
                    4 => envelope.level = number(&action[2]) as u16,
                    _ => panic!("invalid envelope fixture action"),
                }
            }
            envelope.clock();
            append(&mut output, envelope_state(&envelope));
        }
        observed(&output, case);
        assert_eq!(
            [
                u32::from(envelope.control[0]) | u32::from(envelope.control[1]) << 16,
                u32::from(envelope.level),
                envelope.divider,
                envelope.phase as u32
            ],
            std::array::from_fn(|index| number(&case["after"][index]))
        );
    }
}

#[test]
fn volume_register_writes_and_phase_changes_match() {
    for case in owned()["traces"]["volume"].as_array().unwrap() {
        let state = &case["state"];
        let mut volume = SpuVolume {
            control: number(&state[0]) as u16,
            level: number(&state[1]) as u16,
            divider: number(&state[2]),
        };
        let mut output = Vec::new();
        for clock in 0..number(&case["clocks"]) {
            for action in case["actions"].as_array().unwrap() {
                if number(&action[0]) != clock {
                    continue;
                }
                match number(&action[1]) {
                    1 => volume.write_control(number(&action[2]) as u16),
                    2 => volume.level = number(&action[2]) as u16,
                    _ => panic!("invalid volume fixture action"),
                }
            }
            volume.clock();
            append(&mut output, volume_state(&volume));
        }
        observed(&output, case);
        assert_eq!(
            volume_state(&volume),
            std::array::from_fn(|index| number(&case["after"][index]))
        );
    }
}

#[test]
#[ignore = "requires local original VH/VB banks and private software envelope reference"]
fn all_original_tone_envelopes_match_supplied_software_lifecycles() {
    let reference =
        std::env::var_os("LSD_SPU_ENVELOPE_REFERENCE").expect("set LSD_SPU_ENVELOPE_REFERENCE");
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("set LSD_GAME_FILES"));
    let fixture: Value = serde_json::from_slice(&std::fs::read(reference).unwrap()).unwrap();
    let clocks = number(&fixture["clocks"]);
    let release_at = number(&fixture["release_at"]);
    let mut cache = BTreeMap::new();
    let mut tones = 0;
    for item in fixture["banks"].as_array().unwrap() {
        let path = root.join(item["path"].as_str().unwrap());
        let header = std::fs::read(&path).unwrap();
        let body = std::fs::read(path.with_extension("VB")).unwrap();
        assert_eq!(sha256_hex(&header), item["header_sha256"].as_str().unwrap());
        let bank = Vab::parse(&header, &body).unwrap();
        for expected in item["tones"].as_array().unwrap() {
            let program = bank
                .programs
                .iter()
                .find(|p| u32::from(p.id) == number(&expected["program"]))
                .unwrap();
            let tone = &program.tones[number(&expected["tone"]) as usize];
            let parameters = tone.adsr();
            assert_eq!(
                parameters,
                std::array::from_fn(|index| number(&expected["control"][index]) as u16)
            );
            let (hash, after) = cache.entry(parameters).or_insert_with(|| {
                let mut envelope = SpuEnvelope::new(parameters);
                envelope.restart_attack();
                let mut output = Vec::new();
                for clock in 0..clocks {
                    if clock == release_at {
                        envelope.begin_release();
                    }
                    envelope.clock();
                    append(&mut output, envelope_state(&envelope));
                }
                (
                    sha256_hex(&output),
                    [
                        u32::from(parameters[0]) | u32::from(parameters[1]) << 16,
                        u32::from(envelope.level),
                        envelope.divider,
                        envelope.phase as u32,
                    ],
                )
            });
            assert_eq!(hash, expected["sha256"].as_str().unwrap());
            assert_eq!(
                *after,
                std::array::from_fn(|index| number(&expected["after"][index]))
            );
            tones += 1;
        }
    }
    assert_eq!(tones, 351);
    assert_eq!(cache.len(), 75);
}
