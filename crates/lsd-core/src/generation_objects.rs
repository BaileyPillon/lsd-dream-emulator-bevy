use crate::{GenerationFault, GenerationSpawn, PsxRng, transform_number};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationObjectTables {
    pub model_indices: [u32; 2],
    pub palette_coordinates: [i16; 2],
    pub child_origin: [i32; 3],
    pub child_spacing: [i32; 5],
    pub child_motion: [i32; 7],
    pub motion_translation: [i32; 3],
    pub motion_rotation: [u32; 3],
    pub sprite_x_offsets: [i32; 6],
    pub sprite_scale_even: [u32; 3],
    pub sprite_scale_odd: [u32; 3],
    pub sprite_scale_low: [u32; 3],
    pub sprite_translation: [i32; 3],
    pub sprite_random_scales: [[u32; 3]; 6],
    pub sprite_rectangles: [[u16; 6]; 2],
    pub sprite_cluts: [[i16; 2]; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationPose {
    pub position: [i32; 3],
    pub angles: [i16; 3],
    pub scale: [i32; 3],
}
impl Default for GenerationPose {
    fn default() -> Self {
        Self {
            position: [0; 3],
            angles: [0; 3],
            scale: [4096; 3],
        }
    }
}
impl GenerationPose {
    fn rotate(&mut self, packet: [u32; 3], absolute: bool) -> Result<(), GenerationFault> {
        for (angle, word) in self.angles.iter_mut().zip(packet) {
            let value = transform_number(word).ok_or(GenerationFault::InvalidDivision)? / 360;
            *angle = if absolute {
                value as i16
            } else {
                (i32::from(*angle).wrapping_add(value) % 4096) as i16
            };
        }
        Ok(())
    }
    fn scale(&mut self, packet: [u32; 3]) -> Result<(), GenerationFault> {
        for (scale, word) in self.scale.iter_mut().zip(packet) {
            *scale =
                i32::from(transform_number(word).ok_or(GenerationFault::InvalidDivision)? as i16);
        }
        Ok(())
    }
    fn translate(&mut self, value: [i32; 3]) {
        for (position, delta) in self.position.iter_mut().zip(value) {
            *position = position.wrapping_add(delta);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationChild {
    Model {
        pose: GenerationPose,
        model_index: u32,
    },
    Sprite(GenerationSprite),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationSprite {
    pub pose: GenerationPose,
    pub sprite_id: usize,
    pub color: [u8; 3],
    pub flags: u32,
    pub scale: [i16; 2],
    pub rotation: i32,
    pub rectangle: [u16; 6],
    pub clut: [i16; 2],
}
impl GenerationSprite {
    fn scale(&mut self, packet: [u32; 3]) -> Result<(), GenerationFault> {
        self.scale = [
            transform_number(packet[0]).ok_or(GenerationFault::InvalidDivision)? as i16,
            transform_number(packet[1]).ok_or(GenerationFault::InvalidDivision)? as i16,
        ];
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationStructure {
    pub mode: i32,
    pub spawn: GenerationSpawn,
    pub pose: GenerationPose,
    pub model_index: Option<u32>,
    pub children: Vec<GenerationChild>,
    pub tick: u32,
}
impl GenerationStructure {
    pub fn construct(
        mode: i32,
        spawn: GenerationSpawn,
        tables: &GenerationObjectTables,
        base: [i32; 3],
        rng: &mut PsxRng,
    ) -> Result<Self, GenerationFault> {
        if !(0..=3).contains(&mode) {
            return Err(GenerationFault::InvalidEntry);
        }
        let mut pose = GenerationPose {
            position: add(base, spawn.position),
            ..Default::default()
        };
        pose.rotate(spawn.rotation, true)?;
        pose.scale(spawn.scale)?;
        let model_index = (mode < 2).then(|| tables.model_indices[mode as usize]);
        let mut object = Self {
            mode,
            spawn,
            pose,
            model_index,
            children: Vec::new(),
            tick: 0,
        };
        if mode == 0 && spawn.index != 0 {
            for position in object.child_positions(tables)? {
                let mut child = GenerationPose {
                    position,
                    ..Default::default()
                };
                child.rotate(spawn.rotation, true)?;
                child.scale(spawn.scale)?;
                object.children.push(GenerationChild::Model {
                    pose: child,
                    model_index: model_index.unwrap(),
                });
            }
        } else if mode >= 2 {
            let parity = if mode == 2 {
                Some(rng.next_u15() % 2)
            } else {
                None
            };
            for _ in 0..5 {
                let mut sprite = GenerationSprite {
                    pose: Default::default(),
                    sprite_id: 0,
                    color: spawn.color,
                    flags: 0,
                    scale: [4096; 2],
                    rotation: 0,
                    rectangle: tables.sprite_rectangles[0],
                    clut: tables.sprite_cluts[0],
                };
                if parity == Some(0) {
                    sprite.scale(tables.sprite_scale_even)?;
                }
                object.children.push(GenerationChild::Sprite(sprite));
            }
            if let Some(parity) = parity {
                let GenerationChild::Sprite(sprite) = &mut object.children[1] else {
                    unreachable!()
                };
                if spawn.random >= 2 {
                    let x = *usize::try_from(spawn.random)
                        .ok()
                        .and_then(|i| tables.sprite_x_offsets.get(i))
                        .ok_or(GenerationFault::InvalidEntry)?;
                    let mut translation = tables.sprite_translation;
                    translation[0] = x;
                    sprite.pose.translate(translation);
                    sprite.color = spawn.alternate_color.unwrap_or(spawn.color);
                } else {
                    sprite.flags |= 1 << 30;
                    sprite.flags &= !(3 << 28);
                    sprite.scale(if parity != 0 {
                        tables.sprite_scale_odd
                    } else {
                        tables.sprite_scale_low
                    })?;
                }
                let GenerationChild::Sprite(sprite) = &mut object.children[2] else {
                    unreachable!()
                };
                sprite.flags |= 1 << 31;
            }
        }
        Ok(object)
    }
    fn child_positions(
        &self,
        tables: &GenerationObjectTables,
    ) -> Result<[[i32; 3]; 2], GenerationFault> {
        let spacing = *usize::try_from(self.spawn.index)
            .ok()
            .and_then(|i| tables.child_spacing.get(i))
            .ok_or(GenerationFault::InvalidEntry)?;
        let mut position = tables.child_origin;
        let delta = if self.spawn.index < 3 {
            i32::from(self.spawn.scale[0] as i16).wrapping_mul(spacing)
        } else {
            spacing
        };
        Ok(std::array::from_fn(|_| {
            let axis = usize::from(self.spawn.index >= 3);
            position[axis] = position[axis].wrapping_add(delta);
            position
        }))
    }
    pub fn update(
        &mut self,
        tables: &GenerationObjectTables,
        base: [i32; 3],
        view_height_delta: i32,
        rng: &mut PsxRng,
    ) -> Result<(), GenerationFault> {
        self.tick = self.tick.wrapping_add(1);
        self.pose.position = add(base, self.spawn.position);
        self.pose.position[1] = self.pose.position[1].wrapping_add(view_height_delta);
        match self.mode {
            0 if self.spawn.index != 0 => {
                let motion = *usize::try_from(self.spawn.random)
                    .ok()
                    .and_then(|i| tables.child_motion.get(i))
                    .ok_or(GenerationFault::InvalidEntry)?;
                if motion != 0 && self.tick >= 501 {
                    self.pose.rotate(tables.motion_rotation, false)?;
                    for (i, child) in self.children.iter_mut().enumerate() {
                        let GenerationChild::Model { pose, .. } = child else {
                            return Err(GenerationFault::InvalidEntry);
                        };
                        let mut delta = tables.motion_translation;
                        delta[2] =
                            delta[2].wrapping_add((i as i32).wrapping_mul(3).wrapping_add(motion));
                        pose.translate(delta);
                        pose.rotate(tables.motion_rotation, false)?;
                    }
                    let count = 0x5fb4_i32
                        .checked_div(motion)
                        .ok_or(GenerationFault::InvalidDivision)?
                        .unsigned_abs();
                    if count == 0 {
                        return Err(GenerationFault::InvalidDivision);
                    }
                    if self.tick.is_multiple_of(count) {
                        let positions = self.child_positions(tables)?;
                        for (child, position) in self.children.iter_mut().zip(positions) {
                            let GenerationChild::Model { pose, .. } = child else {
                                unreachable!()
                            };
                            pose.position = position;
                        }
                    }
                }
            }
            3 => {
                for child in self.children.iter_mut().skip(1).take(4) {
                    let GenerationChild::Sprite(sprite) = child else {
                        return Err(GenerationFault::InvalidEntry);
                    };
                    sprite.scale(tables.sprite_random_scales[(rng.next_u15() % 6) as usize])?;
                    sprite.rotation = i32::from(rng.next_u15() % 360) << 12;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
fn add(left: [i32; 3], right: [i32; 3]) -> [i32; 3] {
    std::array::from_fn(|i| left[i].wrapping_add(right[i]))
}
