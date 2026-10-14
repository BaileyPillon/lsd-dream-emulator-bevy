use crate::{DreamLinks, LinkError, PlayerControls};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkPalette {
    pub dream: i32,
    pub flashback: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapLinkCommand {
    Fade {
        color: i32,
        target: i32,
        ticks: i32,
        attach: bool,
    },
    Notify(i32),
    MoveCallback(i32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapLinkPolicy {
    pub state: i32,
    pub tick: i32,
    pub location: i32,
    pub move_callback: i32,
    pub reset_view_pending: bool,
    pub view_active: bool,
}
impl MapLinkPolicy {
    /// Port of the map-scene link notification and its original start handlers.
    pub fn notify(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        event: i32,
        palette: LinkPalette,
    ) -> Result<Vec<MapLinkCommand>, LinkError> {
        let mut commands = Vec::new();
        if self.state != 0 {
            if event >= 9 {
                links.execution.state = 0;
            }
            return Ok(commands);
        }
        let mut fade = |color, ticks| {
            commands.push(MapLinkCommand::Fade {
                color,
                target: 0,
                ticks,
                attach: true,
            })
        };
        match event {
            10 => {
                self.state = 4;
                if links.execution.flashback {
                    fade(0, 5);
                } else {
                    match self.tick.wrapping_add(self.location) & 3 {
                        0 => commands.push(MapLinkCommand::Notify(4)),
                        1 => fade(0, 10),
                        2 => fade(4, 10),
                        3 => fade(7, 5),
                        _ => unreachable!(),
                    }
                }
            }
            12 if links.execution.stage >= 0 => {
                self.state = 5;
                fade(palette.dream, 10);
                controls.input_locked = true;
            }
            12 | 13 => {
                self.state = 6;
                fade(palette.dream, 30);
                controls.input_locked = true;
            }
            14 => {
                self.state = 7;
                fade(palette.flashback, 5);
                controls.input_locked = true;
            }
            15 | 16 => {
                self.state = if event == 15 { 8 } else { 10 };
                fade(0, 6);
                if event == 16 {
                    self.move_callback = 2;
                    self.reset_view_pending = true;
                    self.view_active = true;
                    commands.push(MapLinkCommand::MoveCallback(2));
                }
                controls.paused = if event == 15 { 1 } else { 2 };
                controls.move_direction = 1;
                controls.previous_move_direction = 1;
                if let Some(packet) = links.source_transform {
                    controls.apply_rotation_packet(true, packet)?;
                }
            }
            17 => commands.push(MapLinkCommand::Notify(11)),
            _ => {}
        }
        Ok(commands)
    }
}
