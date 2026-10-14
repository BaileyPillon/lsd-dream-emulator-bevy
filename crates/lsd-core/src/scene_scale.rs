use crate::{GenerationFault, transform_number};

/// Five frac_t packets: positive/negative, slow/fast, then absolute reset.
/// These are loaded from the identified executable, never bundled game data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneScaleTables {
    pub packets: [[u32; 3]; 5],
}

/// The scene owns seven arrays of 410 persistent notify nodes. Their scales
/// start alike and every scale operation visits all of them, even empty slots.
/// One value therefore represents their common state across stream rebindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneScale {
    pub remaining: i32,
    pub selected: Option<usize>,
    pub scale: [i32; 3],
    pub tables: SceneScaleTables,
}

impl SceneScale {
    pub fn new(tables: SceneScaleTables) -> Self {
        Self {
            remaining: 0,
            selected: None,
            scale: [4096; 3],
            tables,
        }
    }

    /// func_8004CFB8 replaces the request without restoring existing node scales.
    pub fn request(&mut self, amount: i32, mode: i32) {
        let index = usize::from(amount <= 0) * 2 + usize::from(mode != 0);
        self.selected = Some(index);
        let count = i32::from((self.tables.packets[index][1] >> 16) as i16);
        self.remaining = count.wrapping_mul(amount.wrapping_abs());
    }

    fn apply(&mut self, index: usize, absolute: bool) -> Result<(), GenerationFault> {
        let mut values = [0; 3];
        for (value, word) in values.iter_mut().zip(self.tables.packets[index]) {
            *value =
                i32::from(transform_number(word).ok_or(GenerationFault::InvalidDivision)? as i16);
        }
        for (scale, value) in self.scale.iter_mut().zip(values) {
            *scale = if absolute {
                value
            } else {
                scale.wrapping_add(value)
            };
        }
        Ok(())
    }

    /// Called after the scene's render traversal, not by the stream clock.
    pub fn step(&mut self) -> Result<bool, GenerationFault> {
        if self.remaining <= 0 {
            return Ok(false);
        }
        self.apply(self.selected.expect("positive count has a packet"), false)?;
        self.remaining = self.remaining.wrapping_sub(1);
        if self.remaining == 0 {
            self.remaining = -1;
        }
        Ok(true)
    }

    /// func_8004D088 also resets a completed request (-1). A zero request
    /// deliberately disables this reset gate without restoring existing scales.
    pub fn reset(&mut self) -> Result<bool, GenerationFault> {
        if self.remaining == 0 {
            return Ok(false);
        }
        self.apply(4, true)?;
        self.remaining = 0;
        Ok(true)
    }
}
