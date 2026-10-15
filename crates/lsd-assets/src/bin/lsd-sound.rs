//! Controlled sound-object/SDK/SPU diagnostic, not a natural-game audio replay.
use lsd_assets::{
    AudioImmediateCommand, SoftwareSound, SoundBankBinding, dispatch_audio_commands, formats::Vab,
    load_spu_pitch_table, load_spu_reverb_presets, sha256_hex,
};
use lsd_core::{AudioNoteDriver, AudioRegisterWrite, GenerationSound, SpuDevice};
use serde_json::{Value, json};
use std::{io::Write, path::PathBuf};

fn apply(
    device: &mut SpuDevice,
    events: &mut Vec<Value>,
    clock: u32,
    write: AudioRegisterWrite,
) -> Result<(), Box<dyn std::error::Error>> {
    device.engine.write_register(write.offset, write.value)?;
    events.push(json!([clock, "write", write.offset, write.value]));
    Ok(())
}

fn dispatch(
    device: &mut SpuDevice,
    events: &mut Vec<Value>,
    clock: u32,
    commands: &mut Vec<AudioImmediateCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    for command in commands.iter() {
        events.push(match command {
            AudioImmediateCommand::Write(write) => {
                json!([clock, "write", write.offset, write.value])
            }
            AudioImmediateCommand::DisableNoise(mask) => json!([clock, "noise", mask, 0]),
        });
    }
    dispatch_audio_commands(&mut device.engine, commands)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if !(4..=6).contains(&arguments.len()) {
        return Err("Usage: lsd-sound <SLPS_015.56> <bank.VH> <packed-note> <new-output.wav> [reverb-mode0..9] [new-trace.json]".into());
    }
    let executable_path = PathBuf::from(&arguments[0]);
    let header_path = PathBuf::from(&arguments[1]);
    let note: i32 = arguments[2].to_str().ok_or("Invalid note")?.parse()?;
    let destination = PathBuf::from(&arguments[3]);
    let mode: usize = arguments
        .get(4)
        .map(|value| {
            value
                .to_str()
                .ok_or("Invalid reverb mode")?
                .parse()
                .map_err(|_| "Invalid reverb mode")
        })
        .transpose()?
        .unwrap_or(0);
    let trace_path = arguments.get(5).map(PathBuf::from);
    if note < 0 || mode > 9 {
        return Err("Expected nonnegative packed note and reverb mode0..9".into());
    }
    if destination.exists()
        || trace_path
            .as_ref()
            .is_some_and(|path| path.exists() || path == &destination)
    {
        return Err("Choose new output paths; existing files are preserved".into());
    }
    let executable = std::fs::read(&executable_path)?;
    let header = std::fs::read(&header_path)?;
    let body_path = header_path.with_extension(
        if header_path
            .extension()
            .is_some_and(|extension| extension == "vh")
        {
            "vb"
        } else {
            "VB"
        },
    );
    let body = std::fs::read(&body_path)?;
    let bank = Vab::parse(&header, &body)?;
    let pitch = load_spu_pitch_table(&executable)?;
    let presets = load_spu_reverb_presets(&executable)?;
    if mode != 0 && 0x1000 + body.len() > usize::from(presets[mode].start_address) * 8 {
        return Err(
            "Bank overlaps the supplied reverb workspace at this diagnostic placement".into(),
        );
    }
    let mut binding = SoundBankBinding::from_vab(&bank, 0, 0x1000)?;
    let mut device = SpuDevice::default();
    device.engine.memory.upload(0x1000, &body)?;
    let mut events = Vec::new();
    for write in presets[mode].register_writes() {
        apply(&mut device, &mut events, 0, write)?;
    }
    for (offset, value) in [
        (0x180, 15480),
        (0x182, 15480),
        (0x184, if mode == 0 { 0 } else { 0x4000 }),
        (0x186, if mode == 0 { 0 } else { 0x4000 }),
        (0x1ac, 4),
        (0x1aa, if mode == 0 { 0xc000 } else { 0xc080 }),
    ] {
        apply(
            &mut device,
            &mut events,
            0,
            AudioRegisterWrite { offset, value },
        )?;
    }
    let mut driver = AudioNoteDriver::new(24);
    let mut commands = Vec::new();
    let mut faults = Vec::new();
    let handle = SoftwareSound {
        bank: &mut binding,
        driver: &mut driver,
        pitch: &pitch,
        commands: &mut commands,
        faults: &mut faults,
    }
    .play_note(note, 100, 100);
    if handle < 0 || !faults.is_empty() {
        return Err(format!("Sound request failed: {faults:?}").into());
    }
    dispatch(&mut device, &mut events, 0, &mut commands)?;
    let mut pcm = Vec::with_capacity(88200 * 4);
    let mut nonzero_frames = 0;
    let mut peak = 0_i32;
    for clock in 0..88200_u32 {
        if clock == 44100 {
            SoftwareSound {
                bank: &mut binding,
                driver: &mut driver,
                pitch: &pitch,
                commands: &mut commands,
                faults: &mut faults,
            }
            .stop_note(handle);
            dispatch(&mut device, &mut events, clock, &mut commands)?;
        }
        // Supplied diagnostic cadence, not acceptance of original IRQ/VBlank timing.
        if clock.is_multiple_of(735) {
            let writes = driver
                .tick_engine(&mut device.engine)
                .map_err(|error| format!("SDK tick: {error:?}"))?;
            events.extend(
                writes
                    .into_iter()
                    .map(|write| json!([clock, "write", write.offset, write.value])),
            );
        }
        let samples = device.clock_sample([0; 2])?.stereo;
        if samples != [0; 2] {
            nonzero_frames += 1;
        }
        for sample in samples {
            peak = peak.max(i32::from(sample).abs());
            pcm.extend_from_slice(&sample.to_le_bytes());
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
    let metadata = json!({"output":destination,"frames":88200,"sample_rate":44100,"channels":2,"nonzero_frames":nonzero_frames,"peak":peak,
        "pcm_sha256":sha256_hex(&pcm),"header_sha256":sha256_hex(&header),"body_sha256":sha256_hex(&body),"executable_sha256":sha256_hex(&executable),
        "packed_note":note,"reverb_mode":mode,"schedule":"supplied60Hz SDK ticks, release after1second, render2seconds; not natural game callback acceptance","original_audio_playback_accepted":false});
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)?
        .write_all(&wav)?;
    if let Some(path) = trace_path {
        let trace = json!({"metadata":metadata,"spu_base":4096,"events":events});
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(serde_json::to_string_pretty(&trace)?.as_bytes())?;
    }
    println!("{}", serde_json::to_string_pretty(&metadata)?);
    Ok(())
}
