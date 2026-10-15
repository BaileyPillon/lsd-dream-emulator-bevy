//! A single continuous Bevy source for the shared 24-voice SPU.
use bevy::{
    audio::{AddAudioSource, ChannelCount, SampleRate, Source},
    prelude::*,
};
use lsd_assets::SceneAudio;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Asset, TypePath, Clone)]
pub struct SceneSpuSource {
    audio: SceneAudio,
    claimed: Arc<AtomicBool>,
}

impl SceneSpuSource {
    pub fn new(audio: SceneAudio) -> Self {
        Self {
            audio,
            claimed: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct SceneSpuDecoder {
    source: SceneSpuSource,
    claimed: bool,
    ended: bool,
    right: Option<i16>,
}

impl Decodable for SceneSpuSource {
    type Decoder = SceneSpuDecoder;

    fn decoder(&self) -> Self::Decoder {
        let claimed = !self.claimed.swap(true, Ordering::AcqRel);
        SceneSpuDecoder {
            source: self.clone(),
            claimed,
            ended: !claimed,
            right: None,
        }
    }
}

impl Drop for SceneSpuDecoder {
    fn drop(&mut self) {
        if self.claimed {
            self.source.claimed.store(false, Ordering::Release);
        }
    }
}

impl Iterator for SceneSpuDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.ended {
            return None;
        }
        if let Some(right) = self.right.take() {
            return Some(f32::from(right) / 32768.0);
        }
        let mut frame = [[0; 2]];
        if self.source.audio.render(&mut frame).is_err() {
            self.ended = true;
            return None;
        }
        self.right = Some(frame[0][1]);
        Some(f32::from(frame[0][0]) / 32768.0)
    }
}

impl Source for SceneSpuDecoder {
    fn current_span_len(&self) -> Option<usize> {
        self.ended.then_some(0)
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(44100).unwrap()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

#[derive(Resource)]
pub struct SceneAudioOutput(pub SceneAudio);

/// Add after Bevy's AssetPlugin and AudioPlugin. Exactly one nonspatial player
/// retains the device's stereo mix; Bevy performs only host conversion/output.
pub struct SceneAudioPlugin(pub SceneAudio);

impl Plugin for SceneAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SceneAudioOutput(self.0.clone()))
            .add_audio_source::<SceneSpuSource>()
            .add_systems(Startup, start_audio)
            .add_systems(Update, report_audio_fault);
    }
}

fn report_audio_fault(audio: Res<SceneAudioOutput>, mut reported: Local<bool>) {
    if !*reported && let Some(fault) = audio.0.snapshot().fault {
        eprintln!("Scene audio stopped: {fault}");
        *reported = true;
    }
}

fn start_audio(
    audio: Res<SceneAudioOutput>,
    mut assets: ResMut<Assets<SceneSpuSource>>,
    mut commands: Commands,
) {
    commands.spawn(AudioPlayer(
        assets.add(SceneSpuSource::new(audio.0.clone())),
    ));
}
