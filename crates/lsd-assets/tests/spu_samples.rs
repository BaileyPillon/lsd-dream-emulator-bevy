use lsd_assets::{formats::Vab, sha256_hex};
use lsd_core::{SpuAdpcmDecoder, spu_gaussian, spu_pitch_step};
use serde_json::Value;
use std::path::PathBuf;

fn owned() -> Value {
    serde_json::from_str(include_str!("fixtures/spu-samples.json")).unwrap()
}

fn signed(value: &Value) -> i16 {
    value.as_i64().unwrap().try_into().unwrap()
}

fn array<const N: usize>(value: &Value) -> [i16; N] {
    std::array::from_fn(|index| signed(&value[index]))
}

#[test]
fn stored_blocks_match_pinned_software_decoder_with_supplied_history() {
    let fixture = owned();
    let cases = fixture["decoder_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1292);
    for (index, case) in cases.iter().enumerate() {
        let hex = case["block"].as_str().unwrap();
        let bytes: [u8; 16] =
            std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap());
        let mut decoder = SpuAdpcmDecoder {
            history: array(&case["history"]),
        };
        let block = decoder.decode_block(&bytes).unwrap();
        assert_eq!(block.samples, array(&case["samples"]), "case {index}");
        assert_eq!(decoder.history, array(&case["after"]), "case {index}");
        assert_eq!(block.flags, bytes[1]);
        assert_eq!(block.loop_end(), bytes[1] & 1 != 0);
        assert_eq!(block.loop_repeat(), bytes[1] & 2 != 0);
        assert_eq!(block.loop_start(), bytes[1] & 4 != 0);
    }
}

#[test]
fn interpolation_matches_pinned_fir_for_every_fraction() {
    let fixture = owned();
    let gaussian = &fixture["gaussian"];
    let vectors = gaussian["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 64);
    let mut actual = Vec::new();
    for vector in vectors {
        for fraction in 0..=255 {
            actual.extend_from_slice(&spu_gaussian(array(vector), fraction).to_le_bytes());
        }
    }
    assert_eq!(sha256_hex(&actual), gaussian["sha256"].as_str().unwrap());
}

#[test]
fn every_pitch_register_matches_pinned_phase_arithmetic() {
    let fixture = owned();
    let pitch = &fixture["pitch"];
    let mut previous = vec![None];
    previous.extend(
        pitch["previous_outputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| Some(signed(value))),
    );
    let mut actual = Vec::new();
    for output in previous {
        for register in 0..=65535 {
            actual.extend_from_slice(&spu_pitch_step(register, output).to_le_bytes());
        }
    }
    assert_eq!(sha256_hex(&actual), pitch["sha256"].as_str().unwrap());
}

#[test]
#[ignore = "requires local original VH/VB banks and private software-oracle hashes"]
fn all_original_stored_samples_match_pinned_decoder() {
    let path = std::env::var_os("LSD_SPU_SAMPLE_REFERENCE").expect("set LSD_SPU_SAMPLE_REFERENCE");
    let root = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("set LSD_GAME_FILES"));
    let reference: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut total_samples = 0;
    let mut total_blocks = 0;
    for item in reference["banks"].as_array().unwrap() {
        let path = root.join(item["path"].as_str().unwrap());
        let header = std::fs::read(&path).unwrap();
        let body = std::fs::read(path.with_extension("VB")).unwrap();
        assert_eq!(sha256_hex(&header), item["header_sha256"].as_str().unwrap());
        assert_eq!(sha256_hex(&body), item["body_sha256"].as_str().unwrap());
        let bank = Vab::parse(&header, &body).unwrap();
        assert_eq!(
            bank.samples.len(),
            item["samples"].as_array().unwrap().len()
        );
        for (sample, expected) in bank.samples.iter().zip(item["samples"].as_array().unwrap()) {
            assert_eq!(
                u64::from(sample.number),
                expected["number"].as_u64().unwrap()
            );
            assert_eq!(
                sample.body_offset as u64,
                expected["body_offset"].as_u64().unwrap()
            );
            assert_eq!(
                sample.data.len() as u64,
                expected["bytes"].as_u64().unwrap()
            );
            let blocks = sample.decode_adpcm().unwrap();
            assert_eq!(blocks.len() as u64, expected["blocks"].as_u64().unwrap());
            let mut pcm = Vec::new();
            for (block, encoded) in blocks.iter().zip(sample.data.chunks_exact(16)) {
                assert_eq!(block.flags, encoded[1]);
                for value in block.samples {
                    pcm.extend_from_slice(&value.to_le_bytes());
                }
            }
            assert_eq!(
                sha256_hex(&pcm),
                expected["pcm_sha256"].as_str().unwrap(),
                "{} sample {}",
                path.display(),
                sample.number
            );
            if let Some(last) = blocks.last() {
                assert_eq!(
                    [last.samples[27], last.samples[26]],
                    array(&expected["last_history"])
                );
            }
            total_samples += 1;
            total_blocks += blocks.len();
        }
    }
    assert_eq!(total_samples, 255);
    assert_eq!(total_blocks, 126210);
}
