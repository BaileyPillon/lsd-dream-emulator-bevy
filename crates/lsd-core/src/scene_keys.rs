use crate::{SceneGrid, SceneGridError, SceneLocation, ScenePlacement, SceneSlots, SpawnLocation};

impl SceneLocation {
    pub fn spawn_key(self) -> SpawnLocation {
        SpawnLocation {
            chunk: i16::from_le_bytes(self.grid_index.map(|value| value as u8)),
            tile: i16::from_le_bytes(self.cell.map(|value| value as u8)),
            position: self.cell_offset,
        }
    }
}

impl SceneGrid {
    pub fn place_spawn(self, base: [i32; 3], spawn: SpawnLocation) -> ScenePlacement {
        self.place(
            base,
            spawn.chunk.to_le_bytes().map(|value| value as i8),
            spawn.tile.to_le_bytes().map(|value| value as i8),
            spawn.position,
        )
    }
}

/// Native current/contact keys are separate ten-byte caches, not live position aliases.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneActorKeys {
    pub current: SpawnLocation,
    pub contact: SpawnLocation,
}

impl SceneActorKeys {
    pub fn current_spawn(
        &mut self,
        refresh: bool,
        slots: &SceneSlots,
        position: [i32; 3],
    ) -> Result<Option<SpawnLocation>, SceneGridError> {
        if refresh {
            let Some(location) = slots.locate(position)? else {
                return Ok(None);
            };
            self.current = location.spawn_key();
        }
        Ok(Some(self.current))
    }

    /// Dispatch retains the current chunk even when a candidate belongs to a neighboring slot.
    pub fn stamp_body_candidate(&mut self, cell: [i8; 2]) {
        self.contact.chunk = self.current.chunk;
        self.contact.tile = i16::from_le_bytes(cell.map(|value| value as u8));
    }
}
