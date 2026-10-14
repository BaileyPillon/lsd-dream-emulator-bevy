use crate::{
    DreamLinks, LinkCatalog, LinkContext, LinkError, PlayerControls, PsxRng, SceneGrid,
    SpawnLocation, rotation_degrees,
};
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mood(pub [i8; 2]);
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoodContributors {
    pub last: Mood,
    pub dynamic: i32,
    pub upper: i32,
    pub count: i32,
}
impl MoodContributors {
    pub fn log(&mut self, value: Mood) {
        self.last = value;
        self.dynamic = self.dynamic.wrapping_add(i32::from(value.0[0]));
        self.upper = self.upper.wrapping_add(i32::from(value.0[1]));
        self.count = self.count.wrapping_add(1);
    }
    pub fn average(&self) -> Result<Mood, DayFault> {
        if self.count == 0 {
            return Ok(self.last);
        }
        let axis = |last: i8, sum: i32| -> Result<i8, DayFault> {
            let value = sum
                .checked_div(self.count)
                .ok_or(DayFault::InvalidDivision)?
                .wrapping_add(i32::from(last) / 3);
            Ok((if value >= 10 {
                -9
            } else if value < -9 {
                9
            } else {
                value
            }) as i8)
        };
        Ok(Mood([
            axis(self.last.0[0], self.dynamic)?,
            axis(self.last.0[1], self.upper)?,
        ]))
    }
}

#[derive(Debug, Clone)]
pub struct DayTables {
    pub grids: [SceneGrid; 14],
    pub moods: [Vec<Mood>; 14],
    pub special_days: [i16; 42],
    pub special_marker: Mood,
    pub colors: [i8; 9],
    pub time_limits: [i16; 14],
    pub save_magic: u32,
    pub start_rotation: [u32; 3],
}
impl DayTables {
    pub fn color(&self, mood: Mood) -> i32 {
        let cell = |value: i8| {
            if value >= 4 {
                2
            } else if value < -3 {
                0
            } else {
                1
            }
        };
        i32::from(self.colors[cell(mood.0[0]) * 3 + cell(mood.0[1])])
    }
    pub fn chunk_mood(&self, stage: i32, key: SpawnLocation) -> Result<Mood, DayFault> {
        let stage = usize::try_from(stage)
            .ok()
            .filter(|&index| index < 14)
            .ok_or(DayFault::InvalidStage)?;
        let coords = key.chunk.to_le_bytes().map(|value| i32::from(value as i8));
        let index = coords[1]
            .wrapping_mul(i32::from(self.grids[stage].columns))
            .wrapping_add(coords[0]);
        usize::try_from(index)
            .ok()
            .and_then(|i| self.moods[stage].get(i))
            .copied()
            .ok_or(DayFault::InvalidMoodIndex)
    }
    pub fn initial_spawn(
        &self,
        catalog: &LinkCatalog,
        context: &mut LinkContext,
        rng: &mut PsxRng,
        mood: Mood,
        tick: i32,
    ) -> Result<(i32, SpawnLocation, i32), DayFault> {
        for stage in 0..14 {
            let grid = self.grids[stage];
            for row in 0..grid.rows {
                for col in 0..grid.columns {
                    let index =
                        (i32::from(row) * i32::from(grid.columns) + i32::from(col)) as usize;
                    if self.moods[stage].get(index).copied() != Some(mood) {
                        continue;
                    }
                    let chunk = i16::from_le_bytes([col as u8, row as u8]);
                    let entries = &catalog.random_spawns[stage];
                    let entry = entries
                        .iter()
                        .find(|entry| entry.chunk == chunk)
                        .or_else(|| {
                            (!entries.is_empty()).then(|| {
                                &entries[(i32::from(chunk) % entries.len() as i32) as usize]
                            })
                        })
                        .ok_or(DayFault::InvalidSpawn)?;
                    let position = *catalog
                        .adjustments
                        .get(usize::from(entry.adjustment))
                        .ok_or(DayFault::InvalidSpawn)?;
                    return Ok((
                        stage as i32,
                        SpawnLocation {
                            chunk: entry.chunk,
                            tile: entry.tile,
                            position,
                        },
                        i32::from(self.time_limits[stage]),
                    ));
                }
            }
        }
        let mut spawn = SpawnLocation::default();
        let stage = catalog.random_spawn(-1, &mut spawn, context, rng)?;
        let _ = tick; // Original fallback random routine ignores this argument.
        Ok((stage, spawn, i32::from(self.time_limits[stage as usize])))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlashbackRecord {
    pub stage: i32,
    pub spawn: SpawnLocation,
    pub rotation: [u32; 3],
    pub seconds: i16,
    pub kind: i16,
    pub padding: [u8; 2],
    pub day: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayFault {
    InvalidStage,
    InvalidMoodIndex,
    InvalidSpawn,
    InvalidDay,
    InvalidFlashback,
    InvalidDivision,
    Link(LinkError),
}
impl From<LinkError> for DayFault {
    fn from(error: LinkError) -> Self {
        Self::Link(error)
    }
}
impl fmt::Display for DayFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported original day input: {self:?}")
    }
}
impl std::error::Error for DayFault {}

pub trait DreamDayEffects {
    fn notify(
        &mut self,
        day: &mut DreamDay,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        code: i32,
    );
    fn attached_actor(&self) -> bool;
    fn current_spawn(
        &mut self,
        day: &mut DreamDay,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
    ) -> SpawnLocation;
}
pub struct DayStartInputs<'a> {
    pub tables: &'a DayTables,
    pub catalog: &'a LinkCatalog,
    pub context: &'a mut LinkContext,
    pub rng: &'a mut PsxRng,
}

