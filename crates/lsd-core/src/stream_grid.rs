use crate::{SceneGrid, SceneGridError, SceneSlots};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotChoice {
    pub direction: u8,
    pub move_slot: bool,
}
pub type SlotSelection = [SlotChoice; 7];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamGridTables {
    pub origin_deltas: [[i32; 3]; 7],
    pub direction_masks: [u32; 7],
    pub index_steps: [[i32; 3]; 7],
    pub label_kinds: [u8; 7],
    pub initial: SlotSelection,
    pub reuse: [Option<SlotSelection>; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapRequest {
    pub label: u8,
    pub map_index: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRebind {
    pub slots: SceneSlots,
    pub coordinate_flags_cleared: [bool; 7],
    pub requests: Vec<MapRequest>,
}

impl SceneGrid {
    pub fn boundary_mask(self, index: i32, parity: i32) -> Result<u32, SceneGridError> {
        if self.columns <= 0 || self.rows <= 0 {
            return Err(SceneGridError::InvalidDimensions);
        }
        if self.mode != 0 {
            return Ok(if self.rows >= 32 {
                u32::MAX
            } else {
                (1u32 << self.rows) - 1
            });
        }
        let columns = i32::from(self.columns);
        let mut excluded = 0u32;
        if index < columns {
            excluded |= 3;
        }
        if index >= columns.wrapping_mul(i32::from(self.rows) - 1) {
            excluded |= 0x60;
        }
        if index % columns == 0 {
            excluded |= if parity != 0 { 0x25 } else { 4 };
        }
        if index.wrapping_add(1) % columns == 0 {
            excluded |= if parity != 0 { 0x10 } else { 0x52 };
        }
        Ok(!excluded)
    }
}

impl StreamGridTables {
    pub fn neighbor(
        &self,
        grid: SceneGrid,
        index: i32,
        parity: i32,
        mask: u32,
        direction: u8,
    ) -> Result<Option<i32>, SceneGridError> {
        let direction = usize::from(direction);
        if direction >= 7 {
            return Err(SceneGridError::InvalidSlot);
        }
        if mask & self.direction_masks[direction] == 0 {
            return Ok(None);
        }
        if grid.mode != 0 {
            return Ok(Some(index.wrapping_add(direction as i32)));
        }
        let step = self.index_steps[direction];
        let offset = if step[0] == 0 {
            step[1]
        } else {
            i32::from(grid.columns)
                .wrapping_mul(step[0])
                .wrapping_add(if parity != 0 { step[1] } else { step[2] })
        };
        Ok(Some(index.wrapping_add(offset)))
    }

    pub fn rebind(
        &self,
        slots: &SceneSlots,
        index: i32,
        center: [i32; 3],
        selection: Option<SlotSelection>,
    ) -> Result<Option<StreamRebind>, SceneGridError> {
        slots.validate()?;
        let Some(selection) = selection else {
            return Ok(None);
        };
        let parity = (index / i32::from(slots.grid.columns)) & 1;
        let mask = slots.grid.boundary_mask(index, parity)?;
        let mut next = slots.clone();
        let mut cleared = [false; 7];
        let mut requests = Vec::new();
        for (old_label, choice) in selection.iter().enumerate() {
            let physical = slots
                .slots
                .iter()
                .position(|slot| slot.label == old_label as i16)
                .ok_or(SceneGridError::MissingSlot(old_label as i16))?;
            if choice.direction >= 7 {
                return Err(SceneGridError::InvalidSlot);
            }
            next.slots[physical].label = i16::from(choice.direction);
            if choice.move_slot {
                let delta = self.origin_deltas[usize::from(choice.direction)];
                next.slots[physical].origin = if slots.grid.mode == 0 {
                    [
                        center[0].wrapping_add(delta[0]),
                        center[1],
                        center[2].wrapping_add(delta[2]),
                    ]
                } else {
                    [
                        center[0].wrapping_sub(20480),
                        center[1].wrapping_add(delta[1]),
                        center[2].wrapping_sub(20480),
                    ]
                };
                cleared[physical] = true;
                requests.push(MapRequest {
                    label: choice.direction,
                    map_index: self.neighbor(slots.grid, index, parity, mask, choice.direction)?,
                });
            }
        }
        next.validate()?;
        Ok(Some(StreamRebind {
            slots: next,
            coordinate_flags_cleared: cleared,
            requests,
        }))
    }
}
