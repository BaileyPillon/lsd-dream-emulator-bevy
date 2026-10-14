use crate::{
    AnimationClip, AnimationError, AnimationPlayer, BodyContactState, BodyPeer, FixedMatrix,
    GenerationFault, GenerationPose, transform_number,
};
/// The original node contact slot contains either a notified peer or a floor tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorContact {
    Peer(BodyPeer),
    Floor(crate::SceneContact),
}
impl ActorContact {
    pub fn peer(self) -> Option<BodyPeer> {
        match self {
            Self::Peer(peer) => Some(peer),
            Self::Floor(_) => None,
        }
    }
}
impl From<BodyPeer> for ActorContact {
    fn from(peer: BodyPeer) -> Self {
        Self::Peer(peer)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAxis {
    X,
    Y,
    Z,
}
impl ActorAxis {
    pub fn notification(self) -> i32 {
        match self {
            Self::X => 7,
            Self::Y => 8,
            Self::Z => 6,
        }
    }
    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelActor {
    pub animation: AnimationPlayer,
    pub root_part: usize,
    /// The actor copies the root part's geometry binding at construction.
    pub collision_primitive: Option<usize>,
    pub pose: GenerationPose,
    pub attributes: u32,
    pub action_kind: u8,
    pub collision_mode: i32,
    /// Signed extension margin at original scene-node offset 0x54.
    pub collision_margin: i32,
    /// Signed halfword at original node offset 0x48, initialized to 300.
    pub move_distance: i16,
    pub body_contact: BodyContactState<ActorContact>,
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
        let collision_primitive = animation.parts[root].primitive;
        Ok(Self {
            animation,
            root_part: root,
            collision_primitive,
            pose: GenerationPose::default(),
            attributes: 1 << 31,
            action_kind: 65,
            collision_mode: 1,
            collision_margin: 0,
            move_distance: 300,
            body_contact: BodyContactState { current: None },
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
    /// Original scene movement truncates the amount before rotating its local vector.
    pub fn move_local(&mut self, axis: ActorAxis, amount: i32, matrix: FixedMatrix) {
        self.move_distance = amount as i16;
        let mut vector = [0; 3];
        vector[axis.index()] = i32::from(self.move_distance);
        self.translate(matrix.apply_long(vector), false);
    }
    pub fn translate(&mut self, vector: [i32; 3], absolute: bool) {
        for (position, value) in self.pose.position.iter_mut().zip(vector) {
            *position = if absolute {
                value
            } else {
                position.wrapping_add(value)
            };
        }
        self.animation.actor_coordinate_cache = 0;
    }
    pub fn rotate_packet(
        &mut self,
        packet: [u32; 3],
        absolute: bool,
    ) -> Result<(), GenerationFault> {
        let mut values = [0; 3];
        for (value, word) in values.iter_mut().zip(packet) {
            *value = transform_number(word).ok_or(GenerationFault::InvalidDivision)? / 360;
        }
        for (angle, value) in self.pose.angles.iter_mut().zip(values) {
            *angle = if absolute {
                value as i16
            } else {
                (i32::from(*angle).wrapping_add(value) % 4096) as i16
            };
        }
        self.animation.actor_coordinate_cache = 0;
        Ok(())
    }
    pub fn scale_packet(
        &mut self,
        packet: [u32; 3],
        absolute: bool,
    ) -> Result<(), GenerationFault> {
        let mut values = [0; 3];
        for (value, word) in values.iter_mut().zip(packet) {
            *value =
                i32::from(transform_number(word).ok_or(GenerationFault::InvalidDivision)? as i16);
        }
        for (scale, value) in self.pose.scale.iter_mut().zip(values) {
            *scale = if absolute {
                value
            } else {
                scale.wrapping_add(value)
            };
        }
        self.animation.actor_coordinate_cache = 0;
        Ok(())
    }
}
