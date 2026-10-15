//! Bounded host-output diagnostic, separate from natural-game audio acceptance.
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    audio::AudioPlugin,
    prelude::*,
};
use lsd_bevy::audio::{SceneAudioOutput, SceneAudioPlugin};
use lsd_core::GenerationSound;

#[derive(Resource)]
struct Check {
    started: std::time::Instant,
    seconds: u64,
    handle: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "Usage: lsd-audio-live <extracted-files-root> <packed-note> <seconds1..30>".into(),
        );
    }
    let root = std::path::PathBuf::from(&args[0]);
    let note: i32 = args[1].to_str().ok_or("Invalid note")?.parse()?;
    let seconds: u64 = args[2].to_str().ok_or("Invalid duration")?.parse()?;
    if note < 0 || !(1..=30).contains(&seconds) {
        return Err("Expected nonnegative note and duration1..30".into());
    }
    let executable = std::fs::read(root.join("SLPS_015.56"))?;
    let mut audio = lsd_assets::load_scene_audio(&root, &executable)?;
    let handle = audio.play_note(note, 100, 100);
    if handle < 0 {
        return Err(format!("Sound request failed: {:?}", audio.snapshot()).into());
    }
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        std::time::Duration::from_millis(10),
    )))
    .add_plugins((AssetPlugin::default(), AudioPlugin::default()))
    .add_plugins(SceneAudioPlugin(audio))
    .insert_resource(Check {
        started: std::time::Instant::now(),
        seconds,
        handle,
    })
    .add_systems(Update, finish);
    match app.run() {
        AppExit::Success => Ok(()),
        _ => Err("Host audio diagnostic did not complete successfully".into()),
    }
}

fn finish(
    check: Res<Check>,
    audio: Res<SceneAudioOutput>,
    sinks: Query<&AudioSink>,
    mut exit: MessageWriter<AppExit>,
) {
    let state = audio.0.snapshot();
    let complete = state.sample_frames >= check.seconds * 44100;
    if complete || state.fault.is_some() || check.started.elapsed().as_secs() > check.seconds + 5 {
        let okay = complete
            && state.nonzero_frames > 0
            && state.fault.is_none()
            && sinks.iter().count() == 1;
        println!(
            "host_audio_diagnostic_success={okay} sink_count={} handle={} frames={} nonzero_frames={} sdk_ticks={} faults={:?} original_audio_scene_accepted=false",
            sinks.iter().count(),
            check.handle,
            state.sample_frames,
            state.nonzero_frames,
            state.sdk_ticks,
            state.fault
        );
        exit.write(if okay {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}
