use crate::transform_number;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionAxis {
    Forward,
    Strafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionRequest {
    pub axis: MotionAxis,
    pub amount: i32,
    pub collision_and_links: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorEffectStep {
    Apply,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementStep {
    pub floor_effect: FloorEffectStep,
    pub motion: Option<MotionRequest>,
}

/// Original pad-driven view and movement state. World collision/link effects are executed by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerControls {
    pub paused: i32,
    pub input_locked: bool,
    pub auto_move_locked: bool,
    pub auto_move_active: bool,
    pub action_pressed: bool,
    pub look_state: i32,
    pub look_vertical: i32,
    pub turn_state: i32,
    pub turn_offset: i32,
    pub left_right: i32,
    pub is_forward: bool,
    pub move_state: i32,
    pub move_direction: i32,
    pub previous_move_direction: i32,
    pub move_step: i32,
    pub angles: [i16; 3],
    pub eye: [i32; 3],
    pub target: [i32; 3],
    pub screen_shake: bool,
}

impl Default for PlayerControls {
    fn default() -> Self {
        Self {
            paused: 0,
            input_locked: false,
            auto_move_locked: false,
            auto_move_active: false,
            action_pressed: false,
            look_state: 0,
            look_vertical: 0,
            turn_state: 0,
            turn_offset: 0,
            left_right: 0,
            is_forward: false,
            move_state: 0,
            move_direction: 1,
            previous_move_direction: 1,
            move_step: 0,
            angles: [0; 3],
            eye: [0; 3],
            target: [0, 0, 1024],
            screen_shake: true,
        }
    }
}

impl PlayerControls {
    pub fn apply_rotation_packet(
        &mut self,
        absolute: bool,
        packet: [u32; 3],
    ) -> Result<(), crate::LinkError> {
        let mut values = [0; 3];
        for (value, word) in values.iter_mut().zip(packet) {
            *value = transform_number(word).ok_or(crate::LinkError::InvalidMovement)? / 360;
        }
        for (angle, value) in self.angles.iter_mut().zip(values) {
            *angle = if absolute {
                value as i16
            } else {
                (i32::from(*angle).wrapping_add(value) % 4096) as i16
            };
        }
        Ok(())
    }

    pub fn notify_pad(&mut self, code: i32) {
        if self.paused != 0 || self.input_locked || self.auto_move_locked {
            return;
        }
        match code.wrapping_sub(2) {
            0 => self.move_state = 1,
            1 => self.move_state = 2,
            2 => self.left_right = 1,
            3 => self.left_right = 2,
            4 => self.look_state = 1,
            5 if self.move_state == 1 => {
                if self.move_direction != 4 {
                    self.previous_move_direction = self.move_direction;
                    self.move_direction = 4;
                }
            }
            6 => self.look_state = 2,
            11 => self.turn_state = 2,
            12 => self.move_state = 4,
            13 => self.turn_state = 1,
            14 => self.move_state = 3,
            23 => self.action_pressed = true,
            32 => self.move_direction = self.previous_move_direction,
            _ => {}
        }
    }

    fn rotate_y(&mut self, degrees: i16) {
        let value = transform_number((1 << 16) | u32::from(degrees as u16)).unwrap() / 360;
        self.angles[1] = (i32::from(self.angles[1]).wrapping_add(value) % 4096) as i16;
    }

    pub fn update_look(&mut self) {
        if self.look_state != 0 {
            let delta = if self.look_state == 1 { -600 } else { 600 };
            let sum = self.look_vertical.wrapping_add(delta);
            if (if sum < 0 { sum.wrapping_neg() } else { sum }) < 9000 {
                self.target[1] = self.target[1].wrapping_add(delta);
                self.look_vertical = sum;
            }
            self.look_state = 0;
        } else if self.look_vertical != 0 {
            let delta = if self.look_vertical < 0 { 600 } else { -600 };
            self.target[1] = self.target[1].wrapping_add(delta);
            self.look_vertical = self.look_vertical.wrapping_add(delta);
        }
    }

    fn decay_turn(&mut self) {
        self.is_forward = false;
        if self.move_state != 0 {
            self.move_state = if self.move_state & 1 == 0 {
                self.move_state.wrapping_sub(1)
            } else {
                self.move_state.wrapping_add(1)
            };
        }
    }

    pub fn update_turn(&mut self) {
        self.is_forward = self.move_state == 1;
        if self.turn_state != 0 {
            let delta = if self.turn_state == 1 { -45 } else { 45 };
            let sum = self.turn_offset.wrapping_add(delta);
            if (if sum < 0 { sum.wrapping_neg() } else { sum }) < 181 {
                self.rotate_y(delta as i16);
                self.turn_offset = sum;
            }
            self.turn_state = 0;
            self.decay_turn();
        } else if self.turn_offset != 0 {
            let delta = if self.turn_offset < 0 { 45 } else { -45 };
            self.rotate_y(delta as i16);
            self.turn_offset = self.turn_offset.wrapping_add(delta);
            self.decay_turn();
        }
    }

    pub fn update_view(&mut self) {
        self.update_look();
        self.update_turn();
    }

    pub fn process_move(&mut self, shake: bool) -> (i32, FloorEffectStep) {
        let saved = self.move_state;
        let mut apply = false;
        if saved != 0 {
            self.move_step = self.move_step.wrapping_add(1);
            if self.move_step >= 4 {
                self.move_state = 0;
                apply = true;
            } else if self.move_direction == 4 {
                apply = self.move_step & 1 == 0;
            }
            if self.screen_shake && shake {
                let delta = if self.move_step >= 3 { 50 } else { -50 };
                self.eye[1] = self.eye[1].wrapping_add(delta);
                self.target[1] = self.target[1].wrapping_add(delta);
            }
            if self.move_state == 0 {
                self.move_step = 0;
            }
        }
        (
            saved,
            if apply {
                FloorEffectStep::Apply
            } else {
                FloorEffectStep::Release
            },
        )
    }

    pub fn update_movement(&mut self) -> Option<MovementStep> {
        if self.paused == 0 {
            if self.left_right != 0 {
                self.rotate_y(if self.left_right == 1 { -6 } else { 6 });
                self.left_right = 0;
            }
            if self.input_locked {
                return None;
            }
        } else {
            self.move_state = 1;
            if self.paused == 2 {
                return None;
            }
        }
        let (saved, floor_effect) = self.process_move(!self.input_locked);
        let motion = if saved != 0 && !self.input_locked {
            let speeds = [0i32, 24, 64, 128, 384];
            let amount =
                speeds[self.move_direction as usize] * if matches!(saved, 2 | 3) { -1 } else { 1 };
            Some(MotionRequest {
                axis: if saved <= 2 {
                    MotionAxis::Forward
                } else {
                    MotionAxis::Strafe
                },
                amount,
                collision_and_links: !self.auto_move_active,
            })
        } else {
            None
        };
        Some(MovementStep {
            floor_effect,
            motion,
        })
    }
}
