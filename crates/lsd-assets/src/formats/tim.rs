use super::reader::{DecodeError, Reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelMode {
    Indexed4,
    Indexed8,
    Direct16,
    Direct24,
}

#[derive(Debug, Clone)]
pub struct TimBlock<'a> {
    pub x: u16,
    pub y: u16,
    pub width_words: u16,
    pub height: u16,
    pub declared_bytes: u32,
    pub payload: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct TimImage<'a> {
    pub mode: PixelMode,
    pub flags: u32,
    pub palette: Option<TimBlock<'a>>,
    pub image: TimBlock<'a>,
    pub used_bytes: usize,
}

impl<'a> TimImage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, DecodeError> {
        let reader = Reader(data);
        if reader.u32(0)? != 0x10 {
            return Err(DecodeError::new(0, "bad TIM magic"));
        }
        let flags = reader.u32(4)?;
        if flags & !0xB != 0 {
            return Err(DecodeError::new(4, "unsupported TIM flags"));
        }
        let mode = match flags & 3 {
            0 => PixelMode::Indexed4,
            1 => PixelMode::Indexed8,
            2 => PixelMode::Direct16,
            _ => PixelMode::Direct24,
        };
        let mut cursor = 8;
        let mut block = |image: bool| -> Result<TimBlock<'a>, DecodeError> {
            let start = cursor;
            let declared_bytes = reader.u32(start)?;
            let x = reader.u16(start + 4)?;
            let y = reader.u16(start + 6)?;
            let width_words = reader.u16(start + 8)?;
            let height = reader.u16(start + 10)?;
            let payload_bytes = usize::from(width_words)
                .checked_mul(usize::from(height))
                .and_then(|words| words.checked_mul(2))
                .ok_or_else(|| DecodeError::new(start, "TIM rectangle size overflow"))?;
            let physical = 12usize
                .checked_add(payload_bytes)
                .ok_or_else(|| DecodeError::new(start, "TIM block size overflow"))?;
            if declared_bytes < 12 || (!image && (declared_bytes as usize) < physical) {
                return Err(DecodeError::new(start, "invalid TIM block length"));
            }
            // Native GsGetTimInfo ignores the image length word; rectangle size is authoritative.
            let extent = physical.max(declared_bytes as usize);
            reader.bytes(start, extent)?;
            let payload = reader.bytes(start + 12, payload_bytes)?;
            cursor += extent;
            Ok(TimBlock {
                x,
                y,
                width_words,
                height,
                declared_bytes,
                payload,
            })
        };
        let palette = if flags & 8 != 0 {
            Some(block(false)?)
        } else {
            None
        };
        let image = block(true)?;
        Ok(Self {
            mode,
            flags,
            palette,
            image,
            used_bytes: cursor,
        })
    }

    pub fn pixel_width(&self) -> usize {
        let words = usize::from(self.image.width_words);
        match self.mode {
            PixelMode::Indexed4 => words * 4,
            PixelMode::Indexed8 => words * 2,
            PixelMode::Direct16 => words,
            PixelMode::Direct24 => words * 2 / 3,
        }
    }
}

#[derive(Debug)]
pub struct TimList<'a> {
    pub offsets: Vec<usize>,
    pub images: Vec<TimImage<'a>>,
}

impl<'a> TimList<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, DecodeError> {
        let reader = Reader(data);
        let count = reader.u32(0)? as usize;
        reader.array(4, count, 4)?;
        let end = 4 + count * 4;
        let offsets = (0..count)
            .map(|i| reader.u32(4 + i * 4).map(|value| value as usize))
            .collect::<Result<Vec<_>, _>>()?;
        if offsets.iter().any(|&at| at < end || at >= data.len()) {
            return Err(DecodeError::new(4, "TIM list offset outside payload"));
        }
        let mut bounds = offsets.clone();
        bounds.push(data.len());
        bounds.sort_unstable();
        bounds.dedup();
        let images = offsets
            .iter()
            .map(|&at| {
                let next = bounds[bounds.partition_point(|&value| value <= at)];
                TimImage::parse(&data[at..next])
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { offsets, images })
    }
}

#[derive(Debug)]
pub struct Tix<'a> {
    pub banks: Vec<TimList<'a>>,
}

