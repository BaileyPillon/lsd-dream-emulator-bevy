#[path = "support/scene_audio.rs"]
mod support;
use lsd_assets::{GenerationAudioRequests, SceneAudio, SoftwareSound};
use lsd_core::{AudioNoteDriver, GenerationSound};

#[test]
fn scene_channels_share_real_voice_handles_and_stop_the_same_pool() {
    let audio = support::runtime();
    let mut generation = GenerationAudioRequests::default();
    let mut npc = GenerationAudioRequests::default();
    generation.connect(audio.clone()).unwrap();
    npc.connect(audio.clone()).unwrap();
    assert_eq!(generation.play_note(0, 100, 100), 0);
    assert_eq!(npc.play_note(0, 100, 100), 1);
    assert_eq!(generation.stop_note(0), -1);
    assert!(generation.requests.is_empty() && npc.requests.is_empty());
    let state = audio.snapshot();
    assert_eq!(
        (state.allocated_notes, state.stops, state.requests),
        (2, 1, 3)
    );
    assert!(state.fault.is_none());
}

#[test]
fn callbacks_and_stereo_pcm_are_independent_of_host_buffer_boundaries() {
    let mut whole = support::runtime();
    let mut split = support::runtime();
    whole.play_note(0, 100, 100);
    split.play_note(0, 100, 100);
    let mut expected = vec![[0; 2]; 4096];
    whole.render(&mut expected).unwrap();
    let mut actual = vec![[0; 2]; 4096];
    for chunk in actual.chunks_mut(127) {
        split.render(chunk).unwrap();
    }
    assert_eq!(actual, expected);
    assert_eq!(split.snapshot(), whole.snapshot());
    assert_eq!(whole.snapshot().sdk_ticks, 6);
    assert!(whole.snapshot().nonzero_frames > 0);
}

#[test]
fn connected_calls_preserve_shared_pitch_context_and_sdk_device_results() {
    let (mut bank, pitch, mut device) = support::setup();
    let mut runtime = SceneAudio::new(
        bank.clone(),
        pitch.clone(),
        device.clone(),
        std::num::NonZeroU32::new(735).unwrap(),
    );
    let mut other_channel = runtime.clone();
    let mut driver = AudioNoteDriver::new(24);
    let mut commands = Vec::new();
    let mut faults = Vec::new();
    runtime.volume_offset(2);
    other_channel.play_note(0, 100, 100);
    {
        let mut original_adapter = SoftwareSound {
            bank: &mut bank,
            driver: &mut driver,
            pitch: &pitch,
            commands: &mut commands,
            faults: &mut faults,
        };
        original_adapter.volume_offset(2);
        assert_eq!(original_adapter.play_note(0, 100, 100), 0);
    }
    lsd_assets::dispatch_audio_commands(&mut device.engine, &mut commands).unwrap();
    let mut expected = Vec::new();
    for clock in 0u32..1470 {
        if clock.is_multiple_of(735) {
            driver.tick_engine(&mut device.engine).unwrap();
        }
        expected.push(device.clock_sample([0; 2]).unwrap().stereo);
    }
    let mut actual = vec![[0; 2]; 1470];
    runtime.render(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert!(faults.is_empty());
}

#[test]
fn queued_history_cannot_be_attached_as_if_handles_had_been_returned() {
    let mut requests = GenerationAudioRequests::default();
    assert_eq!(requests.play_note(0, 100, 100), -1);
    assert!(requests.connect(support::runtime()).is_err());
    let mut connected = GenerationAudioRequests::default();
    connected.connect(support::runtime()).unwrap();
    assert!(connected.connect(support::runtime()).is_err());
}

#[test]
fn a_device_fault_is_retained_without_advancing_or_resetting_playback() {
    let mut audio = support::runtime();
    assert_eq!(audio.play_note(i32::MAX, 100, 100), -1);
    let fault = audio.snapshot().fault.unwrap();
    assert!(audio.render(&mut [[0; 2]; 128]).is_err());
    assert_eq!(audio.snapshot().sample_frames, 0);
    assert_eq!(audio.snapshot().fault.as_deref(), Some(fault.as_str()));
    assert_eq!(audio.play_note(0, 100, 100), -1);
}

#[test]
fn unrelated_executables_cannot_choose_scene_audio_content() {
    assert!(lsd_assets::load_scene_sound_path(&[]).is_err());
}

#[test]
#[ignore = "requires local original sound bank and the independently checked phase52 diagnostic"]
fn continuous_scene_runtime_preserves_the_independently_checked_original_pcm() {
    let root = std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap());
    let executable = std::fs::read(std::env::var_os("LSD_EXECUTABLE").unwrap()).unwrap();
    assert_eq!(
        lsd_assets::load_scene_sound_path(&executable).unwrap(),
        "SND/SE"
    );
    let header = std::fs::read(root.join("CDI/SND/AMBIENT.VH")).unwrap();
    let body = std::fs::read(root.join("CDI/SND/AMBIENT.VB")).unwrap();
    let vab = lsd_assets::formats::Vab::parse(&header, &body).unwrap();
    let bank = lsd_assets::SoundBankBinding::from_vab(&vab, 0, 4096).unwrap();
    let mut device = lsd_core::SpuDevice::default();
    device.engine.memory.upload(4096, &body).unwrap();
    for write in lsd_assets::load_spu_reverb_presets(&executable).unwrap()[1].register_writes() {
        device
            .engine
            .write_register(write.offset, write.value)
            .unwrap();
    }
    for (offset, value) in [
        (0x180, 15480),
        (0x182, 15480),
        (0x184, 0x4000),
        (0x186, 0x4000),
        (0x1ac, 4),
        (0x1aa, 0xc080),
    ] {
        device.engine.write_register(offset, value).unwrap();
    }
    let mut audio = SceneAudio::new(
        bank,
        lsd_assets::load_spu_pitch_table(&executable).unwrap(),
        device,
        std::num::NonZeroU32::new(735).unwrap(),
    );
    let handle = audio.play_note(0, 100, 100);
    assert_eq!(handle, 0);
    let mut frames = vec![[0; 2]; 88200];
    audio.render(&mut frames[..44100]).unwrap();
    audio.stop_note(handle);
    audio.render(&mut frames[44100..]).unwrap();
    let pcm: Vec<u8> = frames
        .into_iter()
        .flatten()
        .flat_map(i16::to_le_bytes)
        .collect();
    assert_eq!(
        lsd_assets::sha256_hex(&pcm),
        "1b3793947f1d1574e70219309efb0146f635a1b3bd143cf8de693ec15102b6d3"
    );
    assert_eq!(audio.snapshot().nonzero_frames, 87091);
    assert_eq!(audio.snapshot().sdk_ticks, 120);
}
