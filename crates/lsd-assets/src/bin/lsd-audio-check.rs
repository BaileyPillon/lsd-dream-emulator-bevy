use lsd_assets::formats::{Seq, Vab};
use lsd_assets::sha256_hex;
use serde_json::json;
use std::path::{Path, PathBuf};

fn walk(path: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        // Do not follow directory symlinks or reparse an unrelated library.
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), files)?;
        } else if kind.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: lsd-audio-check <extracted-game-files-directory>".into());
    }
    let root = PathBuf::from(&args[0]);
    let mut files = Vec::new();
    walk(&root, &mut files)?;
    files.sort();
    let mut banks = Vec::new();
    let mut sequences = Vec::new();
    for path in files {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !extension.eq_ignore_ascii_case("VH") && !extension.eq_ignore_ascii_case("SEQ") {
            continue;
        }
        let name = path
            .strip_prefix(&root)?
            .to_string_lossy()
            .replace('\\', "/");
        let data = std::fs::read(&path)?;
        if extension.eq_ignore_ascii_case("VH") {
            let body_path = path.with_extension(if extension == "vh" { "vb" } else { "VB" });
            let body = std::fs::read(body_path)?;
            let bank = Vab::parse(&data, &body).map_err(|e| format!("{name}: {e}"))?;
            let mut decoded_pcm = Vec::new();
            let mut decoded_blocks = 0;
            for sample in &bank.samples {
                let blocks = sample
                    .decode_adpcm()
                    .map_err(|error| format!("{name} sample {}: {error}", sample.number))?;
                decoded_blocks += blocks.len();
                for block in blocks {
                    for value in block.samples {
                        decoded_pcm.extend_from_slice(&value.to_le_bytes());
                    }
                }
            }
            banks.push(json!({"path":name,"header_sha256":sha256_hex(&data),"body_sha256":sha256_hex(&body),
                "programs":bank.header.program_count,"tones":bank.header.tone_count,"samples":bank.header.sample_count,
                "body_bytes":body.len(),"body_prefix_bytes":bank.body_prefix.len(),
                "decoded_stored_blocks":decoded_blocks,"decoded_pcm_values":decoded_pcm.len()/2,
                "decoded_pcm_sha256":sha256_hex(&decoded_pcm)}));
        } else {
            let seq = Seq::parse(&data).map_err(|e| format!("{name}: {e}"))?;
            sequences.push(json!({"path":name,"sha256":sha256_hex(&data),"bytes":data.len(),"events":seq.events.len(),
                "resolution":seq.header.resolution,"tempo_microseconds":seq.header.tempo_microseconds,
                "original_initial_tempo_bpm":seq.header.initial_tempo_bpm(),
                "timing_issue":if seq.header.tempo_microseconds==0 {Some("zero_tempo_original_initialization_fault")} else {None}}));
        }
    }
    if banks.is_empty() || sequences.is_empty() {
        return Err("No complete audio corpus found: expected VH/VB banks and SEQ files".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"banks":banks,"sequences":sequences,"audio_playback_verified":false})
        )?
    );
    Ok(())
}
