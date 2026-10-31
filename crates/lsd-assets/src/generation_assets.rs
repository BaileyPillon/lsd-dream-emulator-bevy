use crate::{
    formats::{TimImage, Tmd, Vram},
    load_generation_object_tables,
};
use lsd_core::GenerationSprite;
use std::path::Path;

pub struct GenerationAssets {
    pub model: Tmd,
    pub vram: Vram,
    pub sprite_page: u16,
}
impl GenerationAssets {
    pub fn load(files: &Path, executable: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_from_source(&crate::AssetSource::original(files), executable)
    }

    /// Load generation models and textures through the selected content profile.
    pub fn load_from_source(
        files: &crate::AssetSource,
        executable: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let tables = load_generation_object_tables(executable)?;
        let mut model = Tmd::parse(&files.read("CDI/ETC/DREAMER.TMD")?)?;
        let [x, y] = tables.palette_coordinates;
        let clut = (i32::from(x) / 16).wrapping_add(i32::from(y).wrapping_shl(6)) as u16;
        for index in tables.model_indices {
            let object = model
                .objects
                .get_mut(index as usize)
                .ok_or("Generation model index is outside DREAMER.TMD")?;
            let polygon = object
                .polygons
                .first_mut()
                .ok_or("Generation model has no primitive")?;
            if polygon.clut.is_none() {
                return Err("Generation primitive has no CLUT field".into());
            }
            polygon.clut = Some(clut);
        }
        let texture = files.read("CDI/ETC/ETC.TIM")?;
        let image = TimImage::parse(&texture)?;
        let sprite_page = (((image.flags & 3) as u16) << 7)
            | ((image.image.y & 0x100) >> 4)
            | ((image.image.x & 0x3ff) >> 6);
        let mut vram = Vram::default();
        vram.upload(&image)?;
        Ok(Self {
            model,
            vram,
            sprite_page,
        })
    }

    pub fn sprite_rgba(
        &self,
        sprite: &GenerationSprite,
    ) -> Result<(u32, u32, Vec<u8>), Box<dyn std::error::Error>> {
        let width = u32::from(sprite.rectangle[2]);
        let height = u32::from(sprite.rectangle[4]);
        if width == 0 || height == 0 || width > 256 || height > 256 {
            return Err("Invalid generation sprite extent".into());
        }
        let clut = ((sprite.clut[0] as u16 >> 4) & 63) | ((sprite.clut[1] as u16 & 511) << 6);
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let uv = [
                    (u32::from(sprite.rectangle[0]) + x) as u8,
                    (u32::from(sprite.rectangle[1]) + y) as u8,
                ];
                let sample = self.vram.sample(self.sprite_page, clut, uv)?;
                let color = std::array::from_fn::<_, 3, _>(|i| {
                    let value = (((sample.word >> (i * 5)) & 31) * u16::from(sprite.color[i])
                        / 128)
                        .min(31);
                    ((value << 3) | (value >> 2)) as u8
                });
                pixels.extend(color);
                pixels.push(if sample.word == 0 {
                    0
                } else if sprite.flags & (1 << 30) != 0 && sample.semi_transparent {
                    128
                } else {
                    255
                });
            }
        }
        Ok((width, height, pixels))
    }
}
