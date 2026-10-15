//! Controlled SEQ/SDK/SPU render; original scene bank choice/timing is separate.
use lsd_assets::{
    AudioImmediateCommand, SeqMusicTrack, SequenceBankBinding, dispatch_audio_commands,
    formats::{Seq, Vab},
    load_sequence_pitch_table, sha256_hex,
};
use lsd_core::{AudioNoteDriver, SpuDevice};
use serde_json::{Value, json};
use std::{io::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(5..=6).contains(&args.len()) {
        return Err("Usage: lsd-music <SLPS_015.56> <bank.VH> <file.SEQ> <seconds1..60> <new-output.wav> [new-trace.json]".into());
    }
    let seconds: u32 = args[3].to_str().ok_or("Invalid duration")?.parse()?;
    if !(1..=60).contains(&seconds) {
        return Err("Duration must be1..60 seconds".into());
    }
    let destination = PathBuf::from(&args[4]);
    let trace_path = args.get(5).map(PathBuf::from);
    if destination.exists()
        || trace_path
            .as_ref()
            .is_some_and(|p| p.exists() || p == &destination)
    {
        return Err("Choose new output paths; existing files are preserved".into());
    }
    let executable = std::fs::read(&args[0])?;
    let bank_path = PathBuf::from(&args[1]);
    let header = std::fs::read(&bank_path)?;
    let body = std::fs::read(bank_path.with_extension(
        if bank_path.extension().is_some_and(|p| p == "vh") {
            "vb"
        } else {
            "VB"
        },
    ))?;
    let sequence = std::fs::read(&args[2])?;
    let vab = Vab::parse(&header, &body)?;
    let mut bank = SequenceBankBinding::from_vab(&vab, 0, 4096)?;
    let pitch = load_sequence_pitch_table(&executable)?;
    let mut music = SeqMusicTrack::new(Seq::parse(&sequence)?, 60, 0)
        .map_err(|e| format!("Sequence initialization: {e:?}"))?;
    music.channels.sequence_volume = [52; 2];
    let mut device = SpuDevice::default();
    device.engine.memory.upload(4096, &body)?;
    let mut events: Vec<Value> = Vec::new();
    for (offset, value) in [(0x180, 15480), (0x182, 15480), (0x1aa, 0xc000)] {
        device.engine.write_register(offset, value)?;
        events.push(json!([0, "write", offset, value]));
    }
    let mut driver = AudioNoteDriver::new(24);
    for write in driver
        .tick_engine(&mut device.engine)
        .map_err(|e| format!("SDK initialization: {e:?}"))?
    {
        events.push(json!([0, "write", write.offset, write.value]));
    }
    let mut commands = Vec::new();
    let frames = seconds * 44100;
    let mut pcm = Vec::with_capacity(frames as usize * 4);
    let mut nonzero = 0;
    let mut peak = 0_i32;
    for sample in 0..frames {
        if sample.is_multiple_of(735) && driver.pool.lock != 1 {
            driver.pool.lock = 1;
            for write in driver
                .tick_engine(&mut device.engine)
                .map_err(|e| format!("SDK callback: {e:?}"))?
            {
                events.push(json!([sample, "write", write.offset, write.value]));
            }
            music
                .tick(100000, &mut bank, &mut driver, &pitch, &mut commands)
                .map_err(|e| format!("Music callback: {e:?}"))?;
            for command in &commands {
                events.push(match command {
                    AudioImmediateCommand::Write(w) => json!([sample, "write", w.offset, w.value]),
                    AudioImmediateCommand::DisableNoise(mask) => json!([sample, "noise", mask, 0]),
                });
            }
            dispatch_audio_commands(&mut device.engine, &mut commands)?;
            driver.pool.lock = 0;
        }
        let stereo = device.clock_sample([0; 2])?.stereo;
        nonzero += u32::from(stereo != [0; 2]);
        for value in stereo {
            peak = peak.max(i32::from(value).abs());
            pcm.extend_from_slice(&value.to_le_bytes());
        }
    }
    let mut wav = Vec::with_capacity(pcm.len() + 44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    for value in [1_u16, 2] {
        wav.extend_from_slice(&value.to_le_bytes());
    }
    for value in [44100_u32, 176400] {
        wav.extend_from_slice(&value.to_le_bytes());
    }
    for value in [4_u16, 16] {
        wav.extend_from_slice(&value.to_le_bytes());
    }
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    wav.extend_from_slice(&pcm);
    let metadata = json!({"frames":frames,"sample_rate":44100,"channels":2,"nonzero_frames":nonzero,"peak":peak,"pcm_sha256":sha256_hex(&pcm),"executable_sha256":sha256_hex(&executable),"header_sha256":sha256_hex(&header),"body_sha256":sha256_hex(&body),"sequence_sha256":sha256_hex(&sequence),"events_dispatched":music.events_dispatched,"notes_allocated":music.notes_allocated,"loops":music.events.loop_count,"supplied_callback_rate":60,"live_dream_music_connected":false,"original_audio_scene_accepted":false,"scope":"One supplied bank/base4096, one infinite-repeat track at volume52, no reverb, supplied735-sample SDK cadence; driver flush precedes event dispatch."});
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?
        .write_all(&wav)?;
    if let Some(path) = trace_path {
        let trace = json!({"metadata":metadata,"spu_base":4096,"events":events});
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(serde_json::to_string(&trace)?.as_bytes())?;
    }
    println!("{}", serde_json::to_string_pretty(&metadata)?);
    Ok(())
}
