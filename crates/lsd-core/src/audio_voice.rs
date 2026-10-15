//! Original SDK software voice selection, release masks and pitch conversion.
//! These operations do not synthesize sound or advance an SPU device clock.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioVoiceSlot {
    /// Preserved SDK record; fields not touched by an operation stay unchanged.
    pub record: [u8; 52],
}

impl Default for AudioVoiceSlot {
    fn default() -> Self {
        Self { record: [0; 52] }
    }
}

impl AudioVoiceSlot {
    fn word(&self, at: usize) -> u16 {
        u16::from_le_bytes([self.record[at], self.record[at + 1]])
    }
    fn set_word(&mut self, at: usize, value: u16) {
        self.record[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
    pub fn age(&self) -> u16 {
        self.word(2)
    }
    pub fn level(&self) -> u16 {
        self.word(6)
    }
    pub fn priority(&self) -> i16 {
        self.word(24) as i16
    }
    pub fn mode(&self) -> u8 {
        self.record[27]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceAllocation {
    /// Raw SDK result. Values below pool capacity identify a physical voice.
    pub index: u8,
    /// The original mode-two reclaim path requests immediate key-off of all 24.
    pub immediate_key_off: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioVoicePool {
    capacity: u8,
    pub slots: [AudioVoiceSlot; 24],
    pub lock: i32,
    pub current: u16,
    pub key_off: u32,
    pub key_on: u32,
}

impl AudioVoicePool {
    /// Software state after original startup with zero-initialized driver data.
    /// Original hardware register initialization is a separate operation.
    pub fn new(requested: i32) -> Self {
        let capacity = (requested as u8).min(24);
        let mut pool = Self {
            capacity,
            slots: std::array::from_fn(|_| AudioVoiceSlot::default()),
            lock: 0,
            current: u16::from(capacity.saturating_sub(1)),
            key_off: 0,
            key_on: 0,
        };
        for slot in &mut pool.slots[..usize::from(capacity)] {
            slot.set_word(2, 24);
            slot.record[10] = 64;
            slot.set_word(14, u16::MAX);
            slot.set_word(20, 255);
        }
        pool
    }

    pub const fn capacity(&self) -> u8 {
        self.capacity
    }

    /// Execute the original allocator's scan and age updates. Selection alone
    /// does not set up a note, mark a voice playing or prove playback succeeded.
    pub fn allocate(&mut self, priority: u8) -> VoiceAllocation {
        let mut selected = 99_u8;
        let mut candidate = 99_u8;
        let mut candidate_count = 0;
        let mut candidate_level = u16::MAX;
        let mut candidate_age = 0_i32;
        let mut threshold = i32::from(priority);
        for index in 0..self.capacity {
            let slot = &self.slots[usize::from(index)];
            if slot.mode() == 0 && slot.level() == 0 {
                selected = index;
                break;
            }
            let slot_priority = i32::from(slot.priority());
            // The SDK compares a signed stored priority with an unsigned
            // halfword threshold, including after a negative value was chosen.
            let comparison = i32::from(threshold as u16);
            if slot_priority < comparison {
                threshold = slot_priority;
                candidate = index;
                candidate_level = slot.level();
                candidate_age = i32::from(slot.age());
                candidate_count = 1;
            } else if slot_priority == comparison {
                candidate_count += 1;
                if slot.level() < candidate_level {
                    candidate_level = slot.level();
                    candidate_age = i32::from(slot.age());
                    candidate = index;
                } else if slot.level() == candidate_level {
                    let age = i32::from(slot.age() as i16);
                    if candidate_age < age {
                        candidate_age = age;
                        candidate = index;
                    }
                }
            }
        }
        if selected == 99 {
            selected = if candidate_count == 0 {
                self.capacity
            } else {
                candidate
            };
        }
        let mut immediate_key_off = None;
        if selected < self.capacity {
            for slot in &mut self.slots[..usize::from(self.capacity)] {
                slot.set_word(2, slot.age().wrapping_add(1));
            }
            let slot = &mut self.slots[usize::from(selected)];
            slot.set_word(2, 0);
            slot.set_word(24, u16::from(priority));
            if slot.mode() == 2 {
                immediate_key_off = Some(0x00ff_ffff);
            }
        }
        VoiceAllocation {
            index: selected,
            immediate_key_off,
        }
    }

    /// Original single-voice release, including the global reentrancy lock.
    pub fn release(&mut self, handle: i32) -> i32 {
        if self.lock == 1 {
            return -1;
        }
        self.lock = 1;
        let index = handle as u16;
        if index >= 24 {
            self.lock = 0;
            return -1;
        }
        self.current = index;
        let slot = &mut self.slots[usize::from(index)];
        slot.record[27] = 0;
        slot.set_word(4, 0);
        slot.set_word(0, 0);
        self.key_off |= 1_u32 << index;
        // The routine clears every already-pending key-off bit, not only index.
        self.key_on &= !self.key_off;
        self.lock = 0;
        0
    }
}

impl Default for AudioVoicePool {
    fn default() -> Self {
        Self::new(24)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpuPitchFault {
    pub table_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuPitchTable {
    pub values: Box<[u16; 192]>,
}

impl SpuPitchTable {
    /// Original SDK note conversion, including halfword truncation and MIPS
    /// variable-shift widths. Out-of-table inputs are reported explicitly.
    pub fn pitch(
        &self,
        note: i32,
        fine: i32,
        center: u8,
        correction: u8,
    ) -> Result<u16, SpuPitchFault> {
        let steps = (i32::from(fine as u16) + i32::from(correction)) / 8;
        let carry = i32::from(steps >= 16);
        let fraction = steps - carry * 16;
        let adjusted = note
            .wrapping_add(60)
            .wrapping_sub(i32::from(center))
            .wrapping_add(carry) as i16;
        let octave = (adjusted / 12 - 5) as i32;
        let index = i32::from(adjusted % 12) * 16 + fraction;
        let value = usize::try_from(index)
            .ok()
            .and_then(|index| self.values.get(index))
            .ok_or(SpuPitchFault { table_index: index })?;
        let value = if octave > 0 {
            u32::from(*value).wrapping_shl(octave as u32)
        } else {
            u32::from(*value).wrapping_shr((-octave) as u32)
        };
        Ok(value as u16)
    }
}
