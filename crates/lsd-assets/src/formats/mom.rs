use super::{
    Tmd,
    reader::{DecodeError, Reader},
};
use lsd_core::{AnimationClip, AnimationFrame, AnimationRecord, AnimationWord};

#[derive(Debug, Clone)]
pub struct Mom {
    pub declared_bytes: usize,
    pub tmd_offset: usize,
    pub tmd: Tmd,
    pub animation_offsets: Vec<usize>,
    pub animations: Vec<AnimationClip>,
    pub initial_part_types: Vec<u8>,
}

impl Mom {
    pub fn construct_actor(&self) -> Result<lsd_core::ModelActor, lsd_core::AnimationError> {
        lsd_core::ModelActor::construct(
            &self.animations,
            &self.initial_part_types,
            self.tmd.objects.len(),
        )
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, DecodeError> {
        let reader = Reader(bytes);
        if reader.bytes(0, 4)? != b"MOM " {
            return Err(DecodeError::new(0, "bad MOM signature"));
        }
        let size = reader.u32(4)? as usize;
        if size < 24 {
            return Err(DecodeError::new(4, "MOM size too small"));
        }
        let bytes = reader.bytes(0, size)?;
        let reader = Reader(bytes);
        let tmd_offset = reader.u32(8)? as usize;
        if tmd_offset < 20 || !tmd_offset.is_multiple_of(4) {
            return Err(DecodeError::new(8, "invalid MOM TMD offset"));
        }
        reader.bytes(tmd_offset, 12)?;
        if reader.bytes(12, 4)? != b"MOS " {
            return Err(DecodeError::new(12, "missing MOS signature"));
        }
        let count = reader.u32(16)? as usize;
        reader.array(20, count, 4)?;
        let table_end = 20 + count * 4;
        if table_end > tmd_offset {
            return Err(DecodeError::new(16, "MOS table overlaps TMD"));
        }
        let mut offsets = Vec::with_capacity(count);
        for i in 0..count {
            let at = 12usize
                .checked_add(reader.u32(20 + i * 4)? as usize)
                .ok_or_else(|| DecodeError::new(20 + i * 4, "MOS offset overflow"))?;
            if at < table_end || !at.is_multiple_of(4) || at > tmd_offset.saturating_sub(8) {
                return Err(DecodeError::new(
                    20 + i * 4,
                    "MOS animation offset outside payload",
                ));
            }
            offsets.push(at);
        }
        let mut bounds = offsets.clone();
        bounds.push(tmd_offset);
        bounds.sort_unstable();
        bounds.dedup();
        let mut animations = Vec::with_capacity(count);
        for &at in &offsets {
            let end = bounds[bounds.partition_point(|&value| value <= at)];
            let data = Reader(reader.bytes(at, end - at)?);
            let frame_count = data.u32(4)? as usize;
            if frame_count > (data.0.len() - 8) / 8 {
                return Err(DecodeError::new(at, "MOS frame count exceeds extent"));
            }
            let mut cursor = 8;
            let mut frames = Vec::with_capacity(frame_count);
            for _ in 0..frame_count {
                let tag = data.u16(cursor)?;
                let record_count = usize::from(data.u16(cursor + 2)?);
                let metadata = data.u32(cursor + 4)?;
                cursor += 8;
                if record_count > data.0.len().saturating_sub(cursor) / 4 {
                    return Err(DecodeError::new(
                        at + cursor,
                        "MOS record count exceeds extent",
                    ));
                }
                let mut records = Vec::with_capacity(record_count);
                for _ in 0..record_count {
                    let header = AnimationWord::decode(data.u32(cursor)?);
                    let length = usize::from(header.word_count) * 4;
                    if length == 0 {
                        return Err(DecodeError::new(at + cursor, "zero-length MOS command"));
                    }
                    data.bytes(cursor, length)?;
                    let minimum = match header.kind {
                        0 => 3,
                        2 | 3 => 2,
                        1 => {
                            1 + usize::from(header.flags & 2 != 0) * 3
                                + usize::from(header.flags & 4 != 0) * 2
                                + usize::from(header.flags & 8 != 0) * 3
                        }
                        _ => 1,
                    };
                    if usize::from(header.word_count) < minimum {
                        return Err(DecodeError::new(
                            at + cursor,
                            "MOS command shorter than native reads",
                        ));
                    }
                    let payload = (1..usize::from(header.word_count))
                        .map(|i| data.u32(cursor + i * 4))
                        .collect::<Result<Vec<_>, _>>()?;
                    records.push(AnimationRecord { header, payload });
                    cursor += length;
                }
                frames.push(AnimationFrame {
                    tag,
                    metadata,
                    records,
                });
            }
            if cursor != data.0.len() {
                return Err(DecodeError::new(
                    at + cursor,
                    "MOS animation has unexplained trailing bytes",
                ));
            }
            animations.push(AnimationClip {
                tag: data.u16(0)?,
                header_halfword: data.u16(2)?,
                frames,
            });
        }
        let initial_part_types = animations
            .first()
            .and_then(|clip| clip.frames.first())
            .map(|frame| {
                frame
                    .records
                    .iter()
                    .filter(|record| record.header.kind == 8 && record.header.flags == 0)
                    .map(|record| record.header.part_type)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if initial_part_types.len() > 255 {
            return Err(DecodeError::new(
                12,
                "part count exceeds native byte allocation domain",
            ));
        }
        let tmd = Tmd::parse(&bytes[tmd_offset..])?;
        for clip in &animations {
            for frame in &clip.frames {
                for record in &frame.records {
                    // The original interpreter skips commands with an unknown child type.
                    if !initial_part_types.contains(&record.header.part_type) {
                        continue;
                    }
                    if record.header.kind == 2
                        && usize::from(record.payload[0] as u16) > tmd.objects.len()
                    {
                        return Err(DecodeError::new(
                            12,
                            "animation references missing TMD primitive",
                        ));
                    }
                    if record.header.kind == 3
                        && !matches!(record.payload[0], 0 | 0xFFFF)
                        && !initial_part_types.contains(&(record.payload[0] as u8))
                    {
                        return Err(DecodeError::new(
                            12,
                            "animation references missing parent part",
                        ));
                    }
                }
            }
        }
        Ok(Self {
            declared_bytes: size,
            tmd_offset,
            tmd,
            animation_offsets: offsets,
            animations,
            initial_part_types,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Mml {
    pub offsets: Vec<usize>,
    pub models: Vec<Mom>,
}

impl Mml {
    pub fn parse(bytes: &[u8]) -> Result<Self, DecodeError> {
        let reader = Reader(bytes);
        if reader.bytes(0, 4)? != b"MML " {
            return Err(DecodeError::new(0, "bad MML signature"));
        }
        let count = reader.u32(4)? as usize;
        reader.array(8, count, 4)?;
        let table_end = 8 + count * 4;
        let offsets = (0..count)
            .map(|i| reader.u32(8 + i * 4).map(|at| at as usize))
            .collect::<Result<Vec<_>, _>>()?;
        if offsets.iter().any(|&at| {
            at < table_end || !at.is_multiple_of(4) || at > bytes.len().saturating_sub(24)
        }) {
            return Err(DecodeError::new(8, "MML model offset outside payload"));
        }
        let mut bounds = offsets.clone();
        bounds.push(bytes.len());
        bounds.sort_unstable();
        bounds.dedup();
        let models = offsets
            .iter()
            .map(|&at| {
                let end = bounds[bounds.partition_point(|&value| value <= at)];
                Mom::parse(&bytes[at..end])
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { offsets, models })
    }
}
