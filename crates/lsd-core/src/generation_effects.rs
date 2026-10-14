use crate::GenerationFault;

/// Original generation effect callback writes into the sound context. A
/// generation actor has five placement words followed by this 21-word context.
pub fn generation_effect(
    effect: i8,
    context: &mut [i32],
    distance: i32,
    range: i32,
    divisor: i32,
) -> Result<(), GenerationFault> {
    if context.len() < 21 {
        return Err(GenerationFault::InvalidContext);
    }
    let unit = range
        .checked_div(divisor)
        .ok_or(GenerationFault::InvalidDivision)?;
    context[4] = distance
        .checked_div(unit)
        .ok_or(GenerationFault::InvalidDivision)?;
    let tick = context[1];
    match effect {
        1 => match tick {
            0 => {
                context[7] = 7;
                context[8] = 0;
            }
            2 => {
                context[12] = 7;
                context[13] = 0;
            }
            5 => {
                context[17] = 7;
                context[18] = 0;
            }
            _ if tick >= 8 => context[1] = -1,
            _ => {}
        },
        2 => {
            if tick == 0 {
                context[7] = 24;
                context[8] = -2;
            } else if tick >= 1025 {
                context[1] = -1;
            }
        }
        3 => {
            if tick == 0 {
                context[7] = 12;
                context[8] = 2;
            } else if tick >= 5 {
                context[1] = -1;
            }
        }
        4 => {
            if tick % 20 == 0 {
                context[7] = 30;
                context[9] = 32;
                context[8] = 0;
                context[10] = 10;
            }
            if tick % 400 == 0 {
                context[12] = 30;
                context[13] = 0;
            }
            context[17] = 6;
            context[19] = 32;
            context[18] = 0;
            context[20] = 10;
        }
        5 => {
            if tick % 3 == 0 {
                context[7] = 30;
                context[8] = 0;
            }
            if tick % 5 == 0 {
                context[12] = 30;
                context[13] = 0;
                context[14] = 24;
                context[15] = 24;
            }
            if tick % 7 == 0 {
                context[7] = 30;
                context[8] = 0;
            }
            context[17] = 6;
            context[18] = 1;
            context[19] = 42;
            context[20] = 10;
        }
        6 => {
            if tick == 0 {
                context[7] = 30;
                context[8] = -1;
            } else if tick < 50 && (tick / 5).wrapping_mul(5) == tick.wrapping_sub(4) {
                context[12] = 30;
                context[13] = 0;
                let value = context[14].wrapping_sub(tick.wrapping_mul(2));
                context[14] = value;
                context[15] = value;
            } else if (tick.wrapping_sub(101) as u32) < 9 {
                context[17] = 13;
                context[18] = 1;
            } else if tick >= 201 {
                context[1] = -1;
            }
        }
        7 => {
            if tick == 0 {
                context[7] = 7;
                context[8] = 2;
            } else if tick >= 27 {
                context[1] = -1;
            }
        }
        8 => {
            if tick == 0 {
                context[7] = 20;
                context[8] = 1;
            } else if tick == 3 {
                context[8] = 2;
                context[9] = 24;
                context[7] = 3;
                context[10] = 20;
            } else if tick >= 51 {
                context[1] = -1;
            }
        }
        9 => {
            if tick % 20 == 0 {
                context[7] = 9;
                context[8] = 0;
                context[9] = 64;
                context[10] = 64;
            }
        }
        10 => {
            if tick % 20 == 0 {
                context[7] = 9;
                context[8] = -2;
            }
        }
        11 | 12 => {
            if tick % 20 == 1 {
                context[7] = 9;
                context[8] = -2;
            } else if tick % 20 == 16 {
                context[12] = 9;
                context[13] = -2;
            }
            if effect == 12 {
                let rem = tick % 70;
                if rem == 50 {
                    context[17] = 20;
                    context[18] = 1;
                } else if (rem.wrapping_sub(54) as u32) < 5 {
                    context[17] = 13;
                    context[18] = 1;
                } else if rem == 61 {
                    context[17] = 9;
                    context[18] = -1;
                }
            }
        }
        13 => {
            if tick == 0 {
                context[7] = 20;
                context[8] = -2;
                context[12] = 20;
                context[13] = -2;
            } else if tick == 4 {
                context[12] = 20;
                context[13] = -2;
            } else if tick == 20 {
                context[7] = 16;
                context[8] = -2;
                context[12] = 18;
                context[13] = -2;
            } else if tick >= 201 {
                context[1] = -1;
            }
        }
        14 => {
            if tick == 0 {
                context[7] = 24;
                context[8] = 0;
            }
        }
        _ => return Err(GenerationFault::InvalidEffect),
    }
    Ok(())
}
