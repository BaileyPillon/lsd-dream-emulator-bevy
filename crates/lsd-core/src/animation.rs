/// Fields decoded by the original animation record's lead-word reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationWord {
    pub part_type: u8,
    pub kind: u8,
    pub flags: u8,
    pub word_count: u8,
}

impl AnimationWord {
    pub const fn decode(word: u32) -> Self {
        Self {
            part_type: word as u8,
            kind: ((word >> 16) & 0xF) as u8,
            flags: ((word >> 20) & 0xF) as u8,
            word_count: (word >> 24) as u8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationRecord {
    pub header: AnimationWord,
    pub payload: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationFrame {
    pub tag: u16,
    pub metadata: u32,
    pub records: Vec<AnimationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationClip {
    pub tag: u16,
    pub header_halfword: u16,
    pub frames: Vec<AnimationFrame>,
}
