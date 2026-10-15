use crate::AudioNoteDriver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioRegisterWrite {
    pub offset: u16,
    pub value: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpuNoiseMode {
    pub registers: [u16; 2],
}

impl SpuNoiseMode {
    /// SpuSetNoiseVoice updates NON0/NON1, not the key-off registers.
    pub fn set(&mut self, mode: i32, mask: u32) -> (u32, Vec<AudioRegisterWrite>) {
        let mut writes = Vec::new();
        let mask = mask & 0xffffff;
        match mode {
            0 => {
                self.registers[0] &= !(mask as u16);
                self.registers[1] &= !((mask >> 16) as u16);
            }
            1 => {
                self.registers[0] |= mask as u16;
                self.registers[1] |= (mask >> 16) as u16;
            }
            _ => {
                return (
                    (u32::from(self.registers[0]) | u32::from(self.registers[1]) << 16) & 0xffffff,
                    writes,
                );
            }
        }
        for (offset, value) in [0x194, 0x196].into_iter().zip(self.registers) {
            writes.push(AudioRegisterWrite { offset, value });
        }
        (
            (u32::from(self.registers[0]) | u32::from(self.registers[1]) << 16) & 0xffffff,
            writes,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTickFault {
    PanAutomationPending,
    Volume(crate::AudioVolumeFault),
}

impl AudioNoteDriver {
    /// Original sound-effect driver update with identified device envelope
    /// reads. Pan automation requires its separate SDK implementation.
    pub fn tick(
        &mut self,
        levels: [u16; 24],
        noise_registers: [u16; 2],
    ) -> Result<Vec<AudioRegisterWrite>, AudioTickFault> {
        if self.pool.slots.iter().any(|slot| slot.word(40) != 0) {
            return Err(AudioTickFault::PanAutomationPending);
        }
        let mut writes = Vec::new();
        let mut noise = SpuNoiseMode {
            registers: noise_registers,
        };
        self.history_index = self.history_index.wrapping_add(1) & 15;
        self.automation[self.history_index as usize] = 0;
        for (index, level) in levels
            .iter()
            .enumerate()
            .take(usize::from(self.pool.capacity()))
        {
            self.pool.slots[index].set_word(6, *level);
            if *level == 0 {
                self.automation[self.history_index as usize] |= 1_u32 << index;
            }
        }
        if self.reclaim_disabled == 0 {
            let idle = self.automation[..15]
                .iter()
                .fold(u32::MAX, |value, mask| value & mask);
            for index in 0..usize::from(self.pool.capacity()) {
                if idle & (1_u32 << index) != 0 {
                    if self.pool.slots[index].mode() == 2 {
                        writes.extend(noise.set(0, 0xffffff).1);
                    }
                    self.pool.slots[index].record[27] = 0;
                }
            }
        }
        self.pool.key_on &= !self.pool.key_off;
        for index in 0..24 {
            if self.pool.slots[index].word(28) != 0 {
                self.advance_volume(index).map_err(AudioTickFault::Volume)?;
            }
        }
        for index in 0..24 {
            let dirty = self.dirty[index];
            let base = (index * 16) as u16;
            let mut write = |word: usize| {
                writes.push(AudioRegisterWrite {
                    offset: base + (word * 2) as u16,
                    value: self.registers[index][word],
                })
            };
            if dirty & 1 != 0 {
                write(0);
                write(1);
            }
            if dirty & 4 != 0 {
                write(2);
            }
            if dirty & 8 != 0 {
                write(3);
            }
            if dirty & 16 != 0 {
                write(4);
                write(5);
            }
            self.dirty[index] = 0;
        }
        for (offset, value) in [
            (0x18c, self.pool.key_off as u16),
            (0x18e, (self.pool.key_off >> 16) as u16),
            (0x188, self.pool.key_on as u16),
            (0x18a, (self.pool.key_on >> 16) as u16),
            (0x198, self.reverb as u16),
            (0x19a, (self.reverb >> 16) as u16),
        ] {
            writes.push(AudioRegisterWrite { offset, value });
        }
        self.pool.key_on = 0;
        self.pool.key_off = 0;
        Ok(writes)
    }

    pub fn stop_all(&mut self) -> Vec<AudioRegisterWrite> {
        let mut writes = Vec::new();
        for voice in 0..usize::from(self.pool.capacity()) {
            let slot = &mut self.pool.slots[voice];
            slot.set_word(2, 24);
            slot.set_word(0, 0);
            slot.record[27] = 0;
            slot.set_word(4, 0);
            slot.set_word(6, 0);
            slot.set_word(14, 255);
            slot.set_word(16, 0);
            slot.set_word(18, 0);
            slot.set_word(20, 255);
            let base = (voice * 16) as u16;
            for (offset, value) in [
                (6, 0x200),
                (4, 0x1000),
                (8, 0x80ff),
                (0, 0),
                (2, 0),
                (10, 0x4000),
            ] {
                writes.push(AudioRegisterWrite {
                    offset: base + offset,
                    value,
                });
            }
            self.pool.current = voice as u16;
            self.pool.key_off |= 1_u32 << voice;
            self.pool.key_on &= !self.pool.key_off;
        }
        writes
    }
}
