use crate::{CellRectangle, SceneGridError, SceneLocation, SceneQuery, SceneSlots};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyNotification {
    Query,
    Contact,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyContactState<T> {
    pub current: Option<T>,
}
impl<T> BodyContactState<T> {
    pub fn notify(&mut self, sender: T, code: i32) -> BodyNotification {
        match code {
            2 | 3 => BodyNotification::Query,
            4 => {
                self.current = Some(sender);
                BodyNotification::Contact
            }
            _ => BodyNotification::Ignored,
        }
    }
}

impl SceneSlots {
    /// Body traversal uses ascending rows and can split across the staggered neighbors.
    pub fn body_queries(&self, location: SceneLocation) -> Result<Vec<SceneQuery>, SceneGridError> {
        self.validate()?;
        let [cell_x, cell_z] = location.cell.map(i32::from);
        if self.grid.mode != 0 {
            let slot = self
                .slots
                .iter()
                .position(|slot| slot.map_index == location.map_index && slot.ready)
                .ok_or(SceneGridError::InvalidSlot)?;
            return Ok(vec![SceneQuery {
                slot,
                cells: CellRectangle {
                    x: cell_x.saturating_sub(1).max(0) as i16,
                    z: cell_z.saturating_sub(1).max(0) as i16,
                    columns: 3 - i32::from(cell_x == 0) - i32::from(cell_x == 19),
                    rows: 3 - i32::from(cell_z == 0) - i32::from(cell_z == 19),
                },
            }]);
        }
        let label_slot = |label: i16| {
            self.slots
                .iter()
                .position(|slot| slot.label == label)
                .ok_or(SceneGridError::MissingSlot(label))
        };
        let mut x = cell_x - 1;
        let mut z = cell_z - 1;
        let mut mode = 3i16;
        let mut wrapped_x = false;
        if x < 0 {
            x += 20;
            wrapped_x = true;
            mode = 2;
        }
        if z < 0 {
            z += 20;
            mode = 0;
            if wrapped_x {
                x -= 10;
            } else if x < 10 {
                x += 10;
            } else {
                x -= 10;
                mode = 1;
            }
        }
        let x_overflow = (x + 3 - 20).max(0);
        let z_overflow = (z + 3 - 20).max(0);
        let mut result = vec![SceneQuery {
            slot: label_slot(mode)?,
            cells: CellRectangle {
                x: x.max(0) as i16,
                z: z as i16,
                columns: 3 - x_overflow,
                rows: 3 - z_overflow,
            },
        }];
        if z_overflow > 0 {
            let next_mode = mode + if x < 10 { 2 } else { 3 };
            let next_x = x + if x < 10 { 10 } else { -10 };
            let overflow = (next_x + 3 - 20).max(0);
            result.push(SceneQuery {
                slot: label_slot(next_mode)?,
                cells: CellRectangle {
                    x: next_x as i16,
                    z: 0,
                    columns: 3 - overflow,
                    rows: z_overflow,
                },
            });
            if overflow > 0 {
                result.push(SceneQuery {
                    slot: label_slot(next_mode + 1)?,
                    cells: CellRectangle {
                        x: 0,
                        z: 0,
                        columns: overflow,
                        rows: z_overflow,
                    },
                });
            }
        }
        if x_overflow > 0 {
            result.push(SceneQuery {
                slot: label_slot(mode + 1)?,
                cells: CellRectangle {
                    x: 0,
                    z: z as i16,
                    columns: x_overflow,
                    rows: 3 - z_overflow,
                },
            });
        }
        Ok(result)
    }
}
