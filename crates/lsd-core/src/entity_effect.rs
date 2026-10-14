use crate::{
    EffectEvent, EffectFault, EffectPalette, EffectState, FrameSignal, GenerationLayer,
    GpuBoxPacket,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEffectDefinition {
    pub palette: EffectPalette,
    pub size: [u16; 2],
    pub offset: [i32; 2],
}
/// The actor transform binding survives color-cycle completion; the clock
/// subscription ends independently. The owning entity retains this allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEffect {
    pub definition: EntityEffectDefinition,
    pub state: EffectState,
    pub owner_bound: bool,
    pub clock_attached: bool,
}
impl EntityEffect {
    pub fn new(definition: EntityEffectDefinition) -> Self {
        let state = EffectState::new(definition.palette.attached[0], 8191, 0);
        Self {
            definition,
            state,
            owner_bound: false,
            clock_attached: false,
        }
    }
    pub fn bind_and_start(
        &mut self,
        color_step: i32,
        variant: i32,
        mode: i32,
    ) -> Result<bool, EffectFault> {
        self.owner_bound = true;
        // entity_create_effect resets the step even when the retained cycle is active.
        self.state.color_step = color_step;
        let events = self
            .state
            .start(&self.definition.palette, false, variant, mode)?;
        let attach = events.contains(&EffectEvent::Attach);
        self.clock_attached |= attach;
        Ok(attach)
    }
    pub fn tick(&mut self, signal: FrameSignal) -> Vec<EffectEvent> {
        if !self.clock_attached {
            return Vec::new();
        }
        let events = self.state.tick(&self.definition.palette, signal.code());
        if events.contains(&EffectEvent::Detach) {
            self.clock_attached = false;
        }
        events
    }
    pub fn packet(&self, screen: [i32; 2]) -> Option<GpuBoxPacket> {
        if !self.owner_bound {
            return None;
        }
        GenerationLayer {
            offset: self.definition.offset,
            size: self.definition.size,
            color: self.state.color,
            flags: self.state.render_flags,
            depth: 0,
            parent: None,
            relative: true,
        }
        .gpu_box_packet(screen)
    }
}
