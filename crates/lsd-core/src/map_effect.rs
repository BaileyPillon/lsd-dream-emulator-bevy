use crate::{
    EffectEvent, EffectFault, EffectPalette, EffectState, MapCompletionCommand, MapLinkCommand,
    MapLinkPolicy, PlayerControls,
};

/// Supplies the two effect-parent relationships for the development scene.
/// Natural actor lists, renderer composition and session reconstruction are separate consumers.
#[derive(Debug, Clone)]
pub struct MapEffect {
    pub effect: EffectState,
    pub palette: EffectPalette,
    pub render_attached: bool,
    pub map_attached: bool,
    pub stage_color: [u8; 3],
}
impl MapEffect {
    /// Original map-start uses the decreasing cycle with variant reuse outside flashbacks.
    pub fn start_scene(&mut self, flashback: bool, variant: i32) -> Result<(), EffectFault> {
        self.map_attached = true;
        self.effect.linked = i32::from(flashback);
        self.effect.linked_divisor = if flashback { 3 } else { 0 };
        let events = self.effect.start(
            &self.palette,
            false,
            if flashback { variant } else { -1 },
            0,
        )?;
        self.render_attached |= events.contains(&EffectEvent::Attach);
        Ok(())
    }

    pub fn new(palette: EffectPalette) -> Self {
        Self {
            effect: EffectState::new(palette.attached[0], 8191, 0),
            stage_color: palette.attached[0],
            palette,
            render_attached: false,
            map_attached: false,
        }
    }

    pub fn consume(
        &mut self,
        command: MapLinkCommand,
    ) -> Result<Vec<MapCompletionCommand>, EffectFault> {
        match command {
            MapLinkCommand::Fade {
                variant,
                mode,
                color_step,
                attach,
            } => {
                if color_step != 0 {
                    self.effect.color_step = color_step;
                }
                if attach {
                    self.map_attached = true;
                }
                let events = self.effect.start(&self.palette, true, variant, mode)?;
                self.render_attached |= events.contains(&EffectEvent::Attach);
                Ok(Vec::new())
            }
            MapLinkCommand::Notify(code) => Ok(vec![MapCompletionCommand::Notify(code)]),
            MapLinkCommand::MoveCallback(_) => Ok(Vec::new()),
        }
    }

    pub fn tick(
        &mut self,
        policy: &mut MapLinkPolicy,
        controls: &mut PlayerControls,
    ) -> Result<Vec<MapCompletionCommand>, EffectFault> {
        if !self.render_attached {
            return Ok(Vec::new());
        }
        let mut commands = Vec::new();
        for event in self.effect.tick(&self.palette, 2) {
            match event {
                EffectEvent::Detach => self.render_attached = false,
                EffectEvent::Notify(code) if self.map_attached => {
                    commands.extend(policy.complete_effect(
                        controls,
                        &self.effect,
                        &self.palette,
                        code,
                    )?);
                }
                _ => {}
            }
        }
        for command in &commands {
            match command {
                MapCompletionCommand::DetachEffect => self.map_attached = false,
                MapCompletionCommand::SetStageColor(color) => self.stage_color = *color,
                _ => {}
            }
        }
        Ok(commands)
    }
}
