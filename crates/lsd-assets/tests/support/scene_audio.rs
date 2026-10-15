use lsd_assets::{SoundBankBinding, formats::Vab};
use lsd_core::{SpuDevice, SpuPitchTable};

// Owned metadata already checked against original SDK routines. Sound RAM below
// contains an owned looping waveform, never an original game's sample.
pub fn setup() -> (SoundBankBinding, SpuPitchTable, SpuDevice) {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/audio-clock.json")).unwrap();
    let case = &fixture["sound_objects"][0];
    let decode = |name: &str| {
        let text = case[name].as_str().unwrap();
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect::<Vec<_>>()
    };
    let header = decode("header_hex");
    let body = decode("body_hex");
    let vab = Vab::parse(&header, &body).unwrap();
    let bank = SoundBankBinding::from_vab(&vab, 0, 0x2000).unwrap();
    let pitch = SpuPitchTable {
        values: Box::new(std::array::from_fn(|i| {
            fixture["pitch_table"][i].as_u64().unwrap() as u16
        })),
    };
    let mut device = SpuDevice::default();
    device.engine.memory.upload(0x2000, &body).unwrap();
    for sample in &vab.samples {
        let mut waveform = vec![0x31; sample.data.len()];
        for block in waveform.chunks_exact_mut(16) {
            block[0] = 0;
            block[1] = 7;
        }
        device
            .engine
            .memory
            .upload(0x2000 + sample.body_offset as u32, &waveform)
            .unwrap();
    }
    for (offset, value) in [(0x180, 15480), (0x182, 15480), (0x1ac, 4), (0x1aa, 0xc000)] {
        device.engine.write_register(offset, value).unwrap();
    }
    (bank, pitch, device)
}

pub fn runtime() -> lsd_assets::SceneAudio {
    let (bank, pitch, device) = setup();
    lsd_assets::SceneAudio::new(bank, pitch, device, std::num::NonZeroU32::new(735).unwrap())
}
