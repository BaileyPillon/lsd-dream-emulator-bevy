use lsd_assets::formats::Mom;
use lsd_core::{
    BehaviorFault, BodyPeer, EntityBehavior, EntityBodyEvent, EntityBodyHost, EntityScriptHost,
    FixedMatrix, GenerationSound, ModelActor, NpcScript, PsxRng,
};
use serde_json::{Value, json};
struct Sound {
    events: Vec<Value>,
}
impl GenerationSound for Sound {
    fn stop_note(&mut self, handle: i32) -> i32 {
        self.events.push(json!(["stop", handle]));
        -1
    }
    fn volume_offset(&mut self, offset: i32) {
        self.events.push(json!(["volume", offset]));
    }
    fn play_note(&mut self, note: i32, period: i32, counter: i32) -> i32 {
        self.events.push(json!(["play", note, period, counter]));
        41
    }
}
struct Host {
    notifications: Vec<i32>,
    queries: Vec<&'static str>,
}
impl EntityScriptHost for Host {
    fn movement_matrix(&mut self, _angles: [i16; 3]) -> Result<FixedMatrix, BehaviorFault> {
        Err(BehaviorFault::MissingMotionServices)
    }
    fn notify(&mut self, _rng: &mut PsxRng, code: i32) -> Result<(), BehaviorFault> {
        self.notifications.push(code);
        Ok(())
    }
}
impl EntityBodyHost for Host {
    fn query_body(
        &mut self,
        _actor: &mut ModelActor,
        _event: EntityBodyEvent,
    ) -> Result<(), BehaviorFault> {
        // The native inputs have empty primitives. Only inherited query entry
        // and policy are compared here; real geometry is a separate backend.
        self.queries.push("geometry_query");
        Ok(())
    }
}
fn state(
    actor: &ModelActor,
    behavior: &EntityBehavior,
    rng: &PsxRng,
    sound: &Sound,
    host: &Host,
) -> Value {
    json!({"interaction_timer":actor.animation.ticks,"behavior_tick":behavior.ticks,"state":behavior.state,
        "triggered":behavior.triggered,"link_enabled":behavior.link_enabled,"behavior_active":behavior.active,
        "animation_enabled":actor.animation.enabled,"action_enabled":actor.animation.action_enabled,
        "clip":actor.animation.selected_clip.unwrap(),"frame":actor.animation.frame_index,
        "part_attributes":actor.animation.parts.iter().map(|part|part.attributes).collect::<Vec<_>>(),
        "part_positions":actor.animation.parts.iter().map(|part|part.coordinate_translation).collect::<Vec<_>>(),
        "context":behavior.context.words,"rng":rng.state(),"audio":sound.events,
        "body_contact":actor.body_contact.current.and_then(lsd_core::ActorContact::peer).map(|peer|peer.identity),"notifications":host.notifications,"queries":host.queries})
}
#[test]
fn original_actor_transform_scene_and_entity_body_notifications_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/entity-body.json"
    ))
    .unwrap();
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let model = Mom::parse(&bytes).unwrap();
    assert!(
        model
            .tmd
            .objects
            .iter()
            .all(|object| object.vertices.is_empty() && object.polygons.is_empty())
    );
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let fields = std::array::from_fn(|i| input["fields"][i].as_i64().unwrap() as i8);
        let mut actor = model.construct_actor().unwrap();
        let mut sound = Sound { events: vec![] };
        let mut behavior =
            EntityBehavior::construct(17, NpcScript::None, fields, &mut actor, &mut sound);
        if !input["active"].as_bool().unwrap() {
            behavior.disable_link(&mut actor, &mut sound);
        }
        for (index, voice) in [(6, 41), (11, 42), (16, 43)] {
            behavior.context.words[index] = voice;
        }
        if input["previous_contact"].as_bool().unwrap() {
            actor.body_contact.current = Some(
                BodyPeer {
                    type_id: 0x1f34,
                    identity: 2,
                }
                .into(),
            );
        }
        let mut rng = PsxRng::new(1);
        let mut host = Host {
            notifications: vec![],
            queries: vec![],
        };
        sound.events.clear();
        assert_eq!(
            state(&actor, &behavior, &rng, &sound, &host),
            case["initial"],
            "initial {number}"
        );
        behavior
            .receive_body(
                &mut actor,
                fields,
                EntityBodyEvent {
                    sender: BodyPeer {
                        type_id: input["sender_type"].as_u64().unwrap() as u32,
                        identity: 1,
                    },
                    code: input["code"].as_i64().unwrap() as i32,
                },
                &mut rng,
                &mut sound,
                &mut host,
            )
            .unwrap();
        assert_eq!(
            state(&actor, &behavior, &rng, &sound, &host),
            case["expected"],
            "body case {number}"
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 2016);
}
