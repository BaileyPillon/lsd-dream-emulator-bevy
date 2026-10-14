#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use lsd_assets::formats::Mom;
use lsd_core::{
    ActorJournalEvent, ActorJournalHost, DayFault, DreamDay, DreamDayEffects, DreamLinkEffects,
    DreamLinks, FlashbackRecord, LinkCatalog, LinkContext, LinkExecution, Mood, MoodContributors,
    PlayerControls, PsxRng, SpawnLocation, actor_journal,
};
use lsd_core::{BodyPeer, EntityBodyEvent, ModelActor, entity_body_plan};
use serde_json::{Value, json};
struct Host {
    actor: ModelActor,
    fields: [i8; 12],
    dream_contact: Option<BodyPeer>,
    catalog: LinkCatalog,
    context: LinkContext,
    trace: Vec<Value>,
    sound: Vec<Value>,
    attached: bool,
    cancel: bool,
}
impl DreamDayEffects for Host {
    fn notify(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _code: i32,
    ) {
    }
    fn attached_actor(&self) -> bool {
        self.attached
    }
    fn current_spawn(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
    ) -> SpawnLocation {
        self.trace.push(json!(["current_spawn"]));
        SpawnLocation {
            chunk: 257,
            tile: 513,
            position: [11, 22, 33],
        }
    }
}
struct LinkEffects<'a> {
    cancel: bool,
    trace: &'a mut Vec<Value>,
    sound: &'a mut Vec<Value>,
}
impl DreamLinkEffects for LinkEffects<'_> {
    fn notify(&mut self, links: &mut DreamLinks, _controls: &mut PlayerControls, event: i32) {
        self.trace.push(json!(["notify", event]));
        if self.cancel {
            links.execution.state = 0;
        }
    }
    fn sound(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        parameters: [i32; 3],
    ) {
        self.sound
            .push(json!(["play", parameters[0], parameters[1], parameters[2]]));
    }
    fn rotate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _absolute: bool,
        _packet: [u32; 3],
    ) {
        unreachable!()
    }
    fn translate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _delta: [i32; 3],
    ) {
        unreachable!()
    }
    fn relocate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _spawn: SpawnLocation,
    ) {
        unreachable!()
    }
}
impl ActorJournalHost for Host {
    fn base_journal(&mut self, event: ActorJournalEvent) {
        if event.code == 4 {
            self.dream_contact = Some(BodyPeer {
                type_id: event.sender_type,
                identity: event.sender_identity,
            });
        }
    }
    fn acknowledge_actor(&mut self, code: i32) -> Option<i32> {
        self.trace.push(json!(["acknowledge", code]));
        let event = EntityBodyEvent {
            sender: BodyPeer {
                type_id: 0x1f34,
                identity: 0,
            },
            code,
        };
        let plan = entity_body_plan(self.fields, event);
        if plan.store_contact {
            self.actor.body_contact.current = Some(event.sender.into());
        }
        plan.notification
    }
    fn capture_flashback(
        &mut self,
        day: &mut DreamDay,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        day.save_flashback(links, controls, rng, 0, 16, self)?;
        Ok(())
    }
    fn dynamic_link(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        let mut effects = LinkEffects {
            cancel: self.cancel,
            trace: &mut self.trace,
            sound: &mut self.sound,
        };
        links.force_dynamic(
            controls,
            &self.catalog,
            &mut self.context,
            rng,
            &mut effects,
        )?;
        Ok(())
    }
}
fn snapshot(day: &DreamDay, links: &DreamLinks, rng: &PsxRng, host: &Host) -> Value {
    let records = day.flashbacks[..day.flashback_count as usize]
        .iter()
        .map(|record| {
            json!([
                record.stage,
                record.spawn.chunk,
                record.spawn.tile,
                record.spawn.position[0],
                record.spawn.position[1],
                record.spawn.position[2],
                record.rotation[0],
                record.rotation[1],
                record.rotation[2],
                record.seconds,
                record.kind,
                record.padding[0],
                record.padding[1],
                record.day
            ])
        })
        .collect::<Vec<_>>();
    json!({"dream_body_contact":host.dream_contact.map(|peer|peer.identity),
        "actor_acknowledged":host.actor.body_contact.current.is_some(),"instances":{"last":day.instances.last.0,"dynamic":day.instances.dynamic,"upper":day.instances.upper,"count":day.instances.count},
        "score":day.instance_score,"state":links.execution.state,"stage":links.execution.stage,"timer":links.execution.dream_timer,
        "cinematic":day.cinematic,"spawn":[links.spawn.chunk,links.spawn.tile,links.spawn.position[0],links.spawn.position[1],links.spawn.position[2]],
        "flashback_records":records,"penalty":host.context.dynamic_penalty,"rng":rng.state(),"trace":host.trace,"sound":host.sound})
}
fn journal_state(value: &Value) -> Value {
    value.clone()
}
#[test]
fn original_coupled_journal_actor_contact_and_recursive_consequences_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/actor-journal-body.json"
    ))
    .unwrap();
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let model = Mom::parse(&bytes).unwrap();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let mut day = DreamDay {
            instances: MoodContributors {
                last: Mood([-3, 3]),
                dynamic: -12,
                upper: 12,
                count: 3,
            },
            instance_score: input["score"].as_i64().unwrap() as i32,
            day: 12,
            cinematic: [3, 4],
            flashback_count: input["flashback_count"].as_i64().unwrap() as i32,
            flashbacks: [FlashbackRecord::default(); 10],
            ..Default::default()
        };
        let mut links = DreamLinks {
            execution: LinkExecution {
                state: input["state"].as_i64().unwrap() as i32,
                stage: 2,
                dream_timer: input["timer"].as_u64().unwrap() as u32,
                flashback: input["flashback"].as_bool().unwrap(),
            },
            spawn: SpawnLocation {
                chunk: 257,
                tile: 513,
                position: [11, 22, 33],
            },
            source_transform: None,
            target_transform: None,
            auto_action: None,
            auto_step: 0,
            auto_target: SpawnLocation::default(),
            time_limit: input["limit"].as_i64().unwrap() as i32,
            default_teleport_transform: false,
        };
        let mut controls = PlayerControls {
            angles: [-101, 2057, 44],
            ..Default::default()
        };
        let mut rng = PsxRng::new(input["seed"].as_u64().unwrap() as u32);
        let mut host = Host {
            actor: model.construct_actor().unwrap(),
            fields: std::array::from_fn(|i| input["fields"][i].as_i64().unwrap() as i8),
            dream_contact: None,
            catalog: support::catalog(&fixture["catalog"]),
            context: LinkContext {
                dynamic_penalty: 17,
                ..Default::default()
            },
            trace: vec![],
            sound: vec![],
            attached: input["attached"].as_bool().unwrap(),
            cancel: input["cancel"].as_bool().unwrap(),
        };
        assert_eq!(
            snapshot(&day, &links, &rng, &host),
            journal_state(&case["initial"]),
            "initial {number}"
        );
        actor_journal(
            ActorJournalEvent {
                sender_type: input["sender_type"].as_u64().unwrap() as u32,
                sender_identity: 1,
                code: input["code"].as_i64().unwrap() as i32,
                fields: std::array::from_fn(|i| input["fields"][i].as_i64().unwrap() as i8),
            },
            &mut day,
            &mut links,
            &mut controls,
            &mut rng,
            &mut host,
        )
        .unwrap();
        assert_eq!(
            snapshot(&day, &links, &rng, &host),
            journal_state(&case["expected"]),
            "journal {number}: {input}"
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 336);
}
