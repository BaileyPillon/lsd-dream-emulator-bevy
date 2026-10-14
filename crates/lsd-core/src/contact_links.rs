use crate::{
    DreamLinkEffects, DreamLinks, LinkCatalog, LinkContext, LinkError, LinkKind, PlayerControls,
    PsxRng, SpawnLocation,
};

/// The original saved transform contains an 80-byte coordinate block and a 40-byte tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkTransformState {
    pub head: [u32; 20],
    pub tail: [u32; 10],
}
impl LinkTransformState {
    pub fn restore_from(&mut self, saved: &Self) {
        *self = saved.clone();
        self.head[0] = 0;
    }
}

pub trait DreamContactEffects: DreamLinkEffects {
    fn base_floor(&mut self, state: &mut DreamLinks, controls: &mut PlayerControls, code: i32);
    fn base_wall(&mut self, state: &mut DreamLinks, controls: &mut PlayerControls, code: i32);
    fn restore_link_pose(&mut self, state: &mut DreamLinks, controls: &mut PlayerControls);
}

impl DreamLinks {
    pub fn wall_at(
        &mut self,
        controls: &mut PlayerControls,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        request: WallLinkRequest,
        rng: &mut PsxRng,
        effects: &mut impl DreamContactEffects,
    ) -> Result<(), LinkError> {
        effects.base_wall(self, controls, request.code);
        if request.code != 4 || self.execution.state != 0 {
            return Ok(());
        }
        self.spawn = request.contact;
        let key = self.spawn;
        let static_handled = if let Some(target) = catalog.static_spawn(
            LinkKind::Wall,
            key,
            self.execution.stage,
            &mut self.spawn,
            context,
        )? {
            self.execute(controls, target, 13, true, effects);
            true
        } else {
            false
        };
        if !static_handled && request.can_dynamic && self.execution.state == 0 {
            let target =
                catalog.random_spawn(self.execution.stage, &mut self.spawn, context, rng)?;
            self.execute(controls, target, 12, true, effects);
        }
        effects.restore_link_pose(self, controls);
        Ok(())
    }

    pub fn floor_event(
        &mut self,
        controls: &mut PlayerControls,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        request: FloorLinkRequest,
        rng: &mut PsxRng,
        effects: &mut impl DreamContactEffects,
    ) -> Result<FloorLinkResponse, LinkError> {
        effects.base_floor(self, controls, request.code);
        let mut surface = request.previous_surface;
        let mut dynamic = false;
        match request.code {
            -1 => {
                surface = i32::from(request.tile_flags & 0x7f);
                if surface >= 24 {
                    surface = 0;
                }
                if self.execution.state == 15 && surface == 0 {
                    surface = 2;
                }
                dynamic = self.execution.stage == 9;
            }
            -2 => {
                if request.owner_ready == Some(2) {
                    dynamic = true;
                } else {
                    effects.restore_link_pose(self, controls);
                }
            }
            _ => {}
        }
        if dynamic {
            self.dynamic_at(
                controls,
                catalog,
                context,
                request.current_key,
                rng,
                effects,
            )?;
        }
        Ok(FloorLinkResponse { surface })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallLinkRequest {
    pub code: i32,
    pub contact: SpawnLocation,
    pub can_dynamic: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorLinkRequest {
    pub code: i32,
    pub tile_flags: u8,
    pub owner_ready: Option<i16>,
    pub current_key: SpawnLocation,
    pub previous_surface: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorLinkResponse {
    pub surface: i32,
}
