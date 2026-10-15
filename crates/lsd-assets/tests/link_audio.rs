#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod links;
#[allow(dead_code)]
#[path = "support/link_audio.rs"]
mod support;
use lsd_assets::SceneAudio;
use lsd_core::{
    ActorJournalEvent, ActorJournalHost, DayFault, DreamDay, DreamLinkEffects, DreamLinks,
    GenerationSound, LinkContext, LinkExecution, PlayerControls, PsxRng, SpawnLocation,
};
use serde_json::{Value, json};

struct Effects {
    audio: SceneAudio,
    trace: Vec<Value>,
    cancel: bool,
    prenote: bool,
}
impl Effects {
    fn play(&mut self, state: &DreamLinks, params: [i32; 3]) {
        self.trace.push(json!([
            "sound",
            params[0],
            params[1],
            params[2],
            state.execution.stage as u32,
            state.execution.state
        ]));
        let handle = self.audio.play_note(params[0], params[1], params[2]);
        self.trace.push(json!(["sound_return", handle]));
    }
}
impl DreamLinkEffects for Effects {
    fn notify(&mut self, state: &mut DreamLinks, _: &mut PlayerControls, event: i32) {
        self.trace
            .push(json!(["notify", event, state.execution.stage as u32]));
        if self.prenote {
            self.play(state, [144, 110, 110]);
        }
        if self.cancel {
            state.execution.state = 0;
        }
    }
    fn sound(&mut self, state: &mut DreamLinks, _: &mut PlayerControls, parameters: [i32; 3]) {
        self.play(state, parameters);
    }
    fn rotate(&mut self, _: &mut DreamLinks, _: &mut PlayerControls, _: bool, _: [u32; 3]) {
        unreachable!()
    }
    fn translate(&mut self, _: &mut DreamLinks, _: &mut PlayerControls, _: [i32; 3]) {
        unreachable!()
    }
    fn relocate(&mut self, _: &mut DreamLinks, _: &mut PlayerControls, _: SpawnLocation) {
        unreachable!()
    }
}
struct Journal {
    effects: Effects,
    catalog: lsd_core::LinkCatalog,
    context: LinkContext,
}
impl ActorJournalHost for Journal {
    fn base_journal(&mut self, _: ActorJournalEvent) {}
    fn acknowledge_actor(&mut self, _: i32) -> Option<i32> {
        unreachable!()
    }
    fn capture_flashback(
        &mut self,
        _: &mut DreamDay,
        _: &mut DreamLinks,
        _: &mut PlayerControls,
        _: &mut PsxRng,
    ) -> Result<(), DayFault> {
        unreachable!()
    }
    fn dynamic_link(
        &mut self,
        state: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        state.force_dynamic(
            controls,
            &self.catalog,
            &mut self.context,
            rng,
            &mut self.effects,
        )?;
        Ok(())
    }
}
fn compare(fixture: &Value, header: &[u8], body: &[u8]) {
    let source: Value =
        serde_json::from_str(include_str!("../../lsd-core/tests/fixtures/links.json")).unwrap();
    for (index, row) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let mut audio = support::runtime(fixture, header, body);
        assert_eq!(
            audio.driver_snapshot().unwrap(),
            support::driver(&row["before"])
        );
        audio.volume_offset(row["offset"].as_i64().unwrap() as i32);
        let key = SpawnLocation {
            chunk: 257,
            tile: 513,
            position: [11, 22, 33],
        };
        let mut state = DreamLinks {
            execution: LinkExecution {
                state: 0,
                stage: 2,
                dream_timer: 123,
                flashback: row["flashback"].as_bool().unwrap(),
            },
            spawn: key,
            source_transform: None,
            target_transform: None,
            auto_action: None,
            auto_step: 0,
            auto_target: key,
            time_limit: 3600,
            default_teleport_transform: false,
        };
        let mut host = Journal {
            effects: Effects {
                audio: audio.clone(),
                trace: Vec::new(),
                cancel: row["cancel"].as_bool().unwrap(),
                prenote: row["prenote"].as_bool().unwrap(),
            },
            catalog: links::catalog(&source["catalog"]),
            context: LinkContext::default(),
        };
        let mut controls = PlayerControls::default();
        if row["route"] == "execute" {
            assert_eq!(
                state.execute(&mut controls, 3, 12, true, &mut host.effects),
                !row["cancel"].as_bool().unwrap()
            );
        } else {
            lsd_core::actor_journal(
                ActorJournalEvent {
                    sender_type: 0x1f234,
                    sender_identity: 1,
                    code: 10,
                    fields: [-4, 4, -50, 0, 0, 1, 1, -14, 1, 1, 10, 0],
                },
                &mut DreamDay::default(),
                &mut state,
                &mut controls,
                &mut PsxRng::new(1),
                &mut host,
            )
            .unwrap();
        }
        let trace: Vec<_> = row["trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| matches!(e[0].as_str(), Some("notify" | "sound" | "sound_return")))
            .cloned()
            .collect();
        assert_eq!(host.effects.trace, trace, "case {index}");
        assert_eq!(
            state.execution.state as u32,
            row["link_state"].as_u64().unwrap() as u32,
            "case {index}"
        );
        assert_eq!(
            state.execution.stage as u32,
            row["stage"].as_u64().unwrap() as u32,
            "case {index}"
        );
        assert_eq!(
            state.execution.dream_timer,
            row["timer"].as_u64().unwrap() as u32,
            "case {index}"
        );
        assert_eq!(
            audio.driver_snapshot().unwrap(),
            support::driver(&row["after"]),
            "case {index}"
        );
        assert!(audio.snapshot().fault.is_none());
    }
}

#[test]
fn original_link_and_journal_sound_chains_match_order_handles_and_complete_sdk_state() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/link-audio.json")).unwrap();
    compare(
        &fixture,
        &support::decode(fixture["header_hex"].as_str().unwrap()),
        &support::decode(fixture["body_hex"].as_str().unwrap()),
    );
}

#[test]
#[ignore = "requires local original SE bank and fresh original link/journal/SDK observations"]
fn original_se_sound_chains_match_the_shared_scene_driver() {
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("LSD_LINK_AUDIO_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    let root = std::path::PathBuf::from(std::env::var_os("LSD_GAME_FILES").unwrap());
    let path = root.join(fixture["path"].as_str().unwrap());
    let header = std::fs::read(&path).unwrap();
    let body = std::fs::read(path.with_extension("VB")).unwrap();
    assert_eq!(lsd_assets::sha256_hex(&header), fixture["header_sha256"]);
    assert_eq!(lsd_assets::sha256_hex(&body), fixture["body_sha256"]);
    compare(&fixture, &header, &body);
}
