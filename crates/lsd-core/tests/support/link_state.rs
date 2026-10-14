use crate::support;
use lsd_core::{
    AutoMoveTables, DreamLinkEffects, DreamLinks, LinkContext, LinkExecution, PlayerControls,
    PsxRng, SceneGrid, SpawnLocation,
};
use serde_json::{Value, json};
pub fn integer(value: &Value, name: &str) -> i32 {
    value[name].as_i64().unwrap() as i32
}
pub fn array<T: Copy, const N: usize>(value: &Value, read: impl Fn(&Value) -> T) -> [T; N] {
    std::array::from_fn(|i| read(&value[i]))
}
pub fn initial(value: &Value) -> (DreamLinks, PlayerControls, LinkContext, PsxRng) {
    let state = DreamLinks {
        execution: LinkExecution {
            state: integer(value, "state"),
            stage: integer(value, "stage"),
            dream_timer: value["timer"].as_u64().unwrap() as u32,
            flashback: integer(value, "flashback") != 0,
        },
        spawn: support::location(&value["spawn"]),
        source_transform: None,
        target_transform: None,
        auto_action: value["auto_action"].as_u64().map(|v| v as u8),
        auto_step: integer(value, "auto_step"),
        auto_target: support::location(&value["auto_target"]),
        time_limit: integer(value, "limit"),
        default_teleport_transform: false,
    };
    let controls = PlayerControls {
        angles: array(&value["angles"], |v| v.as_i64().unwrap() as i16),
        move_state: integer(value, "move_state"),
        move_direction: integer(value, "move_direction"),
        previous_move_direction: integer(value, "previous_direction"),
        left_right: integer(value, "left_right"),
        look_state: integer(value, "look_state"),
        is_forward: integer(value, "is_forward") != 0,
        auto_move_locked: integer(value, "auto_locked") != 0,
        auto_move_active: integer(value, "auto_active") != 0,
        ..PlayerControls::default()
    };
    let context = LinkContext {
        source_stage: 13,
        source_index: 7,
        target_stage: 11,
        target_index: 9,
        teleports_enabled: integer(value, "enabled"),
        dynamic_penalty: 17,
        ..LinkContext::default()
    };
    (
        state,
        controls,
        context,
        PsxRng::new(value["seed"].as_u64().unwrap() as u32),
    )
}
pub struct Effects {
    pub cancel: bool,
    pub position: [i32; 3],
    pub trace: Vec<Value>,
}
impl DreamLinkEffects for Effects {
    fn notify(&mut self, state: &mut DreamLinks, _controls: &mut PlayerControls, event: i32) {
        self.trace.push(json!(["notify", event]));
        if self.cancel {
            state.execution.state = 0;
        }
    }
    fn sound(
        &mut self,
        _state: &mut DreamLinks,
        _controls: &mut PlayerControls,
        parameters: [i32; 3],
    ) {
        self.trace.push(json!([
            "sound",
            parameters[0],
            parameters[1],
            parameters[2]
        ]));
    }
    fn rotate(
        &mut self,
        _state: &mut DreamLinks,
        controls: &mut PlayerControls,
        absolute: bool,
        packet: [u32; 3],
    ) {
        self.trace.push(json!(["rotate", absolute, packet]));
        controls.apply_rotation_packet(absolute, packet).unwrap();
    }
    fn translate(
        &mut self,
        _state: &mut DreamLinks,
        _controls: &mut PlayerControls,
        delta: [i32; 3],
    ) {
        self.trace.push(json!(["translate", delta]));
        self.position = std::array::from_fn(|i| self.position[i].wrapping_add(delta[i]));
    }
    fn relocate(
        &mut self,
        _state: &mut DreamLinks,
        _controls: &mut PlayerControls,
        spawn: SpawnLocation,
    ) {
        self.position = SceneGrid {
            columns: 3,
            rows: 3,
            mode: 0,
        }
        .place_spawn([1000, -3000, 2000], spawn)
        .position;
        self.trace.push(json!(["relocate", self.position]));
    }
}
pub fn tables(fixture: &Value) -> AutoMoveTables {
    let value = &fixture["auto_tables"];
    AutoMoveTables {
        targets: array(&value["targets"], |row| {
            array(row, |v| v.as_i64().unwrap() as i16)
        }),
        clockwise: array(&value["clockwise"], |v| v.as_u64().unwrap() as u32),
        counterclockwise: array(&value["counterclockwise"], |v| v.as_u64().unwrap() as u32),
    }
}
pub fn check(
    state: &DreamLinks,
    controls: &PlayerControls,
    context: &LinkContext,
    rng: &PsxRng,
    effects: &Effects,
    value: &Value,
) {
    assert_eq!(state.execution.state, integer(value, "state"));
    assert_eq!(state.execution.stage, integer(value, "stage"));
    assert_eq!(
        state.execution.dream_timer,
        value["timer"].as_u64().unwrap() as u32
    );
    assert_eq!(state.execution.flashback, integer(value, "flashback") != 0);
    assert_eq!(state.time_limit, integer(value, "limit"));
    assert_eq!(
        state.auto_action,
        value["auto_action"].as_u64().map(|v| v as u8)
    );
    assert_eq!(state.auto_step, integer(value, "auto_step"));
    assert_eq!(state.spawn, support::location(&value["spawn"]));
    assert_eq!(state.auto_target, support::location(&value["auto_target"]));
    for (name, actual) in [
        ("source", state.source_transform),
        ("target", state.target_transform),
    ] {
        assert_eq!(
            actual,
            value[name]
                .as_array()
                .map(|row| std::array::from_fn(|i| row[i].as_u64().unwrap() as u32))
        );
    }
    assert_eq!(
        state.default_teleport_transform,
        value["default_teleport_transform"].as_bool().unwrap()
    );
    for (name, actual) in [
        ("move_state", controls.move_state),
        ("move_direction", controls.move_direction),
        ("previous_direction", controls.previous_move_direction),
        ("left_right", controls.left_right),
        ("look_state", controls.look_state),
        ("is_forward", i32::from(controls.is_forward)),
        ("auto_locked", i32::from(controls.auto_move_locked)),
        ("auto_active", i32::from(controls.auto_move_active)),
    ] {
        assert_eq!(actual, integer(value, name), "{name}");
    }
    assert_eq!(
        controls.angles,
        array(&value["angles"], |v| v.as_i64().unwrap() as i16)
    );
    assert_eq!(
        effects.position,
        array(&value["position"], |v| v.as_i64().unwrap() as i32)
    );
    assert_eq!(effects.trace, *value["trace"].as_array().unwrap());
    assert_eq!(
        [
            context.source_stage,
            context.source_index,
            context.target_stage,
            context.target_index
        ],
        array(&value["globals"], |v| v.as_i64().unwrap() as i32)
    );
    assert_eq!(
        context.navigation,
        array(&value["navigation"], |v| v.as_u64().unwrap() as u8)
    );
    assert_eq!(context.dynamic_penalty, integer(value, "penalty"));
    assert_eq!(rng.state(), value["rng_state"].as_u64().unwrap() as u32);
}
