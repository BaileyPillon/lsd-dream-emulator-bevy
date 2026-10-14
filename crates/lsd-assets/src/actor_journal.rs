use crate::{
    EncounterEventHost, EncounterNotification, EncounterNotificationKind, StageSessionEvent,
};
use lsd_core::{
    ActorJournalEvent, ActorJournalHost, DayFault, DreamDay, DreamDayEffects, DreamLinkEffects,
    DreamLinks, LinkCatalog, LinkContext, LinkPalette, MapLinkCommand, MapLinkPolicy,
    PlayerControls, PsxRng, SpawnLocation, actor_journal,
};

pub struct ActorJournalServices<'a> {
    pub catalog: &'a LinkCatalog,
    pub context: &'a mut LinkContext,
    pub map: &'a mut MapLinkPolicy,
    pub palette: LinkPalette,
    pub commands: &'a mut Vec<MapLinkCommand>,
    pub sounds: &'a mut Vec<[i32; 3]>,
    pub current_key: SpawnLocation,
    pub attached: bool,
    pub pending: &'a mut Vec<EncounterNotification>,
    pub events: &'a mut Vec<StageSessionEvent>,
    source: Option<EncounterNotification>,
}
impl<'a> ActorJournalServices<'a> {
    pub fn new(catalog: &'a LinkCatalog, bindings: ActorJournalBindings<'a>) -> Self {
        Self {
            catalog,
            context: bindings.context,
            map: bindings.map,
            palette: bindings.palette,
            commands: bindings.commands,
            sounds: bindings.sounds,
            current_key: bindings.current_key,
            attached: bindings.attached,
            pending: bindings.pending,
            events: bindings.events,
            source: None,
        }
    }
}
pub struct ActorJournalBindings<'a> {
    pub context: &'a mut LinkContext,
    pub map: &'a mut MapLinkPolicy,
    pub palette: LinkPalette,
    pub commands: &'a mut Vec<MapLinkCommand>,
    pub sounds: &'a mut Vec<[i32; 3]>,
    pub current_key: SpawnLocation,
    pub attached: bool,
    pub pending: &'a mut Vec<EncounterNotification>,
    pub events: &'a mut Vec<StageSessionEvent>,
}
struct LinkEffects<'a> {
    map: &'a mut MapLinkPolicy,
    palette: LinkPalette,
    commands: &'a mut Vec<MapLinkCommand>,
    sounds: &'a mut Vec<[i32; 3]>,
    error: Option<lsd_core::LinkError>,
}
impl DreamLinkEffects for LinkEffects<'_> {
    fn notify(&mut self, links: &mut DreamLinks, controls: &mut PlayerControls, event: i32) {
        match self.map.notify(links, controls, event, self.palette) {
            Ok(commands) => self.commands.extend(commands),
            Err(error) => self.error = Some(error),
        }
    }
    fn sound(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        parameters: [i32; 3],
    ) {
        self.sounds.push(parameters);
    }
    fn rotate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _absolute: bool,
        _packet: [u32; 3],
    ) {
        unreachable!("direct actor dynamic link does not rotate")
    }
    fn translate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _delta: [i32; 3],
    ) {
        unreachable!("direct actor dynamic link does not translate")
    }
    fn relocate(
        &mut self,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        _spawn: SpawnLocation,
    ) {
        unreachable!("direct actor dynamic link does not relocate")
    }
}
impl DreamDayEffects for ActorJournalServices<'_> {
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
        self.current_key
    }
}
impl ActorJournalHost for ActorJournalServices<'_> {
    // The inherited journal/body chain is separate; supplied valid policy
    // graphs have no base mutation for these events.
    fn base_journal(&mut self, _code: i32) {}
    fn acknowledge_actor(&mut self, code: i32) {
        if let Some(mut source) = self.source {
            source.kind = EncounterNotificationKind::Acknowledge(code);
            self.pending.push(source);
        }
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
            map: self.map,
            palette: self.palette,
            commands: self.commands,
            sounds: self.sounds,
            error: None,
        };
        links.force_dynamic(controls, self.catalog, self.context, rng, &mut effects)?;
        if let Some(error) = effects.error {
            return Err(error.into());
        }
        Ok(())
    }
}
pub struct ActorJournalDispatcher<'a> {
    pub day: &'a mut DreamDay,
    pub tables: &'a lsd_core::DayTables,
    pub links: &'a mut DreamLinks,
    pub controls: &'a mut PlayerControls,
    pub services: ActorJournalServices<'a>,
}
impl EncounterEventHost for ActorJournalDispatcher<'_> {
    fn dispatch(
        &mut self,
        event: EncounterNotification,
        fields: [i8; 12],
        rng: &mut PsxRng,
    ) -> Result<(), DayFault> {
        let EncounterNotificationKind::Notify(code) = event.kind else {
            self.services.pending.push(event);
            return Ok(());
        };
        self.services.source = Some(event);
        self.services.palette.dream = self.tables.color(self.day.chart()?);
        let applied = self.links.execution.state == 0
            && (matches!(code, 4 | 10)
                || (!self.links.execution.flashback && matches!(code, 9 | 11 | 12)));
        actor_journal(
            ActorJournalEvent {
                sender_type: 0x1f234,
                code,
                fields,
            },
            self.day,
            self.links,
            self.controls,
            rng,
            &mut self.services,
        )?;
        self.services.events.push(StageSessionEvent::ActorJournal {
            entity: event.entity,
            code,
            applied,
        });
        Ok(())
    }
}
