#[path = "support/spu_state.rs"]
mod spu_state;
use lsd_assets::sha256_hex;
use lsd_core::{SpuDevice, SpuVoiceFrame};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

fn number(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn owned() -> Value {
    serde_json::from_str(include_str!("fixtures/spu-output.json")).unwrap()
}

fn output_state(device: &SpuDevice) -> Vec<u32> {
    let mut values = spu_state::snapshot(&device.engine);
    for side in &device.output.downsample {
        values.extend(side.map(|value| u32::from(value as u16)));
    }
    for side in &device.output.upsample {
        values.extend(side.map(|value| u32::from(value as u16)));
    }
    values.push(device.output.position);
    assert_eq!(values.len(), 2101);
    values
}

fn input_values(clock: u32, kind: u32, stage: &str) -> [i32; 4] {
    if stage == "device" {
        if kind == 0 {
            [0; 4]
        } else {
            [
                ((clock * 97 + 12345) & 65535) as i32 - 32768,
                ((clock * 173 + 54321) & 65535) as i32 - 32768,
                0,
                0,
            ]
        }
    } else {
        match kind {
            0 => [
                0,
                0,
                if clock.is_multiple_of(997) { 32767 } else { 0 },
                if clock.is_multiple_of(991) { -32768 } else { 0 },
            ],
            1 => [95000, -95000, 32767, -32768],
            2 => [-786432, 786432, -32768, 32767],
            _ => [
                ((clock * 257) & 65535) as i32 - 32768,
                ((clock * 503) & 65535) as i32 - 32768,
                ((clock * 173 + 12345) & 65535) as i32 - 32768,
                ((clock * 97 + 54321) & 65535) as i32 - 32768,
            ],
        }
    }
}

fn initialize_pattern(device: &mut SpuDevice, kind: u32) {
    if kind == 0 {
        return;
    }
    let mut seed = 0x12345678_u32;
    for address in 0..0x40000 {
        let value = match kind {
            1 => 0x8000,
            2 => 0x7fff,
            _ => {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 16) as u16
            }
        };
        device.engine.memory.write_word(address, value);
    }
}

fn run_case(device: &mut SpuDevice, case: &Value, label: &str) {
    let mut actions: BTreeMap<u32, Vec<&Value>> = BTreeMap::new();
    for row in case["actions"].as_array().unwrap() {
        actions.entry(number(&row[0])).or_default().push(row);
    }
    let checkpoints: BTreeMap<_, _> = case["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (number(&row["clock"]), row))
        .collect();
    let mut reads = Vec::new();
    let mut pcm = Vec::new();
    let mut wet_nonzero = 0;
    let stage = case["stage"].as_str().unwrap();
    for clock in 0..number(&case["clocks"]) {
        if let Some(rows) = actions.get(&clock) {
            for row in rows {
                match row[1].as_str().unwrap() {
                    "write" => device
                        .engine
                        .write_register(number(&row[2]) as u16, number(&row[3]) as u16)
                        .unwrap(),
                    "read" => reads.push([
                        i64::from(clock),
                        i64::from(number(&row[2])),
                        i64::from(device.engine.read_register(number(&row[2]) as u16).unwrap()),
                    ]),
                    "dma_write" => device.engine.dma_write(number(&row[3])),
                    "dma_read" => {
                        reads.push([i64::from(clock), -1, i64::from(device.engine.dma_read())])
                    }
                    _ => panic!("invalid output fixture operation"),
                }
            }
        }
        let input = input_values(clock, number(&case["input_mode"]), stage);
        let frame = if stage == "device" {
            device
                .clock_sample([input[0] as i16, input[1] as i16])
                .unwrap()
        } else {
            device.output.finish_frame(
                &mut device.engine,
                SpuVoiceFrame {
                    dry: [input[0], input[1]],
                    reverb_input: [input[2] as i16, input[3] as i16],
                },
            )
        };
        let frontend = frame
            .stereo
            .map(|sample| ((i32::from(sample) * 3 + 2) >> 2) as i16);
        for value in [
            frame.stereo[0],
            frame.stereo[1],
            frontend[0],
            frontend[1],
            frame.reverb[0],
            frame.reverb[1],
        ] {
            pcm.extend_from_slice(&value.to_le_bytes());
        }
        if frame.reverb != [0; 2] {
            wet_nonzero += 1;
        }
        if let Some(expected) = checkpoints.get(&clock) {
            let state = output_state(device);
            let bytes: Vec<_> = state.iter().flat_map(|value| value.to_le_bytes()).collect();
            assert_eq!(
                sha256_hex(&bytes),
                expected["sha256"].as_str().unwrap(),
                "{label} clock {clock}"
            );
        }
    }
    assert_eq!(
        serde_json::to_value(reads).unwrap(),
        case["reads"],
        "{label} reads"
    );
    assert_eq!(
        sha256_hex(&pcm),
        case["pcm_and_frontend_and_wet_sha256"].as_str().unwrap(),
        "{label} PCM"
    );
    assert_eq!(
        wet_nonzero,
        number(&case["wet_nonzero_frames"]),
        "{label} wet response"
    );
    assert_eq!(
        sha256_hex(&device.engine.memory.to_bytes()),
        case["ram_sha256"].as_str().unwrap(),
        "{label} RAM"
    );
}

#[test]
fn reverb_main_output_and_complete_device_match_software_reference() {
    let fixture = owned();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 83);
    for case in cases {
        let mut device = SpuDevice::default();
        initialize_pattern(
            &mut device,
            case.get("ram_pattern").map(number).unwrap_or(0),
        );
        if let Some(uploads) = case["uploads"].as_array() {
            for upload in uploads {
                let hex = upload["hex"].as_str().unwrap();
                let bytes: Vec<_> = (0..hex.len())
                    .step_by(2)
                    .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
                    .collect();
                device
                    .engine
                    .memory
                    .upload(number(&upload["address"]), &bytes)
                    .unwrap();
            }
        }
        run_case(&mut device, case, case["name"].as_str().unwrap());
    }
}

