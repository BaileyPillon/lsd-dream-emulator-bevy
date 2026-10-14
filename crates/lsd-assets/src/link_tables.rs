use crate::{PsxExeHeader, is_canonical_reference};
use lsd_core::{LinkCatalog, LinkMotionRules, LinkSet, LinkTrigger, SpawnEntry};

pub struct OriginalLinkTables {
    pub catalog: LinkCatalog,
    pub motion: LinkMotionRules,
}

/// Load reference link content locally. None of these original tables are bundled.
pub fn load_link_tables(executable: &[u8]) -> Result<OriginalLinkTables, &'static str> {
    if !is_canonical_reference(executable) {
        return Err("Link tables require canonical SLPS_015.56");
    }
    let header = PsxExeHeader::parse(executable).map_err(|_| "Invalid executable extent")?;
    let bytes = |address: u32, size: usize| -> Result<&[u8], &'static str> {
        let offset = address
            .checked_sub(header.load_address)
            .ok_or("Link pointer below executable")? as usize;
        let start = offset.checked_add(0x800).ok_or("Link offset overflow")?;
        let end = start.checked_add(size).ok_or("Link size overflow")?;
        executable
            .get(start..end)
            .ok_or("Link pointer outside executable")
    };
    let word = |address: u32| -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(bytes(address, 4)?.try_into().unwrap()))
    };
    let entries = |address: u32, count: usize| -> Result<Vec<SpawnEntry>, &'static str> {
        if count == 0 {
            return Ok(Vec::new());
        }
        bytes(address, count * 6)?
            .chunks_exact(6)
            .map(|record| {
                let adjustment = record[4];
                if adjustment >= 37 {
                    return Err("Invalid spawn adjustment");
                }
                Ok(SpawnEntry {
                    chunk: i16::from_le_bytes(record[..2].try_into().unwrap()),
                    tile: i16::from_le_bytes(record[2..4].try_into().unwrap()),
                    adjustment,
                    extra: record[5] as i8,
                })
            })
            .collect()
    };
    let family = |counts: u32,
                  triggers: u32,
                  spawns: u32,
                  source_transforms: Option<u32>,
                  target_transforms: Option<u32>|
     -> Result<LinkSet, &'static str> {
        let mut set = LinkSet {
            triggers: std::array::from_fn(|_| Vec::new()),
            spawns: std::array::from_fn(|_| Vec::new()),
            trigger_transforms: source_transforms.map(|_| std::array::from_fn(|_| Vec::new())),
            spawn_transforms: target_transforms.map(|_| std::array::from_fn(|_| Vec::new())),
        };
        let mut lengths = [0usize; 14];
        for stage in 0..14 {
            let count = usize::from(bytes(counts + stage as u32, 1)?[0]);
            if count == 0 {
                continue;
            }
            let address = word(triggers + stage as u32 * 4)?;
            for record in bytes(address, count * 6)?.chunks_exact(6) {
                let target_stage = record[4] as i8;
                if !(0..14).contains(&target_stage) {
                    return Err("Invalid link target stage");
                }
                let target_spawn = record[5];
                lengths[target_stage as usize] =
                    lengths[target_stage as usize].max(usize::from(target_spawn) + 1);
                set.triggers[stage].push(LinkTrigger {
                    chunk: i16::from_le_bytes(record[..2].try_into().unwrap()),
                    tile: i16::from_le_bytes(record[2..4].try_into().unwrap()),
                    target_stage,
                    target_spawn,
                });
            }
            if let Some(table) = source_transforms {
                let transforms = bytes(word(table + stage as u32 * 4)?, count)?.to_vec();
                if transforms.iter().any(|&index| index >= 8) {
                    return Err("Invalid source link transform");
                }
                set.trigger_transforms.as_mut().unwrap()[stage] = transforms;
            }
        }
        for (stage, count) in lengths.into_iter().enumerate() {
            set.spawns[stage] = entries(word(spawns + stage as u32 * 4)?, count)?;
            if let Some(table) = target_transforms {
                if count == 0 {
                    continue;
                }
                let transforms = bytes(word(table + stage as u32 * 4)?, count)?.to_vec();
                if transforms.iter().any(|&index| index >= 8) {
                    return Err("Invalid target link transform");
                }
                set.spawn_transforms.as_mut().unwrap()[stage] = transforms;
            }
        }
        Ok(set)
    };
    let mut random_spawns = std::array::from_fn(|_| Vec::new());
    for (stage, row) in random_spawns.iter_mut().enumerate() {
        *row = entries(
            word(0x8008_85b8 + stage as u32 * 4)?,
            usize::from(bytes(0x8008_85f0 + stage as u32, 1)?[0]),
        )?;
    }
    // All referenced tunnel/stair transform indices fit the first eight records.
    let mut transforms = Vec::new();
    let mut facing_angles = Vec::new();
    for index in 0..8 {
        let at = 0x8008_8758 + index * 12;
        transforms.push([word(at)?, word(at + 4)?, word(at + 8)?]);
        facing_angles.push(u16::from_le_bytes(bytes(at + 4, 2)?.try_into().unwrap()));
    }
    let catalog = LinkCatalog {
        adjustments: bytes(0x8008_7f30, 37 * 6)?
            .chunks_exact(6)
            .map(|record| {
                std::array::from_fn(|axis| {
                    i16::from_le_bytes(record[axis * 2..axis * 2 + 2].try_into().unwrap())
                })
            })
            .collect(),
        random_spawns,
        wall: family(0x8008_8748, 0x8008_8710, 0x8008_8664, None, None)?,
        tunnel: family(
            0x8008_89f0,
            0x8008_8980,
            0x8008_8820,
            Some(0x8008_89b8),
            Some(0x8008_8858),
        )?,
        teleport: family(0x8008_8b5c, 0x8008_8b24, 0x8008_8a80, None, None)?,
        staircase: family(
            0x8008_8cbc,
            0x8008_8c4c,
            0x8008_8ba4,
            Some(0x8008_8c84),
            Some(0x8008_8bdc),
        )?,
        transforms,
        facing_angles,
        violence_key: word(0x8008_abe8)?,
    };
    if word(0x8008_7e40)? != 0x8005_75b0
        || word(0x8008_7e44)? != 0x8005_75b0
        || word(0x8008_7e48)? != 0x8005_75e0
        || word(0x8008_7e4c)? != 0x8005_75e0
    {
        return Err("Unsupported original movement action table");
    }
    let mut distances = [0; 5];
    for (index, value) in distances.iter_mut().enumerate() {
        *value = word(0x8008_7e20 + index as u32 * 4)? as i32;
    }
    let signs = bytes(0x8008_7e34, 5)?
        .iter()
        .map(|&value| value as i8)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    Ok(OriginalLinkTables {
        catalog,
        motion: LinkMotionRules { distances, signs },
    })
}
