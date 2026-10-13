use crate::{AnimationClip, AnimationRecord};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationParent {
    Actor,
    Part(usize),
}

/// Logical fields consumed by the original actor animation interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationPart {
    pub part_type: u8,
    pub attributes: u32,
    pub coordinate_cache: i32,
    pub angles: [i16; 3],
    pub scale: [i32; 3],
    pub translation: [i32; 3],
    /// The coordinate's translation is separate from the motion state's position.
    pub coordinate_translation: [i32; 3],
    pub primitive: Option<usize>,
    pub parent: Option<AnimationParent>,
    pub children: Vec<usize>,
}

impl AnimationPart {
    pub fn new(part_type: u8) -> Self {
        Self {
            part_type,
            attributes: 0,
            coordinate_cache: 1,
            angles: [0; 3],
            scale: [4096; 3],
            translation: [0; 3],
            coordinate_translation: [0; 3],
            primitive: None,
            parent: None,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationError {
    InvalidClip,
    EmptyClip,
    InvalidRecord,
    MissingPrimitive,
    MissingParent,
}

impl fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "animation error: {self:?}")
    }
}
impl std::error::Error for AnimationError {}

/// Integer animation state; rendering, allocation failure and NPC actions are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationPlayer {
    pub parts: Vec<AnimationPart>,
    pub actor_children: Vec<usize>,
    pub primitive_count: usize,
    pub ticks: u32,
    pub actor_coordinate_cache: i32,
    pub enabled: bool,
    pub action_enabled: bool,
    pub selected_clip: Option<usize>,
    pub frame_index: usize,
}

impl AnimationPlayer {
    pub fn new(part_types: &[u8], primitive_count: usize) -> Self {
        Self {
            parts: part_types.iter().copied().map(AnimationPart::new).collect(),
            actor_children: Vec::new(),
            primitive_count,
            ticks: 0,
            actor_coordinate_cache: 1,
            enabled: false,
            action_enabled: false,
            selected_clip: None,
            frame_index: 0,
        }
    }

    fn part_index(&self, part_type: u8) -> Option<usize> {
        self.parts
            .iter()
            .position(|part| part.part_type == part_type)
    }

    pub fn apply_record(&mut self, record: &AnimationRecord) -> Result<(), AnimationError> {
        let header = record.header;
        if usize::from(header.word_count) != record.payload.len() + 1 {
            return Err(AnimationError::InvalidRecord);
        }
        let Some(index) = self.part_index(header.part_type) else {
            return Ok(());
        };
        let minimum = match header.kind {
            0 => 2,
            2 | 3 => 1,
            1 => {
                usize::from(header.flags & 2 != 0) * 3
                    + usize::from(header.flags & 4 != 0) * 2
                    + usize::from(header.flags & 8 != 0) * 3
            }
            _ => 0,
        };
        if record.payload.len() < minimum {
            return Err(AnimationError::InvalidRecord);
        }
        self.parts[index].coordinate_cache = 0;
        match header.kind {
            0 => {
                self.parts[index].attributes =
                    (self.parts[index].attributes & record.payload[0]) | record.payload[1];
            }
            1 => {
                let part = &mut self.parts[index];
                let relative = header.flags & 1 != 0;
                let mut cursor = 0;
                if header.flags & 2 != 0 {
                    for (angle, &value) in part
                        .angles
                        .iter_mut()
                        .zip(&record.payload[cursor..cursor + 3])
                    {
                        let amount = (value as i32) / 360;
                        *angle = if relative {
                            // The target stores a halfword before its signed remainder.
                            let sum = (*angle as u16).wrapping_add(amount as u16) as i16;
                            sum % 4096
                        } else {
                            amount as i16
                        };
                    }
                    cursor += 3;
                }
                if header.flags & 4 != 0 {
                    let values = [
                        record.payload[cursor] as i16,
                        (record.payload[cursor] >> 16) as i16,
                        record.payload[cursor + 1] as i16,
                    ];
                    for (scale, value) in part.scale.iter_mut().zip(values) {
                        *scale = if relative {
                            scale.wrapping_mul(i32::from(value)) / 4096
                        } else {
                            i32::from(value)
                        };
                    }
                    cursor += 2;
                }
                if header.flags & 8 != 0 {
                    for (position, &value) in part
                        .translation
                        .iter_mut()
                        .zip(&record.payload[cursor..cursor + 3])
                    {
                        *position = if relative {
                            position.wrapping_add(value as i32)
                        } else {
                            value as i32
                        };
                    }
                    part.coordinate_translation = part.translation;
                }
            }
            2 => {
                let selection = usize::from(record.payload[0] as u16);
                if selection != 0 && self.parts[index].primitive.is_none() {
                    if selection > self.primitive_count {
                        return Err(AnimationError::MissingPrimitive);
                    }
                    self.parts[index].primitive = Some(selection - 1);
                }
            }
            3 => {
                let value = record.payload[0];
                let parent = if value == 0 || value == 0xFFFF {
                    AnimationParent::Actor
                } else {
                    AnimationParent::Part(
                        self.part_index(value as u8)
                            .ok_or(AnimationError::MissingParent)?,
                    )
                };
                if self.parts[index].parent.is_none() {
                    self.parts[index].parent = Some(parent);
                    self.parts[index].coordinate_translation = [0; 3];
                    match parent {
                        AnimationParent::Actor => self.actor_children.insert(0, index),
                        AnimationParent::Part(owner) => self.parts[owner].children.insert(0, index),
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn select_clip(
        &mut self,
        clips: &[AnimationClip],
        index: usize,
    ) -> Result<(), AnimationError> {
        let clip = clips.get(index).ok_or(AnimationError::InvalidClip)?;
        let frame = clip.frames.first().ok_or(AnimationError::EmptyClip)?;
        self.selected_clip = Some(index);
        self.frame_index = 0;
        for record in &frame.records {
            self.apply_record(record)?;
        }
        // Selection deliberately leaves the cursor at frame zero.
        Ok(())
    }

    /// The original action callback runs after timer increment and before animation.
    pub fn tick(
        &mut self,
        clips: &[AnimationClip],
        action: impl FnOnce(&mut Self),
    ) -> Result<(), AnimationError> {
        self.ticks = self.ticks.wrapping_add(1);
        if self.action_enabled {
            action(self);
        }
        if self.enabled
            && let Some(index) = self.selected_clip
        {
            let clip = clips.get(index).ok_or(AnimationError::InvalidClip)?;
            if clip.frames.len() >= 2 {
                let frame = clip
                    .frames
                    .get(self.frame_index)
                    .ok_or(AnimationError::InvalidClip)?;
                for record in &frame.records {
                    self.apply_record(record)?;
                }
                self.frame_index += 1;
                if self.frame_index >= clip.frames.len() {
                    self.frame_index = 0;
                }
            }
        }
        self.actor_coordinate_cache = 0;
        Ok(())
    }
}
