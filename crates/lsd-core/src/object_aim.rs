//! Original SDK angle lookup and object-facing packet conversion.
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimFault {
    Division,
    Lookup,
    MissingActor,
}
impl fmt::Display for AimFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported original object aim: {self:?}")
    }
}
impl std::error::Error for AimFault {}

/// Imported from an identified executable, or supplied by an owned test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsxAimTables {
    pub atan: Box<[i16; 1025]>,
}
impl PsxAimTables {
    /// PsyQ ratan2 uses two ratio paths to avoid overflowing its ten-bit shift.
    pub fn atan2(&self, y: i32, x: i32) -> Result<i32, AimFault> {
        let negative_x = x < 0;
        let negative_y = y < 0;
        let x = if negative_x { x.wrapping_neg() } else { x };
        let y = if negative_y { y.wrapping_neg() } else { y };
        if x == 0 && y == 0 {
            return Ok(0);
        }
        let swapped = y >= x;
        let (small, large) = if swapped { (x, y) } else { (y, x) };
        let ratio = if small & 0x7fe0_0000 != 0 {
            small.checked_div(large >> 10)
        } else {
            small.wrapping_shl(10).checked_div(large)
        }
        .ok_or(AimFault::Division)?;
        let index = usize::try_from(ratio).map_err(|_| AimFault::Lookup)?;
        let mut angle = i32::from(*self.atan.get(index).ok_or(AimFault::Lookup)?);
        if swapped {
            angle = 1024_i32.wrapping_sub(angle);
        }
        if negative_x {
            angle = 2048_i32.wrapping_sub(angle);
        }
        if negative_y {
            angle = angle.wrapping_neg();
        }
        Ok(angle)
    }

    /// func_8001EACC measures local origin against cached target world position,
    /// converts to integer degrees, then uses the original angle setter packet.
    pub fn object_angles(
        &self,
        local_position: [i32; 3],
        target_world: [i32; 3],
        horizontal: bool,
        face_forward: bool,
    ) -> Result<[i16; 3], AimFault> {
        let [x, y, z] = std::array::from_fn(|i| target_world[i].wrapping_sub(local_position[i]));
        let yaw = self.atan2(if x == 0 { 1 } else { x }, z)?;
        let pitch = self.atan2(if z == 0 { 1 } else { z }, y)?;
        let mut pitch = ((pitch.wrapping_add(1024).wrapping_mul(360)) / 4096) as i16;
        let mut yaw = (yaw.wrapping_mul(360) / 4096) as i16;
        if horizontal {
            pitch = 0;
        }
        if !face_forward {
            yaw = yaw.wrapping_add(180);
        }
        Ok([pitch, yaw, 0].map(|v| (i32::from(v).wrapping_shl(12) / 360) as i16))
    }
}
