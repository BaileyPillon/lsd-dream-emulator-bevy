use lsd_assets::{
    ActorJournalBindings, ActorJournalDispatcher, ActorJournalPlayerMotion, ActorJournalServices,
};
use lsd_core::{
    BodyContactState, DayTables, DreamDay, DreamLinks, LinkContext, LinkExecution, LinkPalette,
    MapLinkPolicy, Mood, PlayerControls, PsxMathTables, SceneGrid, SpawnLocation,
};
use serde_json::Value;
pub struct DayRuntime {
    pub scene_scale: Option<lsd_core::SceneScale>,
    pub allow_journal: bool,
    pub journal_audio: Vec<[i32; 3]>,
    pub day: DreamDay,
    pub tables: DayTables,
    pub local: [i32; 3],
}
impl DayRuntime {
    pub fn new(colors: [i8; 9]) -> Self {
        Self {
            scene_scale: None,
            allow_journal: false,
            journal_audio: vec![],
            day: DreamDay::default(),
            local: [13, 17, 19],
            tables: DayTables {
                grids: [SceneGrid {
                    columns: 1,
                    rows: 1,
                    mode: 0,
                }; 14],
                moods: std::array::from_fn(|_| vec![Mood([0, 0])]),
                special_days: [0; 42],
                special_marker: Mood([0, 0]),
                colors,
                time_limits: [60; 14],
                save_magic: 0,
                start_rotation: [65536; 3],
            },
        }
    }
    pub fn dispatch<R>(
        &mut self,
        math: &PsxMathTables,
        f: impl FnOnce(&mut ActorJournalDispatcher<'_>) -> R,
    ) -> R {
        let value: Value =
            serde_json::from_str(include_str!("../../../lsd-core/tests/fixtures/links.json"))
                .unwrap();
        let catalog = super::support::catalog(&value["catalog"]);
        let mut context = LinkContext::default();
        let mut map = MapLinkPolicy {
            state: 0,
            tick: 0,
            location: 0,
            move_callback: 1,
            reset_view_pending: false,
            view_active: false,
        };
        let mut links = DreamLinks {
            execution: LinkExecution {
                state: 0,
                stage: 0,
                dream_timer: 123,
                flashback: false,
            },
            spawn: SpawnLocation::default(),
            source_transform: None,
            target_transform: None,
            auto_action: None,
            auto_step: 0,
            auto_target: SpawnLocation::default(),
            time_limit: 3600,
            default_teleport_transform: false,
        };
        let mut controls = PlayerControls::default();
        let mut commands = vec![];
        let mut sounds = vec![];
        let mut pending = vec![];
        let mut events = vec![];
        let mut contact = BodyContactState { current: None };
        let mut distance = 0;
        let mut services = ActorJournalServices::new(
            &catalog,
            ActorJournalBindings {
                context: &mut context,
                map: &mut map,
                palette: LinkPalette {
                    dream: 99,
                    flashback: -7,
                },
                commands: &mut commands,
                sounds: &mut sounds,
                current_key: SpawnLocation::default(),
                attached: true,
                pending: &mut pending,
                events: &mut events,
                body_contact: &mut contact,
            },
        );
        services.player_motion = Some(ActorJournalPlayerMotion {
            position: &mut self.local,
            move_distance: &mut distance,
            math,
        });
        services.scene_scale = self.scene_scale.as_mut();
        let mut dispatcher = ActorJournalDispatcher {
            day: &mut self.day,
            tables: &self.tables,
            links: &mut links,
            controls: &mut controls,
            services,
        };
        let output = f(&mut dispatcher);
        if !self.allow_journal {
            assert!(
                commands.is_empty() && sounds.is_empty() && pending.is_empty() && events.is_empty()
            );
        }
        self.journal_audio = sounds;
        output
    }
}
