#[allow(dead_code)]
#[path = "../../lsd-core/tests/support/links.rs"]
mod support;
use lsd_assets::{
    ActorJournalBindings, ActorJournalDispatcher, ActorJournalServices, EncounterEventHost,
    EncounterNotification, EncounterNotificationKind, StageSessionEvent,
};
use lsd_core::{
    DayTables, DreamDay, DreamLinks, LinkContext, LinkExecution, LinkPalette, MapLinkCommand,
    MapLinkPolicy, Mood, PlayerControls, PsxRng, SceneGrid, SpawnLocation,
};
use serde_json::Value;
fn tables() -> DayTables {
    DayTables {
        grids: [SceneGrid {
            columns: 2,
            rows: 2,
            mode: 0,
        }; 14],
        moods: std::array::from_fn(|_| vec![Mood([0, 0]); 4]),
        special_days: [0; 42],
        special_marker: Mood([-4, 6]),
        colors: [0, 1, 2, 3, 4, 5, 6, 7, 0],
        time_limits: [60; 14],
        save_magic: 0,
        start_rotation: [65536; 3],
    }
}
#[test]
fn actor_journal_adapter_starts_fades_and_preserves_cancelled_source() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../lsd-core/tests/fixtures/actor-journal.json"
    ))
    .unwrap();
    let catalog = support::catalog(&fixture["catalog"]);
    let tables = tables();
    for busy in [false, true] {
        let mut day = DreamDay::default();
        let key = SpawnLocation {
            chunk: 0,
            tile: 0,
            position: [11, 22, 33],
        };
        let mut links = DreamLinks {
            execution: LinkExecution {
                state: 0,
                stage: 0,
                dream_timer: 123,
                flashback: false,
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
        let mut controls = PlayerControls::default();
        let mut rng = PsxRng::new(1);
        let mut context = LinkContext::default();
        let mut map = MapLinkPolicy {
            state: if busy { 4 } else { 0 },
            tick: 0,
            location: 0,
            move_callback: 1,
            reset_view_pending: false,
            view_active: false,
        };
        let mut commands = Vec::new();
        let mut sounds = Vec::new();
        let mut pending = Vec::new();
        let mut events = Vec::new();
        let services = ActorJournalServices::new(
            &catalog,
            ActorJournalBindings {
                context: &mut context,
                map: &mut map,
                palette: LinkPalette {
                    dream: 0,
                    flashback: 0,
                },
                commands: &mut commands,
                sounds: &mut sounds,
                current_key: key,
                attached: true,
                pending: &mut pending,
                events: &mut events,
            },
        );
        let mut dispatcher = ActorJournalDispatcher {
            day: &mut day,
            tables: &tables,
            links: &mut links,
            controls: &mut controls,
            services,
        };
        dispatcher
            .dispatch(
                EncounterNotification {
                    serial: 1,
                    entity: 5,
                    kind: EncounterNotificationKind::Notify(10),
                },
                [-4, 4, -50, 0, 0, 0, 1, -14, 1, -2, 10, -1],
                &mut rng,
            )
            .unwrap();
        assert_eq!(links.execution.stage, if busy { 0 } else { 13 });
        assert_eq!(links.execution.state, if busy { 0 } else { 12 });
        assert_eq!(context.dynamic_penalty, 1);
        assert_ne!(links.spawn, key);
        if busy {
            assert!(commands.is_empty());
            assert!(sounds.is_empty());
        } else {
            // The map observes the negative requested stage before execute_link
            // commits the selected destination, so it chooses its state-six fade.
            assert_eq!(map.state, 6);
            assert!(controls.input_locked);
            assert!(
                commands
                    .iter()
                    .any(|command| matches!(command, MapLinkCommand::Fade { color_step: 30, .. }))
            );
            assert_eq!(sounds, vec![[144, 110, 110]]);
        }
        assert_eq!(
            events,
            vec![StageSessionEvent::ActorJournal {
                entity: 5,
                code: 10,
                applied: true
            }]
        );
    }
}
