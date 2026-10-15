use lsd_assets::{SceneAudio, SoundBankBinding, formats::Vab};
use lsd_core::{AudioNoteDriver, AudioVolumeContext, SpuDevice, SpuPitchTable};
use serde_json::Value;

pub fn decode(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
pub fn runtime(fixture: &Value, header: &[u8], body: &[u8]) -> SceneAudio {
    let bank = Vab::parse(header, body).unwrap();
    let binding = SoundBankBinding::from_vab(&bank, 0, 4096).unwrap();
    let pitch = SpuPitchTable {
        values: Box::new(std::array::from_fn(|i| {
            fixture["pitch_table"][i].as_u64().unwrap() as u16
        })),
    };
    SceneAudio::new(
        binding,
        pitch,
        SpuDevice::default(),
        std::num::NonZeroU32::new(735).unwrap(),
    )
}
pub fn owned() -> SceneAudio {
    let fixture: Value = serde_json::from_str(include_str!("../fixtures/link-audio.json")).unwrap();
    runtime(
        &fixture,
        &decode(fixture["header_hex"].as_str().unwrap()),
        &decode(fixture["body_hex"].as_str().unwrap()),
    )
}
pub fn driver(row: &Value) -> AudioNoteDriver {
    let n = |v: &Value| v.as_u64().unwrap() as u32;
    let mut state = AudioNoteDriver::new(n(&row["capacity"]) as i32);
    for (slot, raw) in state
        .pool
        .slots
        .iter_mut()
        .zip(row["slots"].as_array().unwrap())
    {
        slot.record = decode(raw.as_str().unwrap()).try_into().unwrap();
    }
    state.pool.lock = n(&row["lock"]) as i32;
    state.pool.current = n(&row["current"]) as u16;
    state.pool.key_on = n(&row["key_on"]);
    state.pool.key_off = n(&row["key_off"]);
    state.registers =
        std::array::from_fn(|i| std::array::from_fn(|r| n(&row["registers"][i][r]) as u16));
    state.dirty = std::array::from_fn(|i| n(&row["dirty"][i]) as u8);
    state.automation = std::array::from_fn(|i| n(&row["automation"][i]));
    state.reverb = n(&row["reverb"]);
    state.history_index = n(&row["history_index"]);
    state.reclaim_disabled = n(&row["reclaim_disabled"]) as u8;
    let c = &row["volume_context"];
    state.volume_context = AudioVolumeContext {
        bank_volume: n(&c[0]) as u8,
        program_volume: n(&c[1]) as u8,
        tone_volume: n(&c[2]) as u8,
        tone_pan: n(&c[3]) as u8,
        program_pan: n(&c[4]) as u8,
        request_pan: n(&c[5]) as u8,
        request_volume: n(&c[6]) as u8,
    };
    state
}
