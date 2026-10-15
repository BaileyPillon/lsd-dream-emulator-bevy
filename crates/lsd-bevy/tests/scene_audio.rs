#![cfg(feature = "audio")]
#[path = "../../lsd-assets/tests/support/scene_audio.rs"]
mod support;
use bevy::{audio::Source, prelude::Decodable};
use lsd_bevy::audio::SceneSpuSource;
use lsd_core::GenerationSound;

#[test]
fn continuous_source_preserves_pcm_stereo_order_and_clock_rate() {
    let mut audio = support::runtime();
    let mut expected_audio = support::runtime();
    audio.play_note(0, 100, 100);
    expected_audio.play_note(0, 100, 100);
    let source = SceneSpuSource::new(audio.clone());
    let mut decoder = source.decoder();
    assert_eq!(decoder.channels().get(), 2);
    assert_eq!(decoder.sample_rate().get(), 44100);
    assert_eq!(decoder.total_duration(), None);
    assert_eq!(decoder.current_span_len(), None);
    let mut pcm = vec![[0; 2]; 1025];
    expected_audio.render(&mut pcm).unwrap();
    let expected: Vec<_> = pcm
        .into_iter()
        .flatten()
        .map(|v| f32::from(v) / 32768.0)
        .collect();
    assert_eq!(
        decoder.by_ref().take(expected.len()).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(audio.snapshot(), expected_audio.snapshot());
}

#[test]
fn a_second_consumer_cannot_clock_the_same_device_twice() {
    let audio = support::runtime();
    let source = SceneSpuSource::new(audio.clone());
    let mut first = source.decoder();
    let mut duplicate = source.clone().decoder();
    assert_eq!(duplicate.current_span_len(), Some(0));
    assert_eq!(duplicate.next(), None);
    drop(duplicate);
    assert_eq!(source.decoder().next(), None);
    assert!(first.next().is_some());
    assert_eq!(audio.snapshot().sample_frames, 1);
    drop(first);
    let mut resumed = source.decoder();
    assert!(resumed.next().is_some());
    assert_eq!(audio.snapshot().sample_frames, 2);
}

#[test]
fn source_ends_on_a_retained_device_fault_without_synthesizing_silence() {
    let mut audio = support::runtime();
    audio.play_note(i32::MAX, 100, 100);
    let mut decoder = SceneSpuSource::new(audio).decoder();
    assert_eq!(decoder.next(), None);
    assert_eq!(decoder.next(), None);
    assert_eq!(decoder.current_span_len(), Some(0));
}