fn original() -> (PathBuf, Value) {
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("set LSD_GAME_FILES"));
    let path = std::env::var_os("LSD_SPU_OUTPUT_REFERENCE").expect("set LSD_SPU_OUTPUT_REFERENCE");
    (
        root,
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(),
    )
}

#[test]
#[ignore = "requires private original-executable preset and software output observations"]
fn all_original_sdk_presets_match_supplied_long_reverb_traces() {
    let (_, fixture) = original();
    let presets = fixture["presets"].as_array().unwrap();
    assert_eq!(presets.len(), 10);
    for case in presets {
        assert_eq!(number(&case["clocks"]), 220500);
        run_case(
            &mut SpuDevice::default(),
            case,
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
#[ignore = "requires canonical executable and private original SDK preset writes"]
fn imported_preset_values_match_original_sdk_register_writes() {
    let executable = std::env::var_os("LSD_EXECUTABLE").expect("set LSD_EXECUTABLE");
    let reference =
        std::env::var_os("LSD_NATIVE_REVERB_PRESETS").expect("set LSD_NATIVE_REVERB_PRESETS");
    let image = std::fs::read(executable).unwrap();
    let presets = lsd_assets::load_spu_reverb_presets(&image).unwrap();
    let original: Value = serde_json::from_slice(&std::fs::read(reference).unwrap()).unwrap();
    for (mode, preset) in presets.iter().enumerate() {
        let row = &original["presets"][mode];
        assert_eq!(number(&row["mode"]) as usize, mode);
        assert_eq!(
            u32::from(preset.start_address),
            number(&row["reverb_start_register"])
        );
        for (index, value) in preset.registers.iter().enumerate() {
            assert_eq!(u32::from(*value), number(&row["reverb_registers"][index]));
        }
        let writes: Vec<_> = preset
            .register_writes()
            .into_iter()
            .map(|write| [u32::from(write.offset), 2, u32::from(write.value)])
            .collect();
        assert_eq!(serde_json::to_value(writes).unwrap(), row["writes"]);
    }
}

#[test]
#[ignore = "requires original sample banks and private SDK/full-device output observations"]
fn all_original_sdk_tone_bindings_match_supplied_pcm_and_state() {
    let (root, fixture) = original();
    let mut count = 0;
    for bank in fixture["banks"].as_array().unwrap() {
        let path = root.join(bank["path"].as_str().unwrap());
        let header = std::fs::read(&path).unwrap();
        let body = std::fs::read(path.with_extension("VB")).unwrap();
        assert_eq!(sha256_hex(&header), bank["header_sha256"].as_str().unwrap());
        assert_eq!(sha256_hex(&body), bank["body_sha256"].as_str().unwrap());
        for case in bank["tones"].as_array().unwrap() {
            let mut device = SpuDevice::default();
            device
                .engine
                .memory
                .upload(number(&bank["spu_base"]), &body)
                .unwrap();
            run_case(
                &mut device,
                case,
                &format!(
                    "{} program {} tone {}",
                    bank["path"], case["program"], case["tone"]
                ),
            );
            count += 1;
        }
    }
    assert_eq!(count, 351);
}
