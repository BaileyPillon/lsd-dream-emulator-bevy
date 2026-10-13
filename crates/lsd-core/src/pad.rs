/// Default bit table consumed by the original SDK-facing pad object.
pub const PAD_MASKS: [u32; 16] = [
    0x1000, 0x4000, 0x8000, 0x2000, 0x10, 0x40, 0x80, 0x20, 0x200, 0x400, 0x100, 8, 2, 4, 1, 0x800,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PadState {
    pub held: u32,
    pub released: u32,
    pub pressed: u32,
}

impl PadState {
    pub fn poll(&mut self, buttons: u32) {
        let changed = buttons ^ self.held;
        self.released = changed & self.held;
        self.pressed = changed & buttons;
        self.held = buttons;
    }

    /// Original notifications are accumulated ascending, then delivered descending.
    pub fn notifications(&self, masks: &[u32; 16]) -> Vec<i32> {
        (0..16)
            .rev()
            .filter_map(|index| {
                let mask = masks[index];
                let base = if self.released & mask != 0 {
                    0x22
                } else if self.pressed & mask != 0 {
                    0x12
                } else if self.held & mask != 0 {
                    2
                } else {
                    return None;
                };
                Some(base + index as i32)
            })
            .collect()
    }
}
