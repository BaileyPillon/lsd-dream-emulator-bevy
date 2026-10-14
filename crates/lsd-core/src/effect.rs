use std::fmt;

/// Imported locally, or supplied by a synthetic test. No original palette is embedded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectPalette {
    pub start: [[u8; 3]; 8],
    pub attached: [[u8; 3]; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectFault {
    InvalidPaletteIndex,
    InvalidDivision,
}
impl fmt::Display for EffectFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidPaletteIndex => "effect variant is outside the supplied palette",
            Self::InvalidDivision => "effect division has no supported signed result",
        })
    }
}
impl std::error::Error for EffectFault {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectEvent {
    Attach,
    Detach,
    Notify(i32),
}

/// Original scalar effect controller, independent of GPU blending and actor ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectState {
    pub color: [u8; 3],
    pub render_flags: u32,
    pub color_mask: i32,
    pub state: i32,
    pub variant: i32,
    pub color_step: i32,
    pub channels: i32,
    pub mode: i32,
    pub life: i32,
    pub mask_step: i32,
    pub linked: i32,
    pub linked_divisor: i32,
}
impl EffectState {
    pub fn new(color: [u8; 3], color_mask: i32, variant: i32) -> Self {
        let mut result = Self {
            color,
            color_mask,
            variant,
            render_flags: 0,
            state: 0,
            color_step: 10,
            channels: 0,
            mode: 0,
            life: 0,
            mask_step: 0,
            linked: 0,
            linked_divisor: 0,
        };
        result.reset(variant);
        result
    }

    fn visible(&mut self, value: bool) {
        self.render_flags = (self.render_flags & !(1 << 31)) | (u32::from(!value) << 31);
    }
    fn enabled(&mut self, value: bool) {
        self.render_flags = (self.render_flags & !(1 << 30)) | (u32::from(value) << 30);
    }
    pub fn reset(&mut self, variant: i32) {
        self.variant = variant;
        self.state = 0;
        self.color_step = 10;
        self.channels = 0;
        self.mode = 0;
        self.visible(false);
        self.enabled(false);
        self.linked = 0;
    }

    /// D4 starts the decreasing cycle; D8 starts the increasing/attached cycle.
    pub fn start(
        &mut self,
        palette: &EffectPalette,
        attached: bool,
        variant: i32,
        mode: i32,
    ) -> Result<Vec<EffectEvent>, EffectFault> {
        if self.state != 0 {
            return Ok(Vec::new());
        }
        let variant = if variant < 0 { self.variant } else { variant };
        let index = usize::try_from(variant)
            .ok()
            .filter(|&i| i < 8)
            .ok_or(EffectFault::InvalidPaletteIndex)?;
        let life = 256_i32
            .checked_div(self.color_step)
            .ok_or(EffectFault::InvalidDivision)?;
        let life = if self.linked != 0 {
            life.wrapping_sub(
                life.checked_div(self.linked_divisor)
                    .ok_or(EffectFault::InvalidDivision)?,
            )
        } else {
            life
        };
        let mask_step = self
            .color_mask
            .checked_div(life)
            .ok_or(EffectFault::InvalidDivision)?;
        self.variant = variant;
        self.channels = if variant == 0 { 15 } else { variant };
        self.mode = mode;
        self.life = life;
        self.mask_step = mask_step;
        self.enabled(true);
        let render_mode = if variant == 0 { 2 } else { 1 };
        self.render_flags = (self.render_flags & !(3 << 28)) | (render_mode << 28);
        self.visible(true);
        if attached {
            if self.linked != 0 {
                self.life = self.life.wrapping_sub(1);
            } else {
                self.color = palette.attached[index];
            }
            self.state = 2;
        } else {
            self.color = palette.start[index];
            self.state = 1;
            self.color_step = self.color_step.wrapping_neg();
        }
        Ok(vec![EffectEvent::Attach])
    }

    pub fn deactivate(&mut self, palette: &EffectPalette) -> Vec<EffectEvent> {
        if self.state == 0 {
            return Vec::new();
        }
        let code = if self.state == 1 {
            if self.linked == 0 {
                self.visible(false);
                self.enabled(false);
            }
            5
        } else {
            if self.linked != 0 {
                if self.channels == 15 {
                    self.color = palette.attached[0];
                }
                self.enabled(false);
            }
            6
        };
        if self.color_step < 0 {
            self.color_step = self.color_step.wrapping_neg();
        }
        self.state = 0;
        vec![EffectEvent::Detach, EffectEvent::Notify(code)]
    }

    /// Original notification 2 decrements even an inactive controller when explicitly called.
    pub fn tick(&mut self, palette: &EffectPalette, code: i32) -> Vec<EffectEvent> {
        if code != 2 {
            return Vec::new();
        }
        let previous = self.life;
        self.life = previous.wrapping_sub(1);
        if previous <= 0 {
            return self.deactivate(palette);
        }
        if self.mode != 9 {
            for (value, mask) in self.color.iter_mut().zip([4, 2, 1]) {
                if self.channels & mask != 0 {
                    *value = value.wrapping_add(self.color_step as u8);
                }
            }
        }
        Vec::new()
    }

    pub fn completion_color(&self, palette: &EffectPalette) -> Result<[u8; 3], EffectFault> {
        if self.channels == 15 {
            Ok(palette.attached[0])
        } else {
            usize::try_from(self.channels)
                .ok()
                .and_then(|index| palette.start.get(index))
                .copied()
                .ok_or(EffectFault::InvalidPaletteIndex)
        }
    }
}
