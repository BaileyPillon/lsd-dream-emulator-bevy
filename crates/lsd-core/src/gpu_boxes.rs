//! Untextured rectangle packets and RGB555 composition for original screen boxes.
//! Textured primitives, VRAM masks, interlacing and hardware timing are separate.
use crate::GenerationLayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBoxPacket {
    pub draw_mode: u32,
    pub color_command: u32,
    pub vertex: u32,
    pub size: u32,
}
impl GpuBoxPacket {
    pub const fn words(self) -> [u32; 4] {
        [self.draw_mode, self.color_command, self.vertex, self.size]
    }
    pub fn rectangle(self) -> [i32; 4] {
        fn coordinate(value: u32) -> i32 {
            ((value & 0x7ff) as i32 ^ 1024) - 1024
        }
        [
            coordinate(self.vertex),
            coordinate(self.vertex >> 16),
            (self.size & 1023) as i32,
            ((self.size >> 16) & 511) as i32,
        ]
    }
    pub fn foreground(self) -> u16 {
        let rgb = self.color_command;
        ((rgb >> 3) & 31) as u16
            | (((rgb >> 11) & 31) as u16) << 5
            | (((rgb >> 19) & 31) as u16) << 10
    }
    pub const fn semi_transparent(self) -> bool {
        self.color_command & (1 << 25) != 0
    }
    pub const fn blend_mode(self) -> u8 {
        ((self.draw_mode >> 5) & 3) as u8
    }
}
impl GenerationLayer {
    /// Original GsSortBoxFill writes draw-mode, rectangle color, signed-short
    /// coordinates plus viewport center, and unsigned-short dimensions.
    pub fn gpu_box_packet(self, screen: [i32; 2]) -> Option<GpuBoxPacket> {
        if self.flags & (1 << 31) != 0 {
            return None;
        }
        let [x, y, _, _] = self.screen_rectangle(screen);
        let [r, g, b] = self.color;
        Some(GpuBoxPacket {
            draw_mode: 0xe100_0200 | ((self.flags >> 17) & 0x180) | ((self.flags >> 23) & 0x60),
            color_command: u32::from(r)
                | (u32::from(g) << 8)
                | (u32::from(b) << 16)
                | ((0x60 | ((self.flags >> 29) & 2)) << 24),
            vertex: u32::from(x as u16) | (u32::from(y as u16) << 16),
            size: u32::from(self.size[0]) | (u32::from(self.size[1]) << 16),
        })
    }
}
pub fn rgb555(color: [u8; 3]) -> u16 {
    u16::from(color[0] >> 3) | (u16::from(color[1] >> 3) << 5) | (u16::from(color[2] >> 3) << 10)
}
pub fn rgb555_rgba(word: u16) -> [u8; 4] {
    fn channel(v: u16) -> u8 {
        ((v << 3) | (v >> 2)) as u8
    }
    [
        channel(word & 31),
        channel((word >> 5) & 31),
        channel((word >> 10) & 31),
        255,
    ]
}
pub fn blend_rgb555(back: u16, front: u16, mode: u8) -> u16 {
    let mut word = 0;
    for shift in [0, 5, 10] {
        let b = (back >> shift) & 31;
        let f = (front >> shift) & 31;
        let c = match mode & 3 {
            0 => (b + f) >> 1,
            1 => (b + f).min(31),
            2 => b.saturating_sub(f),
            _ => (b + (f >> 2)).min(31),
        };
        word |= c << shift;
    }
    word
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxFramebuffer {
    width: usize,
    height: usize,
    pixels: Vec<u16>,
}
impl BoxFramebuffer {
    pub fn new(width: u16, height: u16, clear: [u8; 3]) -> Self {
        Self {
            width: usize::from(width),
            height: usize::from(height),
            pixels: vec![rgb555(clear); usize::from(width) * usize::from(height)],
        }
    }
    pub fn pixels(&self) -> &[u16] {
        &self.pixels
    }
    pub fn set_pixels(&mut self, pixels: &[u16]) -> Result<(), &'static str> {
        if pixels.len() != self.pixels.len() {
            return Err("RGB555 pixel count does not match framebuffer");
        }
        for (output, input) in self.pixels.iter_mut().zip(pixels) {
            *output = input & 0x7fff;
        }
        Ok(())
    }
    pub fn rgba(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|&word| rgb555_rgba(word))
            .collect()
    }
    /// Screen-local drawing area. Packets already include the SDK viewport
    /// center; coordinates retain the GPU's signed eleven-bit interpretation.
    pub fn draw_box(&mut self, packet: GpuBoxPacket) {
        let [x, y, width, height] = packet.rectangle();
        let left = x.max(0).min(self.width as i32) as usize;
        let top = y.max(0).min(self.height as i32) as usize;
        let right = (x + width).max(0).min(self.width as i32) as usize;
        let bottom = (y + height).max(0).min(self.height as i32) as usize;
        let foreground = packet.foreground();
        for py in top..bottom {
            for px in left..right {
                let pixel = &mut self.pixels[py * self.width + px];
                *pixel = if packet.semi_transparent() {
                    blend_rgb555(*pixel, foreground, packet.blend_mode())
                } else {
                    foreground
                };
            }
        }
    }
}