impl<'a> Tix<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, DecodeError> {
        let reader = Reader(data);
        reader.bytes(0, 36)?;
        let count = reader.u32(0)? as usize;
        if !(1..=4).contains(&count) {
            return Err(DecodeError::new(0, "invalid TIX bank count"));
        }
        let mut banks = Vec::with_capacity(count);
        for i in 0..count {
            let offset = reader.u32(4 + i * 4)? as usize;
            let size = reader.u32(20 + i * 4)? as usize;
            if offset < 36 || size < 4 {
                return Err(DecodeError::new(offset, "invalid TIX bank extent"));
            }
            banks.push(TimList::parse(reader.bytes(offset, size)?)?);
        }
        Ok(Self { banks })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSample {
    pub word: u16,
    pub rgba: [u8; 4],
    pub semi_transparent: bool,
}

impl TextureSample {
    pub const fn from_word(word: u16) -> Self {
        const fn channel(value: u16) -> u8 {
            ((value << 3) | (value >> 2)) as u8
        }
        Self {
            word,
            rgba: [
                channel(word & 31),
                channel((word >> 5) & 31),
                channel((word >> 10) & 31),
                if word == 0 { 0 } else { 255 },
            ],
            semi_transparent: word & 0x8000 != 0,
        }
    }
}

pub struct Vram {
    words: Vec<u16>,
}

impl Default for Vram {
    fn default() -> Self {
        Self {
            words: vec![0; 1024 * 512],
        }
    }
}

impl Vram {
    pub fn words(&self) -> &[u16] {
        &self.words
    }

    pub fn upload_block(&mut self, block: &TimBlock<'_>) -> Result<(), DecodeError> {
        let (x, y, width, height) = (
            usize::from(block.x),
            usize::from(block.y),
            usize::from(block.width_words),
            usize::from(block.height),
        );
        if x + width > 1024 || y + height > 512 {
            return Err(DecodeError::new(0, "TIM upload rectangle exceeds VRAM"));
        }
        Reader(block.payload).array(0, width * height, 2)?;
        for row in 0..height {
            for col in 0..width {
                self.words[(y + row) * 1024 + x + col] =
                    Reader(block.payload).u16((row * width + col) * 2)?;
            }
        }
        Ok(())
    }

    pub fn upload(&mut self, image: &TimImage<'_>) -> Result<(), DecodeError> {
        // Preserve native image-then-palette transfer order, including overlaps.
        self.upload_block(&image.image)?;
        if let Some(palette) = &image.palette {
            self.upload_block(palette)?;
        }
        Ok(())
    }

    pub fn sample(&self, page: u16, clut: u16, uv: [u8; 2]) -> Result<TextureSample, DecodeError> {
        let depth = (page >> 7) & 3;
        let pixels_per_word = match depth {
            0 => 4,
            1 => 2,
            2 => 1,
            _ => return Err(DecodeError::new(0, "reserved texture-page depth")),
        };
        let x = usize::from(page & 15) * 64 + usize::from(uv[0]) / pixels_per_word;
        let y = usize::from((page >> 4) & 1) * 256 + usize::from(uv[1]);
        if x >= 1024 || y >= 512 {
            return Err(DecodeError::new(
                0,
                "texture page requires unimplemented VRAM wrap",
            ));
        }
        let packed = self.words[y * 1024 + x];
        let word = if depth == 2 {
            packed
        } else {
            let index = if depth == 0 {
                (packed >> (usize::from(uv[0] & 3) * 4)) & 15
            } else {
                (packed >> (usize::from(uv[0] & 1) * 8)) & 255
            };
            let px = usize::from(clut & 63) * 16 + usize::from(index);
            let py = usize::from((clut >> 6) & 511);
            if px >= 1024 {
                return Err(DecodeError::new(
                    0,
                    "CLUT lookup requires unimplemented VRAM wrap",
                ));
            }
            self.words[py * 1024 + px]
        };
        Ok(TextureSample::from_word(word))
    }

    pub fn texture_rgba(&self, page: u16, clut: u16) -> Result<Vec<u8>, DecodeError> {
        let mut rgba = Vec::with_capacity(256 * 256 * 4);
        for y in 0..=255 {
            for x in 0..=255 {
                rgba.extend(self.sample(page, clut, [x, y])?.rgba);
            }
        }
        Ok(rgba)
    }
}
