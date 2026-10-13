use crate::formats::Lbd;
use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::SceneGrid;
use lsd_core::{CollisionPolicy, SceneCollision, SceneSlot, SceneSlots};
use lsd_core::{SlotChoice, StreamGridTables};
use std::path::{Path, PathBuf};

/// Resolve the original catalogue entry, including maps in N1/N2/... subdirectories.
pub fn stage_map_path(executable: &[u8], stage: u8, map: u16) -> Result<PathBuf, &'static str> {
    let grid = load_scene_grid(executable, stage)?;
    if i32::from(map) >= i32::from(grid.columns) * i32::from(grid.rows) {
        return Err("Invalid map index");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let at = (0x8008_19E8 - header.load_address) as usize + 0x800 + usize::from(stage) * 2;
    let first = u16::from_le_bytes(executable[at..at + 2].try_into().unwrap());
    let at = (0x8008_1A04 - header.load_address) as usize
        + 0x800
        + (usize::from(first) + 9 + usize::from(map)) * 28;
    let record = &executable[at..at + 28];
    let end = record
        .iter()
        .position(|&byte| byte == 0)
        .ok_or("Invalid native path record")?;
    let relative = std::str::from_utf8(&record[..end]).map_err(|_| "Invalid native path text")?;
    Ok(PathBuf::from("CDI").join(relative.replace('\\', "/")))
}

pub struct LayeredStage {
    pub collision: SceneCollision,
    pub maps: Vec<Lbd>,
    pub origin: [i32; 3],
}

pub fn load_stream_grid_tables(executable: &[u8]) -> Result<StreamGridTables, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Stream tables require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let offset = |address: u32| (address - header.load_address) as usize + 0x800;
    let word = |address: u32| {
        u32::from_le_bytes(
            executable[offset(address)..offset(address) + 4]
                .try_into()
                .unwrap(),
        )
    };
    let choices = |address: u32| {
        std::array::from_fn(|index| SlotChoice {
            direction: executable[offset(address) + index * 2],
            move_slot: executable[offset(address) + index * 2 + 1] != 0,
        })
    };
    Ok(StreamGridTables {
        origin_deltas: std::array::from_fn(|index| {
            std::array::from_fn(|axis| word(0x8008_6838 + (index * 12 + axis * 4) as u32) as i32)
        }),
        direction_masks: std::array::from_fn(|index| word(0x8008_688C + index as u32 * 4)),
        index_steps: std::array::from_fn(|index| {
            std::array::from_fn(|axis| word(0x8008_68A8 + (index * 12 + axis * 4) as u32) as i32)
        }),
        label_kinds: std::array::from_fn(|index| executable[offset(0x8008_68FC) + index]),
        initial: choices(0x8008_6904),
        reuse: std::array::from_fn(|kind| {
            let pointer = word(0x8008_6974 + kind as u32 * 4);
            if pointer == 0 {
                None
            } else {
                Some(choices(pointer))
            }
        }),
    })
}

/// Load a layered stage for inspection. Original asynchronous stream lifecycle is separate.
pub fn load_layered_stage(
    files: &Path,
    executable: &[u8],
    stage: u8,
    base: [i32; 3],
) -> Result<LayeredStage, Box<dyn std::error::Error>> {
    let grid = load_scene_grid(executable, stage)?;
    if grid.mode != 1 || grid.columns != 1 || !(1..=7).contains(&grid.rows) {
        return Err(
            "Layered scene inspection requires STG00 or STG06; spatial streaming remains pending"
                .into(),
        );
    }
    let placement = grid.place(base, [0, 0], [0, 0], [0, 0, 0]);
    let origin = [
        placement.center[0].wrapping_sub(20480),
        placement.center[1],
        placement.center[2].wrapping_sub(20480),
    ];
    let mut maps = Vec::new();
    let mut chunks = std::array::from_fn(|_| None);
    for (index, chunk) in chunks.iter_mut().enumerate().take(grid.rows as usize) {
        let map = Lbd::parse(&std::fs::read(files.join(stage_map_path(
            executable,
            stage,
            index as u16,
        )?))?)?;
        *chunk = Some(map.collision_chunk(CollisionPolicy::for_stage(stage)));
        maps.push(map);
    }
    let slots = SceneSlots {
        grid,
        slots: std::array::from_fn(|index| SceneSlot {
            label: index as i16,
            map_index: index as i16,
            origin,
            ready: index < maps.len(),
        }),
    };
    slots.validate()?;
    Ok(LayeredStage {
        collision: SceneCollision { slots, chunks },
        maps,
        origin,
    })
}

/// Stage descriptors stay in the local executable; no original table is bundled.
pub fn load_scene_grid(executable: &[u8], stage: u8) -> Result<SceneGrid, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Scene grid requires canonical SLPS_015.56");
    }
    if stage >= 14 {
        return Err("Invalid stage index");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let at = (0x8008_61D4 - header.load_address) as usize + 0x800 + usize::from(stage) * 8;
    Ok(SceneGrid {
        columns: i16::from_le_bytes(executable[at..at + 2].try_into().unwrap()),
        rows: i16::from_le_bytes(executable[at + 2..at + 4].try_into().unwrap()),
        mode: i32::from_le_bytes(executable[at + 4..at + 8].try_into().unwrap()),
    })
}
