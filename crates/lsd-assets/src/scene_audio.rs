//! One shared sound object, SDK driver and SPU for synchronous scene calls.
use crate::{AudioImmediateCommand, SoftwareSound, SoundBankBinding, SoundBankFault};
use lsd_core::{AudioNoteDriver, GenerationSound, SpuDevice, SpuPitchTable};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneAudioSnapshot {
    pub startup_sdk_ticks: u32,
    pub sample_frames: u64,
    pub sdk_ticks: u64,
    pub requests: u64,
    pub allocated_notes: u64,
    pub stops: u64,
    pub nonzero_frames: u64,
    pub fault: Option<String>,
}

#[derive(Debug)]
struct State {
    bank: SoundBankBinding,
    driver: AudioNoteDriver,
    pitch: SpuPitchTable,
    device: SpuDevice,
    commands: Vec<AudioImmediateCommand>,
    note_faults: Vec<SoundBankFault>,
    sdk_period: std::num::NonZeroU32,
    snapshot: SceneAudioSnapshot,
}

/// Clones refer to the SAME sound object and device. In particular, allocation,
/// pitch offset, volume context, sample RAM and reverb are not per scene channel.
#[derive(Debug, Clone)]
pub struct SceneAudio(Arc<Mutex<State>>);

impl SceneAudio {
    /// The caller supplies bank placement, initialized registers and SDK cadence.
    /// This constructor does not establish original startup/interrupt timing.
    pub fn new(
        bank: SoundBankBinding,
        pitch: SpuPitchTable,
        mut device: SpuDevice,
        sdk_period: std::num::NonZeroU32,
    ) -> Self {
        let mut driver = AudioNoteDriver::new(24);
        let mut snapshot = SceneAudioSnapshot::default();
        // Original func_8002EDD4 performs one SDK update during initialization.
        // Keep its history/reclaim state before the first scene note request.
        match driver.tick_engine(&mut device.engine) {
            Ok(_) => snapshot.startup_sdk_ticks = 1,
            Err(error) => snapshot.fault = Some(format!("SDK initialization: {error:?}")),
        }
        Self(Arc::new(Mutex::new(State {
            bank,
            driver,
            pitch,
            device,
            commands: Vec::new(),
            note_faults: Vec::new(),
            sdk_period,
            snapshot,
        })))
    }

    pub fn snapshot(&self) -> SceneAudioSnapshot {
        match self.0.lock() {
            Ok(state) => state.snapshot.clone(),
            Err(_) => SceneAudioSnapshot {
                fault: Some("Scene audio mutex poisoned".into()),
                ..Default::default()
            },
        }
    }

    /// Explicit diagnostic copy. Ordinary host health checks use the smaller
    /// counters above; taking this snapshot does not advance a device clock.
    pub fn driver_snapshot(&self) -> Result<AudioNoteDriver, String> {
        self.0
            .lock()
            .map(|state| state.driver.clone())
            .map_err(|_| "Scene audio mutex poisoned".into())
    }

    /// Render consecutive raw 44.1 kHz stereo frames while holding one device
    /// lock. A fault is retained and stops subsequent playback rather than
    /// resetting the device or substituting a different synthesizer.
    pub fn render(&self, frames: &mut [[i16; 2]]) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "Scene audio mutex poisoned")?;
        if let Some(error) = &state.snapshot.fault {
            return Err(error.clone());
        }
        for frame in frames {
            if state
                .snapshot
                .sample_frames
                .is_multiple_of(u64::from(state.sdk_period.get()))
            {
                let State { driver, device, .. } = &mut *state;
                if let Err(error) = driver.tick_engine(&mut device.engine) {
                    let error = format!("SDK callback: {error:?}");
                    state.snapshot.fault = Some(error.clone());
                    return Err(error);
                }
                state.snapshot.sdk_ticks += 1;
            }
            match state.device.clock_sample([0; 2]) {
                Ok(pcm) => {
                    *frame = pcm.stereo;
                    state.snapshot.sample_frames += 1;
                    state.snapshot.nonzero_frames += u64::from(*frame != [0; 2]);
                }
                Err(error) => {
                    let error = error.to_string();
                    state.snapshot.fault = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn request(&self, action: impl FnOnce(&mut SoftwareSound<'_>) -> i32) -> i32 {
        let Ok(mut state) = self.0.lock() else {
            return -1;
        };
        if state.snapshot.fault.is_some() {
            return -1;
        }
        state.snapshot.requests += 1;
        let State {
            bank,
            driver,
            pitch,
            commands,
            note_faults,
            ..
        } = &mut *state;
        let result = action(&mut SoftwareSound {
            bank,
            driver,
            pitch,
            commands,
            faults: note_faults,
        });
        if let Some(error) = state.note_faults.first() {
            state.snapshot.fault = Some(format!("Sound object: {error:?}"));
        } else {
            let State {
                device, commands, ..
            } = &mut *state;
            if let Err(error) = crate::dispatch_audio_commands(&mut device.engine, commands) {
                state.snapshot.fault = Some(error.to_string());
            }
        }
        result
    }
}

impl GenerationSound for SceneAudio {
    fn play_note(&mut self, note: i32, first: i32, target: i32) -> i32 {
        let handle = self.request(|sound| sound.play_note(note, first, target));
        if handle >= 0
            && let Ok(mut state) = self.0.lock()
        {
            state.snapshot.allocated_notes += 1;
        }
        handle
    }

    fn stop_note(&mut self, handle: i32) -> i32 {
        let result = self.request(|sound| sound.stop_note(handle));
        if let Ok(mut state) = self.0.lock() {
            state.snapshot.stops += 1;
        }
        result
    }

    fn volume_offset(&mut self, offset: i32) {
        self.request(|sound| {
            sound.volume_offset(offset);
            -1
        });
    }
}

/// Supplied placement/cadence for the live prototype. SE is the original dream
/// session's shared sound bank; music bank selection/SEQ playback is separate.
pub fn load_scene_audio(
    root: &std::path::Path,
    executable: &[u8],
) -> Result<SceneAudio, Box<dyn std::error::Error>> {
    load_scene_audio_from_source(&crate::AssetSource::original(root), executable)
}

/// Shared prototype sound bank, resolved through the active content profile.
pub fn load_scene_audio_from_source(
    root: &crate::AssetSource,
    executable: &[u8],
) -> Result<SceneAudio, Box<dyn std::error::Error>> {
    let path = crate::load_scene_sound_path(executable)?;
    let header = root.read(std::path::Path::new("CDI").join(&path).with_extension("VH"))?;
    let body = root.read(std::path::Path::new("CDI").join(path).with_extension("VB"))?;
    let bank = crate::formats::Vab::parse(&header, &body)?;
    let binding = SoundBankBinding::from_vab(&bank, 0, 0x1000)?;
    let mut device = SpuDevice::default();
    device.engine.memory.upload(0x1000, &body)?;
    for write in crate::load_spu_reverb_presets(executable)?[0].register_writes() {
        device.engine.write_register(write.offset, write.value)?;
    }
    for (offset, value) in [(0x180, 15480), (0x182, 15480), (0x1ac, 4), (0x1aa, 0xc000)] {
        device.engine.write_register(offset, value)?;
    }
    Ok(SceneAudio::new(
        binding,
        crate::load_spu_pitch_table(executable)?,
        device,
        std::num::NonZeroU32::new(735).unwrap(),
    ))
}
