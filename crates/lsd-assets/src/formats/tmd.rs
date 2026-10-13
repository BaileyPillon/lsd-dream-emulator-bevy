use super::reader::{DecodeError, Reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector {
    pub xyz: [i16; 3],
    pub padding: i16,
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub output_words: u8,
    pub input_words: u8,
    pub flag: u8,
    pub mode: u8,
    pub vertices: Vec<u16>,
    pub normals: Vec<u16>,
    pub colors: Vec<[u8; 4]>,
    pub uv: Vec<[u8; 2]>,
    pub clut: Option<u16>,
    pub texture_page: Option<u16>,
}

impl Polygon {
    pub fn double_sided(&self) -> bool {
        self.flag & 2 != 0
    }
    pub fn semi_transparent(&self) -> bool {
        self.mode & 2 != 0
    }
    pub fn texture_raw(&self) -> bool {
        self.mode & 1 != 0
    }
    pub fn lighting_enabled(&self) -> bool {
        self.flag & 1 == 0
    }
    pub fn gouraud(&self) -> bool {
        self.mode & 0x10 != 0
    }

    fn parse(
        data: &[u8],
        offset: usize,
        vertex_count: usize,
        normal_count: usize,
    ) -> Result<(Self, usize), DecodeError> {
        let header = Reader(data);
        let output_words = header.u8(offset)?;
        let input_words = header.u8(offset + 1)?;
        let flag = header.u8(offset + 2)?;
        let mode = header.u8(offset + 3)?;
        if mode & 0xE0 != 0x20 || flag & !7 != 0 {
            return Err(DecodeError::new(
                offset,
                "unsupported TMD primitive family or flags",
            ));
        }
        let n = if mode & 8 != 0 { 4usize } else { 3 };
        let textured = mode & 4 != 0;
        let lit = flag & 1 == 0;
        let gouraud = mode & 0x10 != 0;
        let color_count = if textured && lit {
            0
        } else if flag & 4 != 0 || (!lit && gouraud) {
            n
        } else {
            1
        };
        let index_words = if lit && gouraud {
            n
        } else {
            (n + usize::from(lit)).div_ceil(2)
        };
        let expected = usize::from(textured) * n + color_count + index_words;
        if usize::from(input_words) != expected {
            return Err(DecodeError::new(
                offset,
                "TMD polygon payload length disagrees with its attributes",
            ));
        }
        let length = expected * 4;
        let payload = Reader(header.bytes(offset + 4, length)?);
        let mut cursor = 0;
        let mut uv = Vec::new();
        let (mut clut, mut texture_page) = (None, None);
        if textured {
            for i in 0..n {
                uv.push([payload.u8(cursor)?, payload.u8(cursor + 1)?]);
                if i == 0 {
                    clut = Some(payload.u16(cursor + 2)?);
                }
                if i == 1 {
                    texture_page = Some(payload.u16(cursor + 2)?);
                }
                cursor += 4;
            }
        }
        let mut colors = Vec::with_capacity(color_count);
        for _ in 0..color_count {
            colors.push(payload.bytes(cursor, 4)?.try_into().unwrap());
            cursor += 4;
        }
        let mut vertices = Vec::with_capacity(n);
        let mut normals = Vec::new();
        if lit && gouraud {
            for _ in 0..n {
                normals.push(payload.u16(cursor)?);
                vertices.push(payload.u16(cursor + 2)?);
                cursor += 4;
            }
        } else {
            if lit {
                normals.push(payload.u16(cursor)?);
                cursor += 2;
            }
            for _ in 0..n {
                vertices.push(payload.u16(cursor)?);
                cursor += 2;
            }
        }
        if vertices.iter().any(|&i| usize::from(i) >= vertex_count) {
            return Err(DecodeError::new(
                offset,
                "TMD polygon references missing vertex",
            ));
        }
        if normals.iter().any(|&i| usize::from(i) >= normal_count) {
            return Err(DecodeError::new(
                offset,
                "TMD polygon references missing normal",
            ));
        }
        Ok((
            Self {
                output_words,
                input_words,
                flag,
                mode,
                vertices,
                normals,
                colors,
                uv,
                clut,
                texture_page,
            },
            offset + 4 + length,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct TmdObject {
    pub vertices: Vec<Vector>,
    pub normals: Vec<Vector>,
    pub polygons: Vec<Polygon>,
    pub scale: i32,
}

#[derive(Debug, Clone)]
pub struct Tmd {
    pub objects: Vec<TmdObject>,
}

impl Tmd {
    pub fn parse(data: &[u8]) -> Result<Self, DecodeError> {
        let reader = Reader(data);
        if reader.u32(0)? != 0x41 {
            return Err(DecodeError::new(0, "bad TMD magic"));
        }
        if reader.u32(4)? != 0 {
            return Err(DecodeError::new(
                4,
                "mapped TMD requires RAM relocation context",
            ));
        }
        let count = reader.u32(8)? as usize;
        reader.array(12, count, 28)?;
        let descriptors_end = 12 + count * 28;
        let mut objects = Vec::with_capacity(count);
        for i in 0..count {
            let at = 12 + i * 28;
            let offset = |field: usize, size: usize| -> Result<usize, DecodeError> {
                let value = reader.u32(at + field)? as usize;
                let start = 12usize
                    .checked_add(value)
                    .ok_or_else(|| DecodeError::new(at, "TMD offset overflow"))?;
                if size != 0 && (start < descriptors_end || !start.is_multiple_of(4)) {
                    return Err(DecodeError::new(
                        at + field,
                        "TMD section overlaps descriptors or is unaligned",
                    ));
                }
                Ok(start)
            };
            let nv = reader.u32(at + 4)? as usize;
            let nn = reader.u32(at + 12)? as usize;
            let np = reader.u32(at + 20)? as usize;
            let vectors = |start: usize, count: usize| -> Result<Vec<Vector>, DecodeError> {
                reader.array(start, count, 8)?;
                (0..count)
                    .map(|j| {
                        let at = start + j * 8;
                        Ok(Vector {
                            xyz: [reader.i16(at)?, reader.i16(at + 2)?, reader.i16(at + 4)?],
                            padding: reader.i16(at + 6)?,
                        })
                    })
                    .collect()
            };
            let vertices = vectors(offset(0, nv)?, nv)?;
            let normals = vectors(offset(8, nn)?, nn)?;
            let mut cursor = offset(16, np)?;
            if np > data.len().saturating_sub(cursor) / 4 {
                return Err(DecodeError::new(
                    cursor,
                    "TMD primitive count exceeds extent",
                ));
            }
            let mut polygons = Vec::with_capacity(np);
            for _ in 0..np {
                let (polygon, next) = Polygon::parse(data, cursor, nv, nn)?;
                polygons.push(polygon);
                cursor = next;
            }
            objects.push(TmdObject {
                vertices,
                normals,
                polygons,
                scale: reader.i32(at + 24)?,
            });
        }
        Ok(Self { objects })
    }
}
