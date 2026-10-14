use crate::{
    DreamLinks, LinkError, MapLinkPolicy, PlayerControls, SpawnLocation, rotation_degrees,
};

/// Original reset values for services which the scene host owns separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneMotionReset {
    pub mood_key: SpawnLocation,
    pub view_callback: i32,
    pub floor_effect: i32,
    pub floor_surface: i32,
    pub interval: u32,
    pub can_dynamic: i32,
    pub link_parameters: [i32; 3],
    pub cinematic_variant: i16,
    pub reset_flag: i32,
    pub rotation_packet: [u32; 3],
}

pub fn reset_scene_motion(
    controls: &mut PlayerControls,
    links: &mut DreamLinks,
    map: &mut MapLinkPolicy,
    direction: i32,
    interval: u32,
) -> Result<SceneMotionReset, LinkError> {
    if map.move_callback == 2 {
        map.reset_view_pending = false;
        map.view_active = false;
    }
    map.move_callback = 1;
    if direction >= 0 {
        controls.move_direction = direction;
        controls.previous_move_direction = direction;
    }
    controls.move_step = 0;
    controls.move_state = 0;
    controls.left_right = 0;
    controls.look_state = 0;
    controls.turn_state = 0;
    controls.look_vertical = 0;
    controls.turn_offset = 0;
    controls.input_locked = false;
    links.execution.state = 0;
    controls.action_pressed = false;
    controls.auto_move_locked = false;
    controls.auto_move_active = false;
    links.auto_action = None;
    let degrees = rotation_degrees(controls.angles);
    let packet = [
        (1 << 16) | u32::from(degrees[0] as u16),
        (1 << 16) | u32::from(degrees[1] as u16),
        1 << 16,
    ];
    controls.apply_rotation_packet(true, packet)?;
    Ok(SceneMotionReset {
        mood_key: links.spawn,
        view_callback: 1,
        floor_effect: -1,
        floor_surface: 0,
        interval,
        can_dynamic: 0,
        link_parameters: [1; 3],
        cinematic_variant: -1,
        reset_flag: 0,
        rotation_packet: packet,
    })
}
