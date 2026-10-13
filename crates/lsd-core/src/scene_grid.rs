use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneGrid {
    pub columns: i16,
    pub rows: i16,
    pub mode: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenePlacement {
    pub index: i32,
    pub position: [i32; 3],
    pub center: [i32; 3],
}

impl SceneGrid {
    pub fn place(
        self,
        base: [i32; 3],
        grid_index: [i8; 2],
        cell: [i8; 2],
        cell_offset: [i16; 3],
    ) -> ScenePlacement {
        let row = if self.mode != 0 {
            0
        } else {
            i32::from(grid_index[1])
        };
        let rows = if self.mode != 0 {
            1
        } else {
            i32::from(self.rows)
        };
        let index = if self.mode != 0 {
            0
        } else {
            i32::from(grid_index[0]).wrapping_add(i32::from(self.columns).wrapping_mul(row))
        };
        let mut origin = [
            base[0]
                .wrapping_sub(i32::from(self.columns).wrapping_mul(20480))
                .wrapping_add(i32::from(grid_index[0]).wrapping_mul(40960)),
            base[1],
            base[2]
                .wrapping_sub(rows.wrapping_mul(20480))
                .wrapping_add(row.wrapping_mul(40960)),
        ];
        if row & 1 != 0 {
            origin[0] = origin[0].wrapping_sub(20480);
        }
        let position = [
            origin[0]
                .wrapping_add(i32::from(cell[0]).wrapping_mul(2048))
                .wrapping_add(i32::from(cell_offset[0]).wrapping_add(1024)),
            origin[1].wrapping_add(i32::from(cell_offset[1])),
            origin[2]
                .wrapping_add(i32::from(cell[1]).wrapping_mul(2048))
                .wrapping_add(i32::from(cell_offset[2]).wrapping_add(1024)),
        ];
        ScenePlacement {
            index,
            position,
            center: [
                origin[0].wrapping_add(20480),
                origin[1],
                origin[2].wrapping_add(20480),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneSlot {
    /// The stream node's slot label; this is distinct from its map index.
    pub label: i16,
    pub map_index: i16,
    pub origin: [i32; 3],
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneLocation {
    pub slot: usize,
    pub map_index: i16,
    pub grid_index: [i8; 2],
    pub cell: [i8; 2],
    pub cell_offset: [i16; 3],
    pub reference_origin: [i32; 3],
    pub relative_position: [i32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRectangle {
    pub x: i16,
    pub z: i16,
    pub columns: i32,
    pub rows: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneQuery {
    pub slot: usize,
    pub cells: CellRectangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneGridError {
    InvalidDimensions,
    MissingSlot(i16),
    InvalidSlot,
}

impl fmt::Display for SceneGridError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid scene slot graph: {self:?}")
    }
}
impl std::error::Error for SceneGridError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSlots {
    pub grid: SceneGrid,
    pub slots: [SceneSlot; 7],
}

impl SceneSlots {
    /// Supplied graphs retain the original seven slot labels. Original allocation/I/O is separate.
    pub fn validate(&self) -> Result<(), SceneGridError> {
        if self.grid.columns <= 0 || self.grid.rows <= 0 {
            return Err(SceneGridError::InvalidDimensions);
        }
        for label in 0..7 {
            if self.slots.iter().filter(|slot| slot.label == label).count() != 1 {
                return Err(SceneGridError::MissingSlot(label));
            }
        }
        Ok(())
    }

    fn find_slot(&self, label: i16) -> Result<usize, SceneGridError> {
        self.slots
            .iter()
            .position(|slot| slot.label == label)
            .ok_or(SceneGridError::MissingSlot(label))
    }

    /// Original owner selection does not test stream readiness; floor traversal does.
    pub fn locate(&self, position: [i32; 3]) -> Result<Option<SceneLocation>, SceneGridError> {
        self.validate()?;
        for label in 0..7 {
            let index = self.find_slot(label)?;
            let slot = self.slots[index];
            if position[0] < slot.origin[0]
                || position[0] >= slot.origin[0].wrapping_add(40960)
                || position[2] < slot.origin[2]
                || position[2] >= slot.origin[2].wrapping_add(40960)
            {
                continue;
            }
            if self.grid.mode != 0 {
                let upper = -i32::from(label) * 2048;
                if position[1] > upper || position[1] <= upper - 2048 {
                    continue;
                }
            }
            let map_index = i32::from(slot.map_index);
            let columns = i32::from(self.grid.columns);
            let cell = [
                (position[0].wrapping_sub(slot.origin[0]) / 2048) as i8,
                (position[2].wrapping_sub(slot.origin[2]) / 2048) as i8,
            ];
            let reference_origin = [
                slot.origin[0].wrapping_add(20480),
                slot.origin[1],
                slot.origin[2].wrapping_add(20480),
            ];
            return Ok(Some(SceneLocation {
                slot: index,
                map_index: slot.map_index,
                grid_index: [(map_index % columns) as i8, (map_index / columns) as i8],
                cell,
                cell_offset: [
                    position[0]
                        .wrapping_sub(1024)
                        .wrapping_sub(slot.origin[0])
                        .wrapping_sub(i32::from(cell[0]) * 2048) as i16,
                    position[1] as i16,
                    position[2]
                        .wrapping_sub(1024)
                        .wrapping_sub(slot.origin[2])
                        .wrapping_sub(i32::from(cell[1]) * 2048) as i16,
                ],
                reference_origin,
                relative_position: [
                    position[0].wrapping_sub(reference_origin[0]),
                    position[1],
                    position[2].wrapping_sub(reference_origin[2]),
                ],
            }));
        }
        Ok(None)
    }

    pub fn queries(
        &self,
        location: SceneLocation,
        width: i32,
    ) -> Result<Vec<SceneQuery>, SceneGridError> {
        self.validate()?;
        let slot = *self
            .slots
            .get(location.slot)
            .ok_or(SceneGridError::InvalidSlot)?;
        let width = if width & 1 == 0 {
            width.wrapping_add(1)
        } else {
            width
        };
        let [x, z] = location.cell.map(i32::from);
        if width == 1 {
            let cells = CellRectangle {
                x: x as i16,
                z: z as i16,
                columns: 1,
                rows: 1,
            };
            let mut queries = vec![SceneQuery {
                slot: location.slot,
                cells,
            }];
            if self.grid.mode == 1 {
                if slot.label + 1 < self.grid.rows {
                    queries.push(SceneQuery {
                        slot: self.find_slot(slot.label + 1)?,
                        cells,
                    });
                }
                if slot.label > 0 {
                    queries.push(SceneQuery {
                        slot: self.find_slot(slot.label - 1)?,
                        cells,
                    });
                }
            }
            return Ok(queries);
        }
        let mut columns = width;
        let mut rows = width;
        let start_x = if x == 0 {
            columns = columns.wrapping_sub(1);
            x
        } else {
            x - 1
        };
        if x == 19 {
            columns = columns.wrapping_sub(1);
        }
        let start_z = if z == 19 {
            rows = rows.wrapping_sub(1);
            z
        } else {
            z + 1
        };
        if z == 0 {
            rows = rows.wrapping_sub(1);
        }
        Ok(vec![SceneQuery {
            slot: location.slot,
            cells: CellRectangle {
                x: start_x as i16,
                z: start_z as i16,
                columns,
                rows,
            },
        }])
    }
}