#[derive(Debug, Clone)]
pub struct DreamDay {
    pub year: i32,
    pub day: i32,
    pub stored_day: i32,
    pub total_unlock_score: i32,
    pub navigation_score: i32,
    pub instance_score: i32,
    pub history: [Mood; 365],
    pub chunks: MoodContributors,
    pub instances: MoodContributors,
    pub cinematic: [i16; 2],
    pub flashbacks: [FlashbackRecord; 10],
    pub flashback_count: i32,
    pub flashback_index: i32,
    pub is_new_game: i32,
    pub screen_shake: bool,
    pub save_magic: u32,
    pub reserved_flags: [u8; 8],
    pub reserved67c: i32,
    pub reserved680: i32,
    pub reserved684: [u8; 500],
}
impl Default for DreamDay {
    fn default() -> Self {
        Self {
            year: 0,
            day: 0,
            stored_day: 0,
            total_unlock_score: 0,
            navigation_score: 0,
            instance_score: 0,
            history: [Mood::default(); 365],
            chunks: MoodContributors::default(),
            instances: MoodContributors::default(),
            cinematic: [0; 2],
            flashbacks: [FlashbackRecord::default(); 10],
            flashback_count: 0,
            flashback_index: 0,
            is_new_game: 1,
            screen_shake: true,
            save_magic: 0,
            reserved_flags: [0; 8],
            reserved67c: 0,
            reserved680: 0,
            reserved684: [0; 500],
        }
    }
}
impl DreamDay {
    pub fn init_new_game(&mut self, tables: &DayTables, context: &mut LinkContext) {
        self.year = 0;
        self.day = 0;
        self.total_unlock_score = 0;
        self.navigation_score = 0;
        self.instance_score = 0;
        self.flashback_count = 0;
        self.screen_shake = true;
        self.save_magic = tables.save_magic;
        self.reserved_flags[0] = 0;
        self.reserved_flags[7] = 0;
        self.reserved67c = 0;
        self.reserved680 = 0;
        self.reserved684.fill(0);
        context.navigation = [0; 30];
        context.dynamic_penalty = 0;
        // Original initialization retains mood history, old flashback bytes and stored-day/cinematic values.
    }
    pub fn previous_mood(&self, previous_only: bool) -> Result<Mood, DayFault> {
        if !(0..365).contains(&self.day) {
            return Err(DayFault::InvalidDay);
        }
        if previous_only {
            if self.year == 0 && self.day == 0 {
                return Ok(Mood::default());
            }
            if self.day == 0 {
                // The original pointer indexes immediately before the history at the year boundary.
                let bytes = self.instance_score.to_le_bytes();
                return Ok(Mood([bytes[2] as i8, bytes[3] as i8]));
            }
            return Ok(self.history[self.day as usize - 1]);
        }
        let count = if self.year != 0 {
            365
        } else {
            self.day as usize
        };
        if count == 0 {
            return Ok(Mood::default());
        }
        let mut sum = [0_i32; 2];
        for mood in &self.history[..count] {
            for (total, value) in sum.iter_mut().zip(mood.0) {
                *total = total.wrapping_add(i32::from(value));
            }
        }
        Ok(Mood(sum.map(|value| (value / count as i32) as i8)))
    }
    pub fn chart(&self) -> Result<Mood, DayFault> {
        let chunk = self.chunks.average()?;
        let instance = if self.instances.count == 0 {
            chunk
        } else {
            self.instances.average()?
        };
        Ok(Mood(std::array::from_fn(|i| {
            ((i32::from(chunk.0[i]) + i32::from(instance.0[i])) / 2) as i8
        })))
    }
    pub fn log_chunk(
        &mut self,
        tables: &DayTables,
        stage: i32,
        key: SpawnLocation,
    ) -> Result<(), DayFault> {
        self.chunks.log(tables.chunk_mood(stage, key)?);
        Ok(())
    }
    pub fn start(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        inputs: DayStartInputs<'_>,
        effects: &mut impl DreamDayEffects,
    ) -> Result<i32, DayFault> {
        let DayStartInputs {
            tables,
            catalog,
            context,
            rng,
        } = inputs;
        self.flashback_index = 0;
        links.execution.dream_timer = 0;
        self.stored_day = self.day;
        if links.execution.flashback {
            self.load_flashback(links, controls, false, effects)?;
        } else {
            let special = tables
                .special_days
                .iter()
                .position(|&day| i32::from(day) == self.day.wrapping_add(1));
            if let Some(index) = special {
                self.cinematic = [(index % 12) as i16, (rng.next_u15() % 6) as i16];
            }
            self.chunks = MoodContributors::default();
            self.instances = MoodContributors::default();
            if special.is_some() {
                self.chunks.log(tables.special_marker);
                self.instances.log(tables.special_marker);
                return Ok(-1);
            }
            let (stage, spawn, seconds) =
                tables.initial_spawn(catalog, context, rng, self.previous_mood(true)?, self.day)?;
            links.execution.stage = stage;
            links.spawn = spawn;
            links.time_limit = if seconds >= 0 {
                seconds.wrapping_mul(15)
            } else {
                seconds
            };
            links.execution.state = 11;
        }
        Ok(links.execution.stage)
    }
    pub fn advance(&mut self) -> i32 {
        self.day = self.day.wrapping_add(1);
        if self.day > 364 {
            self.day = 0;
            self.year = self.year.wrapping_add(1);
        }
        self.day
    }
    pub fn end(
        &mut self,
        links: &DreamLinks,
        tables: &DayTables,
        context: &mut LinkContext,
        mode: i32,
    ) -> Result<i32, DayFault> {
        self.day = self.stored_day;
        if !links.execution.flashback && mode == 0 {
            let mut score = context
                .navigation
                .iter()
                .filter(|&&value| value != 0)
                .count() as i32
                * 1_000_000;
            if score > 29_999_999 {
                score = 50_000_000;
            }
            score = score.wrapping_sub(context.dynamic_penalty.wrapping_mul(0x2b10));
            self.navigation_score = score.max(0);
            self.instance_score = self.instance_score.clamp(0, 50_000_000);
            self.total_unlock_score = self.navigation_score.wrapping_add(self.instance_score);
            let index = usize::try_from(self.day)
                .ok()
                .filter(|&i| i < 365)
                .ok_or(DayFault::InvalidDay)?;
            self.history[index] = self.chart()?;
            self.advance();
        } else if mode == 2 {
            self.init_new_game(tables, context);
            self.is_new_game = 1;
        }
        Ok(i32::from(links.execution.flashback))
    }
    pub fn load_flashback(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        notify: bool,
        effects: &mut impl DreamDayEffects,
    ) -> Result<bool, DayFault> {
        if self.flashback_index >= self.flashback_count {
            return Ok(false);
        }
        let index = usize::try_from(self.flashback_index)
            .ok()
            .filter(|&i| i < 10)
            .ok_or(DayFault::InvalidFlashback)?;
        links.execution.state = 14;
        if notify {
            effects.notify(self, links, controls, 14);
        }
        // Original reads the indexed slot after notification, retaining callback mutation effects.
        let record = self.flashbacks[index];
        self.day = record.day;
        links.execution.stage = record.stage;
        links.spawn = record.spawn;
        Ok(true)
    }
    pub fn attach_flashback(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
    ) -> Result<(), DayFault> {
        if links.execution.state == 14 {
            let index = usize::try_from(self.flashback_index)
                .ok()
                .filter(|&i| i < 10)
                .ok_or(DayFault::InvalidFlashback)?;
            let record = self.flashbacks[index];
            controls.apply_rotation_packet(true, record.rotation)?;
            let seconds = i32::from(record.seconds).wrapping_add(4);
            links.time_limit = if seconds >= 0 {
                seconds.wrapping_mul(15)
            } else {
                seconds
            };
            self.flashback_index = self.flashback_index.wrapping_add(1);
        }
        Ok(())
    }
    pub fn save_flashback(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
        kind: i32,
        seconds: i32,
        effects: &mut impl DreamDayEffects,
    ) -> Result<bool, DayFault> {
        if !effects.attached_actor() || !rng.next_u15().is_multiple_of(3) {
            return Ok(false);
        }
        if !(0..=10).contains(&self.flashback_count) {
            return Err(DayFault::InvalidFlashback);
        }
        let key = effects.current_spawn(self, links, controls);
        let rotation =
            rotation_degrees(controls.angles).map(|angle| (1 << 16) | u32::from(angle as u16));
        let index = if self.flashback_count < 10 {
            let index = self.flashback_count as usize;
            self.flashback_count += 1;
            index
        } else {
            (links.execution.dream_timer % 9) as usize
        };
        let padding = self.flashbacks[index].padding;
        self.flashbacks[index] = FlashbackRecord {
            stage: links.execution.stage,
            spawn: key,
            rotation,
            seconds: seconds as i16,
            kind: kind as i16,
            padding,
            day: self.day,
        };
        Ok(true)
    }
    /// True runs original view/move callbacks; false skips them on ignored/expired ticks.
    pub fn tick(
        &mut self,
        links: &mut DreamLinks,
        controls: &mut PlayerControls,
        rng: &mut PsxRng,
        code: i32,
        effects: &mut impl DreamDayEffects,
    ) -> Result<bool, DayFault> {
        if code != 2 {
            return Ok(false);
        }
        let previous = links.execution.dream_timer;
        links.execution.dream_timer = previous.wrapping_add(1);
        if previous < links.time_limit as u32 {
            return Ok(true);
        }
        if links.execution.flashback {
            if links.execution.state != 0 || self.load_flashback(links, controls, true, effects)? {
                links.execution.dream_timer = 0;
                return Ok(false);
            }
        } else {
            self.save_flashback(links, controls, rng, 0, 16, effects)?;
        }
        effects.notify(self, links, controls, 10);
        links.execution.dream_timer = 0;
        Ok(false)
    }
}
