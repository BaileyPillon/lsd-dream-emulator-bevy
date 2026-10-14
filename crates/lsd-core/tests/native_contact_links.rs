#[allow(dead_code)]
#[path = "support/link_state.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "support/links.rs"]
mod support;
use lsd_core::{
    DreamContactEffects, DreamLinkEffects, DreamLinks, FloorLinkRequest, LinkTransformState,
    PlayerControls, SpawnLocation, WallLinkRequest,
};
use runtime::{Effects, array, check, initial};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/contact-links.json")).unwrap()
}
struct ContactEffects {
    base: Effects,
    saved: LinkTransformState,
    live: LinkTransformState,
    contact: u32,
}
impl DreamLinkEffects for ContactEffects {
    fn notify(&mut self, state: &mut DreamLinks, controls: &mut PlayerControls, event: i32) {
        self.base.notify(state, controls, event);
    }
    fn sound(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        parameters: [i32; 3],
    ) {
        self.base.sound(state, controls, parameters);
    }
    fn rotate(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        absolute: bool,
        packet: [u32; 3],
    ) {
        self.base.rotate(state, controls, absolute, packet);
    }
    fn translate(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        delta: [i32; 3],
    ) {
        self.base.translate(state, controls, delta);
    }
    fn relocate(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        spawn: SpawnLocation,
    ) {
        self.base.relocate(state, controls, spawn);
    }
}
impl DreamContactEffects for ContactEffects {
    fn base_floor(&mut self, _state: &mut DreamLinks, _controls: &mut PlayerControls, code: i32) {
        self.base.trace.push(json!(["base_floor", code]));
    }
    fn base_wall(&mut self, _state: &mut DreamLinks, _controls: &mut PlayerControls, code: i32) {
        self.base.trace.push(json!(["base_wall", code]));
        if code == 4 {
            self.contact = 0x10c000;
        }
    }
    fn restore_link_pose(&mut self, _state: &mut DreamLinks, controls: &mut PlayerControls) {
        self.base.trace.push(json!(["restore"]));
        self.live.restore_from(&self.saved);
        self.base.position = std::array::from_fn(|i| self.live.head[6 + i] as i32);
        controls.angles = [
            self.live.tail[4] as i16,
            (self.live.tail[4] >> 16) as i16,
            self.live.tail[5] as i16,
        ];
    }
}
fn effects(case: &Value) -> ContactEffects {
    let mut saved = LinkTransformState {
        head: [0; 20],
        tail: [0; 10],
    };
    saved.head[17] = 0x102000;
    for i in 0..3 {
        saved.head[6 + i] = case["initial"]["position"][i].as_i64().unwrap() as u32;
    }
    let angles: [i16; 3] = array(&case["initial"]["angles"], |v| v.as_i64().unwrap() as i16);
    saved.tail[4] = u32::from(angles[0] as u16) | (u32::from(angles[1] as u16) << 16);
    saved.tail[5] = u32::from(angles[2] as u16);
    let mut live = saved.clone();
    for (i, value) in [9000i32, -8000, 7000].into_iter().enumerate() {
        live.head[6 + i] = value as u32;
    }
    live.tail[4] = u32::from((-123i16) as u16) | (234u32 << 16);
    live.tail[5] = u32::from((-345i16) as u16);
    ContactEffects {
        base: Effects {
            cancel: case["cancel"].as_bool().unwrap_or(false),
            position: [9000, -8000, 7000],
            trace: Vec::new(),
        },
        saved,
        live,
        contact: 0x10b000,
    }
}
#[test]
fn wall_links_preserve_native_contact_notification_rng_sound_and_restoration_order() {
    let fixture = fixture();
    let catalog = support::catalog(&fixture["catalog"]);
    for (index, case) in fixture["wall"].as_array().unwrap().iter().enumerate() {
        let (mut state, mut controls, mut context, mut rng) = initial(&case["initial"]);
        controls.angles = [-123, 234, -345];
        let mut effects = effects(case);
        state
            .wall_at(
                &mut controls,
                &catalog,
                &mut context,
                WallLinkRequest {
                    code: case["code"].as_i64().unwrap() as i32,
                    contact: support::location(&case["contact_key"]),
                    can_dynamic: case["can_dynamic"].as_bool().unwrap(),
                },
                &mut rng,
                &mut effects,
            )
            .unwrap();
        check(&state, &controls, &context, &rng, &effects.base, case);
        assert_eq!(
            effects.contact,
            case["contact"].as_u64().unwrap() as u32,
            "contact {index}"
        );
        assert_eq!(
            effects.live.head,
            array(&case["head"], |v| v.as_u64().unwrap() as u32)
        );
        assert_eq!(
            effects.live.tail,
            array(&case["tail"], |v| v.as_u64().unwrap() as u32)
        );
    }
    assert_eq!(fixture["wall"].as_array().unwrap().len(), 240);
}
#[test]
fn floor_surface_fall_restore_and_dynamic_link_rules_match_native_cpu() {
    let fixture = fixture();
    let catalog = support::catalog(&fixture["catalog"]);
    for case in fixture["floor"].as_array().unwrap() {
        let (mut state, mut controls, mut context, mut rng) = initial(&case["initial"]);
        controls.angles = [-123, 234, -345];
        let mut effects = effects(case);
        let response = state
            .floor_event(
                &mut controls,
                &catalog,
                &mut context,
                FloorLinkRequest {
                    code: case["code"].as_i64().unwrap() as i32,
                    tile_flags: case["flags"].as_u64().unwrap() as u8,
                    owner_ready: Some(case["owner_ready"].as_i64().unwrap() as i16),
                    current_key: support::location(&case["initial"]["key"]),
                    previous_surface: 5,
                },
                &mut rng,
                &mut effects,
            )
            .unwrap();
        assert_eq!(response.surface, case["surface"].as_i64().unwrap() as i32);
        check(&state, &controls, &context, &rng, &effects.base, case);
        assert_eq!(
            effects.live.head,
            array(&case["head"], |v| v.as_u64().unwrap() as u32)
        );
        assert_eq!(
            effects.live.tail,
            array(&case["tail"], |v| v.as_u64().unwrap() as u32)
        );
    }
    assert_eq!(fixture["floor"].as_array().unwrap().len(), 360);
}
#[test]
fn complete_coordinate_and_tail_snapshots_match_original_save_restore() {
    let fixture = fixture();
    for case in fixture["snapshots"].as_array().unwrap() {
        let saved = LinkTransformState {
            head: array(&case["initial_head"], |v| v.as_u64().unwrap() as u32),
            tail: array(&case["initial_tail"], |v| v.as_u64().unwrap() as u32),
        };
        assert_eq!(
            saved.head,
            array(&case["saved_head"], |v| v.as_u64().unwrap() as u32)
        );
        assert_eq!(
            saved.tail,
            array(&case["saved_tail"], |v| v.as_u64().unwrap() as u32)
        );
        let mut live = LinkTransformState {
            head: [0x55555555; 20],
            tail: [0x66666666; 10],
        };
        live.restore_from(&saved);
        assert_eq!(
            live.head,
            array(&case["restored_head"], |v| v.as_u64().unwrap() as u32)
        );
        assert_eq!(
            live.tail,
            array(&case["restored_tail"], |v| v.as_u64().unwrap() as u32)
        );
    }
}
