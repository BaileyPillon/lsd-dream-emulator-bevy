use crate::{AnimationClip, AnimationError, AnimationPlayer, GenerationPose};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelActor {
    pub animation: AnimationPlayer,
    pub root_part: usize,
    pub pose: GenerationPose,
    pub attributes: u32,
    pub action_kind: u8,
    pub collision_mode: i32,
    pub turn_limit: i32,
}
impl ModelActor {
    /// Successful original actor construction allocates parts, finds the root
    /// using the first frame's primitive selector, then selects clip zero.
    pub fn construct(
        clips: &[AnimationClip],
        types: &[u8],
        primitives: usize,
    ) -> Result<Self, AnimationError> {
        let frame = clips
            .first()
            .and_then(|v| v.frames.first())
            .ok_or(AnimationError::EmptyClip)?;
        if types.is_empty() {
            return Err(AnimationError::EmptyClip);
        }
        let selector = frame.records.iter().filter(|v| v.header.kind == 2).count() as u16;
        let mut written = 0usize;
        let mut root = 0;
        for record in &frame.records {
            if record.header.kind == 8 && record.header.flags == 0 {
                written += 1;
            } else if record.header.kind == 2
                && record.payload.first().copied().unwrap_or(0) as u16 == selector
                && let Some(index) = types
                    .iter()
                    .take(written)
                    .position(|&v| v == record.header.part_type)
            {
                root = index;
            }
        }
        let mut animation = AnimationPlayer::new(types, primitives);
        animation.select_clip(clips, 0)?;
        Ok(Self {
            animation,
            root_part: root,
            pose: GenerationPose::default(),
            attributes: 1 << 31,
            action_kind: 65,
            collision_mode: 1,
            turn_limit: 300,
        })
    }
    pub fn set_hidden(&mut self, hidden: bool) {
        for part in &mut self.animation.parts {
            if hidden {
                part.attributes |= 1 << 31;
            } else {
                part.attributes &= !(1 << 31);
            }
        }
    }
    pub fn tick_animation(&mut self, clips: &[AnimationClip]) -> Result<(), AnimationError> {
        self.animation.tick(clips, |_| {})
    }
}
