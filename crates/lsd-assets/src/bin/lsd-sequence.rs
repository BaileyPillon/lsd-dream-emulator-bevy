//! Inspect sequence callback/event timing; SDK channel synthesis is separate.
use lsd_assets::{
    SeqEventClock,
    formats::{Seq, SeqEventKind},
    sha256_hex,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err(
            "Usage: lsd-sequence <file.SEQ> <callbacks1..1000000> [signed16-loop-limit;0=infinite]"
                .into(),
        );
    }
    let path = std::path::PathBuf::from(&args[0]);
    let callbacks: u32 = args[1].to_str().ok_or("Invalid callback count")?.parse()?;
    if !(1..=1_000_000).contains(&callbacks) {
        return Err("Callback count must be1..1000000".into());
    }
    let limit = args
        .get(2)
        .map(|v| {
            v.to_str()
                .ok_or("Invalid loop limit")?
                .parse::<i16>()
                .map_err(|_| "Loop limit must fit a signed16-bit value")
        })
        .transpose()?
        .unwrap_or(0);
    let bytes = std::fs::read(&path)?;
    let seq = Seq::parse(&bytes)?;
    let mut player = SeqEventClock::new(seq, 60, limit)
        .map_err(|e| format!("Sequence initialization: {e:?}"))?;
    let mut visited = 0_u64;
    let mut note_on = 0_u64;
    let mut zero_velocity = 0_u64;
    let mut elapsed = 0;
    for _ in 0..callbacks {
        if player.stopped {
            break;
        }
        player
            .tick(1_000_000, |event| {
                visited += 1;
                if let SeqEventKind::Channel { status, data, .. } = event.kind
                    && status >> 4 == 9
                {
                    if data[1] == 0 {
                        zero_velocity += 1;
                    } else {
                        note_on += 1;
                    }
                }
            })
            .map_err(|e| format!("Sequence callback: {e:?}"))?;
        elapsed += 1;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"input_sha256":sha256_hex(&bytes),"supplied_callback_rate":60,"callbacks":elapsed,"events_visited":visited,"note_on_events":note_on,"zero_velocity_note_events":zero_velocity,"loops":player.loop_count,"stopped":player.stopped,"cursor":player.byte_cursor(),"clock":{"countdown":player.clock.countdown,"step":player.clock.step,"initial_step":player.clock.initial_step,"remaining":player.clock.remaining},"audio_playback_implemented":false,"original_audio_scene_accepted":false})
        )?
    );
    Ok(())
}
