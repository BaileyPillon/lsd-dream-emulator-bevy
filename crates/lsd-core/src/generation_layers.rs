use crate::{GenerationTables, shift_generation_color};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VramRectangle {
    pub x: i16,
    pub y: i16,
    pub width: i32,
    pub height: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VramMove {
    pub source: VramRectangle,
    pub destination: [i32; 2],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationEffectTables {
    pub environment_origin: [i32; 2],
    pub environment_size: [i32; 2],
    pub gradient_origin: [i32; 2],
    pub gradient_size: [i32; 2],
    pub borders: [VramRectangle; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationLayer {
    pub offset: [i32; 2],
    pub size: [u16; 2],
    pub color: [u8; 3],
    pub flags: u32,
    pub depth: i32,
    pub parent: Option<usize>,
    pub relative: bool,
}
impl GenerationLayer {
    /// Original relative coordinates are percentages of half the viewport.
    pub fn screen_rectangle(self, screen: [i32; 2]) -> [i32; 4] {
        let offset = std::array::from_fn::<_, 2, _>(|i| {
            if self.relative {
                i32::from(((screen[i] >> 1).wrapping_mul(self.offset[i]) / 100) as i16)
            } else {
                i32::from(self.offset[i] as i16)
            }
        });
        [
            offset[0].wrapping_add(screen[0] >> 1),
            offset[1].wrapping_add(screen[1] >> 1),
            i32::from(self.size[0]),
            i32::from(self.size[1]),
        ]
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GenerationLayers {
    pub environment: Option<GenerationLayer>,
    pub gradient: Vec<GenerationLayer>,
    pub stage_color: Option<[u8; 3]>,
}
impl GenerationLayers {
    pub fn create(
        &mut self,
        tables: &GenerationTables,
        geometry: &GenerationEffectTables,
        environment: Option<[u8; 3]>,
        flags: i32,
        palette: usize,
    ) {
        if let Some(color) = environment {
            self.environment = Some(GenerationLayer {
                offset: geometry.environment_origin,
                size: geometry.environment_size.map(|v| v as u16),
                color,
                flags: 1 << 30,
                depth: 0,
                parent: None,
                relative: true,
            });
        }
        if flags != 0 {
            let mut offset = geometry.gradient_origin;
            if flags == 2 {
                offset[1] = offset[1].wrapping_add(30);
            }
            let mut size = geometry.gradient_size;
            self.gradient.clear();
            self.gradient.push(GenerationLayer {
                offset: [0; 2],
                size: size.map(|v| v as u16),
                color: tables.effect_palettes[palette][0],
                flags: 0,
                depth: 8191,
                parent: None,
                relative: true,
            });
            for index in 1..18 {
                self.gradient.push(GenerationLayer {
                    offset,
                    size: size.map(|v| v as u16),
                    color: tables.effect_palettes[palette][index],
                    flags: 0,
                    depth: 8191,
                    parent: Some(0),
                    relative: true,
                });
                offset[1] = offset[1].wrapping_add(3);
                size[1] = size[1].wrapping_sub(7);
            }
            self.gradient[0].offset = offset;
        }
    }
    pub fn update(
        &mut self,
        tables: &GenerationTables,
        geometry: &GenerationEffectTables,
        flags: i32,
        palette: usize,
        base_color: [u8; 3],
        view: [i32; 2],
    ) {
        if flags == 0 {
            return;
        }
        let step = (view[0].wrapping_sub(view[1]) / 600).wrapping_mul(3);
        if step <= 0 {
            return;
        }
        let mut offset = geometry.gradient_origin;
        if flags == 2 {
            offset[1] = offset[1].wrapping_add(30);
        }
        offset[1] = offset[1].wrapping_add(step.wrapping_mul(3));
        for (index, layer) in self.gradient.iter_mut().enumerate() {
            layer.color = shift_generation_color(tables.effect_palettes[palette][index], step);
            layer.offset = offset;
            offset[1] = offset[1].wrapping_add(3);
        }
        self.stage_color = Some(shift_generation_color(base_color, step));
    }
    pub fn stop(&mut self) {
        self.environment = None;
        self.gradient.clear();
    }
}

pub fn generation_border_moves(stage: i32, tables: &GenerationEffectTables) -> Vec<VramMove> {
    let index = if stage == 2 {
        0
    } else if (stage.wrapping_sub(3) as u32) < 3 {
        2
    } else {
        return Vec::new();
    };
    let rect = tables.borders[index];
    let temporary = tables.borders[index + 1];
    vec![
        VramMove {
            source: VramRectangle {
                x: (i32::from(rect.x).wrapping_add(rect.width).wrapping_sub(1)) as i16,
                y: rect.y,
                width: 1,
                height: rect.height,
            },
            destination: [i32::from(temporary.x), i32::from(temporary.y)],
        },
        VramMove {
            source: VramRectangle {
                x: rect.x,
                y: rect.y,
                width: rect.width.wrapping_sub(1),
                height: rect.height,
            },
            destination: [i32::from(rect.x).wrapping_add(1), i32::from(rect.y)],
        },
        VramMove {
            source: VramRectangle {
                x: temporary.x,
                y: temporary.y,
                width: 1,
                height: rect.height,
            },
            destination: [i32::from(rect.x), i32::from(rect.y)],
        },
    ]
}
