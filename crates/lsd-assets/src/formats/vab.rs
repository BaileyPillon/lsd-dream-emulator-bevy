use super::reader::{DecodeError, Reader};

/// Version-seven VAB metadata. Program numbers are MIDI IDs, not packed indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VabHeader {
    pub version: u32,
    pub bank_id: i32,
    pub total_bytes: u32,
    pub program_count: u16,
    pub tone_count: u16,
    pub sample_count: u16,
    pub volume: u8,
    pub pan: u8,
    pub attributes: [u8; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VabTone {
    /// Includes reserved bytes so an importer does not discard original metadata.
    pub raw: [u8; 32],
}

impl VabTone {
    pub fn center_note(&self) -> u8 {
        self.raw[4]
    }
    pub fn pitch_correction(&self) -> u8 {
        self.raw[5]
    }
    pub fn adsr(&self) -> [u16; 2] {
        [
            u16::from_le_bytes([self.raw[16], self.raw[17]]),
            u16::from_le_bytes([self.raw[18], self.raw[19]]),
        ]
    }
    pub fn parent_program(&self) -> i16 {
        i16::from_le_bytes([self.raw[20], self.raw[21]])
    }
    /// VAB waveform numbers start at one; zero is the body prefix entry.
    pub fn sample_number(&self) -> i16 {
        i16::from_le_bytes([self.raw[22], self.raw[23]])
    }
    pub fn pitch(
        &self,
        table: &lsd_core::SpuPitchTable,
        note: i32,
        fine: i32,
    ) -> Result<u16, lsd_core::SpuPitchFault> {
        table.pitch(note, fine, self.center_note(), self.pitch_correction())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VabProgram {
    pub id: u8,
    pub raw: [u8; 16],
    pub tones: Vec<VabTone>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VabSample<'a> {
    pub number: u16,
    pub body_offset: usize,
    /// SPU ADPCM bytes, without a standalone VAG file header.
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vab<'a> {
    pub header: VabHeader,
    /// Includes inactive program attributes, which the original SDK can bind.
    pub program_attributes: Box<[[u8; 16]; 128]>,
    pub programs: Vec<VabProgram>,
    pub body_prefix: &'a [u8],
    pub samples: Vec<VabSample<'a>>,
}

impl<'a> Vab<'a> {
    /// Read the separate VH/VB layout used by the supplied Japanese game files.
    pub fn parse(header: &[u8], body: &'a [u8]) -> Result<Self, DecodeError> {
        let r = Reader(header);
        if r.bytes(0, 4)? != b"pBAV" {
            return Err(DecodeError::new(0, "invalid VAB signature"));
        }
        if r.u32(4)? != 7 {
            return Err(DecodeError::new(4, "unsupported VAB version"));
        }
        let info = VabHeader {
            version: r.u32(4)?,
            bank_id: r.i32(8)?,
            total_bytes: r.u32(12)?,
            program_count: r.u16(18)?,
            tone_count: r.u16(20)?,
            sample_count: r.u16(22)?,
            volume: r.u8(24)?,
            pan: r.u8(25)?,
            attributes: [r.u8(26)?, r.u8(27)?],
        };
        r.bytes(0, 32)?;
        if info.program_count > 128 || info.sample_count > 254 {
            return Err(DecodeError::new(18, "VAB count exceeds format capacity"));
        }
        let size = header.len().checked_add(body.len());
        if size != Some(info.total_bytes as usize) {
            return Err(DecodeError::new(
                12,
                "VAB total size does not match VH and VB",
            ));
        }
        r.array(32, 128, 16)?;
        let length_table = 2080 + usize::from(info.program_count) * 512;
        r.array(2080, usize::from(info.program_count), 512)?;
        r.array(length_table, 256, 2)?;
        if length_table + 512 != header.len() {
            return Err(DecodeError::new(
                length_table + 512,
                "trailing VAB header data",
            ));
        }
        let mut programs = Vec::new();
        let mut tone_count = 0;
        for id in 0..128 {
            let at = 32 + id * 16;
            let count = usize::from(r.u8(at)?);
            if count > 16 {
                return Err(DecodeError::new(at, "VAB program has more than 16 tones"));
            }
            if count == 0 {
                continue;
            }
            if programs.len() >= usize::from(info.program_count) {
                return Err(DecodeError::new(at, "too many active VAB programs"));
            }
            let mut tones = Vec::with_capacity(count);
            for index in 0..count {
                let tone_at = 2080 + programs.len() * 512 + index * 32;
                let tone = VabTone {
                    raw: r.bytes(tone_at, 32)?.try_into().unwrap(),
                };
                if tone.parent_program() != id as i16 {
                    return Err(DecodeError::new(tone_at + 20, "VAB tone parent mismatch"));
                }
                if !(1..=info.sample_count as i16).contains(&tone.sample_number()) {
                    return Err(DecodeError::new(
                        tone_at + 22,
                        "VAB tone sample out of range",
                    ));
                }
                tones.push(tone);
            }
            tone_count += count;
            programs.push(VabProgram {
                id: id as u8,
                raw: r.bytes(at, 16)?.try_into().unwrap(),
                tones,
            });
        }
        if programs.len() != usize::from(info.program_count)
            || tone_count != usize::from(info.tone_count)
        {
            return Err(DecodeError::new(
                18,
                "VAB active counts do not match header",
            ));
        }
        let body_reader = Reader(body);
        // Entry zero describes bytes preceding sample one, not sample one itself.
        let prefix_size = usize::from(r.u16(length_table)?) * 8;
        let body_prefix = body_reader.bytes(0, prefix_size)?;
        let mut offset = prefix_size;
        let mut samples = Vec::new();
        for number in 1..=info.sample_count {
            let size = usize::from(r.u16(length_table + usize::from(number) * 2)?) * 8;
            let data = body_reader.bytes(offset, size)?;
            samples.push(VabSample {
                number,
                body_offset: offset,
                data,
            });
            offset += size;
        }
        if offset != body.len() {
            return Err(DecodeError::new(
                offset,
                "VAB sample lengths do not fill body",
            ));
        }
        Ok(Self {
            header: info,
            program_attributes: Box::new(std::array::from_fn(|i| {
                r.bytes(32 + i * 16, 16).unwrap().try_into().unwrap()
            })),
            programs,
            body_prefix,
            samples,
        })
    }

    pub fn program(&self, id: u8) -> Option<&VabProgram> {
        self.programs.iter().find(|program| program.id == id)
    }

    pub fn sample(&self, number: u16) -> Option<&VabSample<'a>> {
        self.samples.get(usize::from(number.checked_sub(1)?))
    }

    pub fn tone_for_packed_note(&self, note: i32) -> Option<&VabTone> {
        let note = u16::try_from(note).ok()?;
        let program = u8::try_from(note >> 4).ok()?;
        self.program(program)?.tones.get(usize::from(note & 15))
    }

    pub fn prepare_voice_note(
        &self,
        bank: i16,
        program: u8,
        tone: usize,
        spu_base: u32,
    ) -> Result<lsd_core::VoiceNoteDefinition, DecodeError> {
        if !(0..16).contains(&bank) || tone >= 16 {
            return Err(DecodeError::new(0, "invalid SDK bank or tone index"));
        }
        let (packed, program) = self
            .programs
            .iter()
            .enumerate()
            .find(|(_, p)| p.id == program)
            .ok_or_else(|| DecodeError::new(0, "missing VAB program"))?;
        let attributes = program
            .tones
            .get(tone)
            .ok_or_else(|| DecodeError::new(0, "missing VAB tone"))?;
        let sample = self
            .sample(attributes.sample_number() as u16)
            .ok_or_else(|| DecodeError::new(0, "missing VAB waveform"))?;
        let body_bytes = self
            .samples
            .last()
            .map(|s| s.body_offset + s.data.len())
            .unwrap_or(self.body_prefix.len());
        let end = u32::try_from(body_bytes)
            .ok()
            .and_then(|size| spu_base.checked_add(size));
        if end.is_none_or(|end| end > 0x80000) {
            return Err(DecodeError::new(0, "VAB bank exceeds SPU RAM"));
        }
        let address = spu_base + sample.body_offset as u32;
        if address >= 0x80000 {
            return Err(DecodeError::new(0, "VAB sample starts outside SPU RAM"));
        }
        Ok(lsd_core::VoiceNoteDefinition {
            bank,
            program: i16::from(program.id),
            packed_program: packed as u8,
            tone: tone as i16,
            bank_volume: self.header.volume,
            program_volume: program.raw[1],
            program_pan: program.raw[4],
            attributes: attributes.raw,
            sample_address: (address >> 3) as u16,
        })
    }
}
