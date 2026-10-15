use lsd_core::SpuVoiceEngine;

pub fn snapshot(engine: &SpuVoiceEngine) -> Vec<u32> {
    let mut values = vec![
        u32::from(engine.control),
        engine.noise.divider,
        engine.noise.counter,
        u32::from(engine.noise.level),
        engine.pitch_modulation,
        engine.noise_mode,
        engine.reverb_mode,
        engine.reverb_base,
        engine.reverb_current,
        engine.cd_volume[0] as i32 as u32,
        engine.cd_volume[1] as i32 as u32,
        engine.external_volume[0] as i32 as u32,
        engine.external_volume[1] as i32 as u32,
        engine.reverb_volume[0] as i32 as u32,
        engine.reverb_volume[1] as i32 as u32,
        engine.irq_address,
        engine.transfer_address,
        engine.key_on,
        engine.key_off,
        engine.block_end,
        engine.capture_address,
        u32::from(engine.irq_asserted),
    ];
    for volume in engine.main_volume {
        values.extend([
            u32::from(volume.control),
            u32::from(volume.level),
            volume.divider,
        ]);
    }
    values.extend(engine.registers.map(u32::from));
    values.extend(engine.auxiliary.map(u32::from));
    for voice in &engine.voices {
        values.extend(voice.decoded.map(|value| u32::from(value as u16)));
        values.extend(voice.decoder.history.map(|value| u32::from(value as u16)));
        values.extend([
            voice.play_delay,
            voice.write_position,
            voice.read_position,
            voice.available,
            u32::from(voice.ignore_loop_start),
            u32::from(voice.header & 15),
            u32::from(voice.header >> 4),
            u32::from(voice.flags),
        ]);
        for volume in voice.volume {
            values.extend([
                u32::from(volume.control),
                u32::from(volume.level),
                volume.divider,
            ]);
        }
        values.extend([
            u32::from(voice.pitch),
            voice.phase,
            voice.start_address,
            voice.current_address,
            u32::from(voice.envelope.control[0]) | u32::from(voice.envelope.control[1]) << 16,
            voice.loop_address,
            voice.output as u32,
            u32::from(voice.envelope.level),
            voice.envelope.divider,
            voice.envelope.phase as u32,
            u32::from(voice.sustain_target),
        ]);
    }
    assert_eq!(values.len(), 1716);
    values
}
