use super::{
    Mml, Tmd,
    reader::{DecodeError, Reader},
};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub tag: u8,
    pub category: u8,
    pub model: u16,
    pub flags: u8,
    pub quarter_turn: u8,
    pub height: i16,
    pub next_offset: u32,
}

impl Tile {
    pub fn present(self) -> bool {
        self.tag != 0
    }
    pub fn native_position(self, cell: usize) -> [i32; 3] {
        [
            ((cell % 20) as i32) * 2048 + 1024,
            i32::from(self.height) * 2048,
            ((cell / 20) as i32) * 2048 + 1024,
        ]
    }
    pub fn native_heading(self) -> i16 {
        (u16::from(self.quarter_turn) << 10) as i16
    }
}

#[derive(Debug, Clone)]
pub struct Lbd {
    pub grid_offset: usize,
    pub tmd_offset: usize,
    pub dimensions: [u16; 2],
    pub tiles: Vec<Tile>,
    pub tmd: Tmd,
    pub model_library_extent: Option<Range<usize>>,
    pub model_library: Option<Mml>,
}

impl Lbd {
    pub fn parse(data: &[u8]) -> Result<Self, DecodeError> {
        let reader = Reader(data);
        reader.bytes(0, 24)?;
        let grid_offset = reader.u32(4)? as usize;
        let relative_tmd = reader.u32(8)? as usize;
        if grid_offset < 24
            || !grid_offset.is_multiple_of(4)
            || relative_tmd < 4808
            || !(relative_tmd - 4808).is_multiple_of(12)
        {
            return Err(DecodeError::new(4, "invalid LBD grid/TMD offsets"));
        }
        reader.bytes(grid_offset, relative_tmd)?;
        let tmd_offset = grid_offset + relative_tmd;
        let model_library_extent = if reader.u16(2)? != 0 {
            let at = reader.u32(16)? as usize;
            let size = reader.u32(20)? as usize;
            let bytes = reader.bytes(at, size)?;
            if !at.is_multiple_of(4)
                || at < tmd_offset + 12
                || bytes.len() < 8
                || &bytes[..4] != b"MML "
            {
                return Err(DecodeError::new(at, "invalid LBD MML extent"));
            }
            Some(at..at + size)
        } else {
            None
        };
        let tmd_limit = model_library_extent
            .as_ref()
            .map_or(data.len(), |extent| extent.start);
        let tmd = Tmd::parse(reader.bytes(tmd_offset, tmd_limit - tmd_offset)?)?;
        let model_library = model_library_extent
            .as_ref()
            .map(|extent| Mml::parse(&data[extent.clone()]))
            .transpose()?;
        let count = (relative_tmd - 8) / 12;
        let mut tiles = Vec::with_capacity(count);
        for i in 0..count {
            let at = grid_offset + 8 + i * 12;
            let tile = Tile {
                tag: reader.u8(at)?,
                category: reader.u8(at + 1)?,
                model: reader.u16(at + 2)?,
                flags: reader.u8(at + 4)?,
                quarter_turn: reader.u8(at + 5)?,
                height: reader.i16(at + 6)?,
                next_offset: reader.u32(at + 8)?,
            };
            if tile.present() && usize::from(tile.model) >= tmd.objects.len() {
                return Err(DecodeError::new(
                    at + 2,
                    "present tile references missing TMD object",
                ));
            }
            let next = tile.next_offset as usize;
            if next != 0 && (next < 8 || !(next - 8).is_multiple_of(12) || next + 12 > relative_tmd)
            {
                return Err(DecodeError::new(
                    at + 8,
                    "tile continuation outside grid records",
                ));
            }
            tiles.push(tile);
        }
        let mut state = vec![0u8; count];
        for start in 0..count {
            let mut cursor = Some(start);
            let mut visited = Vec::new();
            while let Some(index) = cursor {
                if state[index] == 2 {
                    break;
                }
                if state[index] == 1 {
                    return Err(DecodeError::new(
                        grid_offset + 8 + index * 12,
                        "cyclic tile chain",
                    ));
                }
                state[index] = 1;
                visited.push(index);
                cursor = (tiles[index].next_offset != 0)
                    .then(|| (tiles[index].next_offset as usize - 8) / 12);
            }
            for index in visited {
                state[index] = 2;
            }
        }
        Ok(Self {
            grid_offset,
            tmd_offset,
            dimensions: [reader.u16(grid_offset + 4)?, reader.u16(grid_offset + 6)?],
            tiles,
            tmd,
            model_library_extent,
            model_library,
        })
    }

    pub fn cell_chain(&self, cell: usize) -> Result<Vec<&Tile>, DecodeError> {
        if cell >= 400 {
            return Err(DecodeError::new(cell, "cell outside 20 by 20 grid"));
        }
        let mut chain = Vec::new();
        let mut index = cell;
        loop {
            let tile = &self.tiles[index];
            chain.push(tile);
            if tile.next_offset == 0 {
                break;
            }
            index = (tile.next_offset as usize - 8) / 12;
        }
        Ok(chain)
    }
}
