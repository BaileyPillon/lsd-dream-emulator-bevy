use crate::{formats::Lbd, load_scene_grid, load_stream_grid_tables, stage_map_path};
use lsd_core::{
    CollisionPolicy, SceneCollision, SceneSlot, SceneSlots, StreamEvent, StreamGridTables,
    StreamProtocol, StreamSlotState,
};
use std::path::{Path, PathBuf};

/// Host file completion feeds the recovered protocol; original CD timing and actor callbacks are separate.
pub struct StreamedStage {
    pub collision: SceneCollision,
    pub maps: [Option<Lbd>; 7],
    pub protocol: StreamProtocol,
    pub revision: u64,
    pub events: Vec<StreamEvent>,
    stage: u8,
    files: PathBuf,
    executable: Vec<u8>,
    tables: StreamGridTables,
    pending_data: [Option<Lbd>; 7],
}

impl StreamedStage {
    pub fn stage_id(&self) -> u8 {
        self.stage
    }

    pub fn scene_scale_tables(&self) -> Result<lsd_core::SceneScaleTables, &'static str> {
        crate::load_scene_scale_tables(&self.executable)
    }

    pub fn load_destination(
        &self,
        stage: u8,
        spawn: lsd_core::SpawnLocation,
    ) -> Result<(Self, [i32; 3]), Box<dyn std::error::Error>> {
        let grid = load_scene_grid(&self.executable, stage)?;
        let placement = grid.place_spawn([0; 3], spawn);
        let map =
            u16::try_from(placement.index).map_err(|_| "Destination map index is negative")?;
        let loaded = Self::load(&self.files, &self.executable, stage, map, [0; 3])?;
        Ok((loaded, placement.position))
    }

    pub fn load_texture_variant(
        &self,
        variant: char,
    ) -> Result<crate::formats::Vram, Box<dyn std::error::Error>> {
        if !matches!(variant, 'A' | 'B' | 'C' | 'D') {
            return Err("Texture variant must be A-D".into());
        }
        let bytes = std::fs::read(
            self.files
                .join(format!("CDI/STG{:02}/TEX{variant}.TIX", self.stage)),
        )?;
        let tix = crate::formats::Tix::parse(&bytes)?;
        let mut vram = crate::formats::Vram::default();
        for bank in &tix.banks {
            for image in &bank.images {
                vram.upload(image)?;
            }
        }
        Ok(vram)
    }

    pub fn load(
        files: &Path,
        executable: &[u8],
        stage: u8,
        map: u16,
        base: [i32; 3],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let grid = load_scene_grid(executable, stage)?;
        stage_map_path(executable, stage, map)?;
        let tables = load_stream_grid_tables(executable)?;
        let initial_index = if grid.mode == 0 { i32::from(map) } else { 0 };
        let placement = grid.place(
            base,
            [
                (initial_index % i32::from(grid.columns)) as i8,
                (initial_index / i32::from(grid.columns)) as i8,
            ],
            [0; 2],
            [0; 3],
        );
        let slots = SceneSlots {
            grid,
            slots: std::array::from_fn(|index| SceneSlot {
                label: index as i16,
                map_index: -1,
                origin: [0; 3],
                ready: false,
            }),
        };
        let protocol = StreamProtocol {
            activity: 0,
            pending: 0,
            finalized: 0,
            slots: std::array::from_fn(|index| StreamSlotState {
                label: index as u16,
                map_index: u16::MAX,
                ..Default::default()
            }),
        };
        let mut result = Self {
            collision: SceneCollision {
                slots,
                chunks: std::array::from_fn(|_| None),
            },
            maps: std::array::from_fn(|_| None),
            protocol,
            revision: 0,
            events: Vec::new(),
            stage,
            files: files.to_owned(),
            executable: executable.to_vec(),
            tables,
            pending_data: std::array::from_fn(|_| None),
        };
        result.rebind(initial_index, placement.center, Some(result.tables.initial))?;
        result.complete_frame()?;
        Ok(result)
    }

    fn rebind(
        &mut self,
        index: i32,
        center: [i32; 3],
        selection: Option<lsd_core::SlotSelection>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(plan) = self
            .tables
            .rebind(&self.collision.slots, index, center, selection)?
        else {
            return Ok(false);
        };
        self.collision.slots = plan.slots;
        for (state, slot) in self
            .protocol
            .slots
            .iter_mut()
            .zip(&self.collision.slots.slots)
        {
            state.label = slot.label as u16;
        }
        let events = self.protocol.request(&plan.requests)?;
        self.handle(events)?;
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }

    fn handle(&mut self, events: Vec<StreamEvent>) -> Result<(), Box<dyn std::error::Error>> {
        for event in events {
            self.events.push(event);
            match event {
                StreamEvent::Release { slot } => {
                    self.maps[slot] = None;
                    self.collision.chunks[slot] = None;
                    self.protocol.slots[slot].ready = 0;
                    self.collision.slots.slots[slot].ready = false;
                }
                StreamEvent::Load { slot, map } => {
                    self.protocol.slots[slot].busy = 1;
                    self.protocol.slots[slot].ready = 0;
                    self.collision.slots.slots[slot].ready = false;
                    let result = (|| {
                        let path = stage_map_path(&self.executable, self.stage, map as u16)?;
                        Ok::<_, Box<dyn std::error::Error>>(Lbd::parse(&std::fs::read(
                            self.files.join(path),
                        )?)?)
                    })();
                    match result {
                        Ok(map) => self.pending_data[slot] = Some(map),
                        Err(error) => {
                            self.protocol.slots[slot].busy = 0;
                            return Err(error);
                        }
                    }
                }
                StreamEvent::Cancel { slot } => {
                    self.pending_data[slot] = None;
                    self.protocol.slots[slot].busy = 0;
                }
                StreamEvent::Bind { slot } => {
                    let map = self.pending_data[slot]
                        .take()
                        .ok_or("Ready stream has no decoded map")?;
                    self.collision.chunks[slot] =
                        Some(map.collision_chunk(CollisionPolicy::for_stage(self.stage)));
                    self.maps[slot] = Some(map);
                    self.collision.slots.slots[slot].ready = true;
                    self.collision.slots.slots[slot].map_index =
                        self.protocol.slots[slot].map_index as i16;
                    self.revision = self.revision.wrapping_add(1);
                }
                StreamEvent::Notify { .. } => {} // Retained events await original scene/actor callback dispatch.
            }
        }
        Ok(())
    }

    pub fn complete_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (state, data) in self.protocol.slots.iter_mut().zip(&self.pending_data) {
            if state.busy != 0 && data.is_some() {
                state.busy = 0;
                state.ready = 1;
                state.completed = 1;
            }
        }
        let events = self.protocol.tick(2);
        self.handle(events)
    }

    pub fn update_for_position(
        &mut self,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.collision.slots.grid.mode != 0 || self.protocol.activity != 0 {
            return Ok(false);
        }
        let Some(location) = self.collision.slots.locate(position)? else {
            return Ok(false);
        };
        let slot = self.collision.slots.slots[location.slot];
        if !slot.ready {
            return Ok(false);
        }
        let kind = usize::from(self.tables.label_kinds[slot.label as usize]);
        self.rebind(
            i32::from(location.map_index),
            location.reference_origin,
            self.tables.reuse[kind],
        )
    }
}
