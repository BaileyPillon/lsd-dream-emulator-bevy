use crate::formats::{DecodeError, Vab};
use lsd_core::{
    AudioNoteDriver, AudioRegisterWrite, AudioVolumeFault, GenerationSound, NoteSetupFault,
    NoteStart, SpuPitchTable, VoiceNoteDefinition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundBankBinding {
    definitions: Vec<Vec<VoiceNoteDefinition>>,
    pub pitch_offset: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundBankFault {
    MissingTone,
    Note(NoteSetupFault),
    Volume(AudioVolumeFault),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioImmediateCommand {
    Write(AudioRegisterWrite),
    DisableNoise(u32),
}

impl SoundBankBinding {
    /// Mirrors the sound object's logical program_count table. The complete
    /// sparse VAB remains available separately for SDK sequence playback.
    pub fn from_vab(vab: &Vab<'_>, bank: i16, spu_base: u32) -> Result<Self, DecodeError> {
        let mut definitions = Vec::new();
        for program in 0..vab.header.program_count {
            let mut tones = Vec::new();
            let packed = vab
                .programs
                .iter()
                .take_while(|p| u16::from(p.id) < program)
                .count();
            if let Some(definition) = vab
                .programs
                .get(packed)
                .filter(|p| u16::from(p.id) < vab.header.program_count)
            {
                for tone in 0..definition.tones.len() {
                    let mut note = vab.prepare_voice_note(bank, definition.id, tone, spu_base)?;
                    note.program = program as i16;
                    note.program_volume = vab.program_attributes[usize::from(program)][1];
                    note.program_pan = vab.program_attributes[usize::from(program)][4];
                    tones.push(note);
                }
            }
            definitions.push(tones);
        }
        Ok(Self {
            definitions,
            pitch_offset: -24,
        })
    }

    /// Original volume_offset selector actually changes pitch by octaves.
    pub fn set_offset_level(&mut self, level: i32) {
        self.pitch_offset = level.wrapping_mul(12).wrapping_sub(24) as i16;
    }

    pub fn play_note(
        &self,
        driver: &mut AudioNoteDriver,
        table: &SpuPitchTable,
        note: i32,
        first: i32,
        target: i32,
    ) -> Result<NoteStart, SoundBankFault> {
        if note < 0 {
            return Ok(NoteStart {
                voice: None,
                disable_noise: None,
            });
        }
        let program = usize::try_from(note >> 4).map_err(|_| SoundBankFault::MissingTone)?;
        let definition = self
            .definitions
            .get(program)
            .and_then(|tones| tones.get((note & 15) as usize))
            .ok_or(SoundBankFault::MissingTone)?;
        let key = u16::from(definition.attributes[4]).wrapping_add(self.pitch_offset as u16) as i16;
        let result = driver
            .key_on(
                definition,
                table,
                i32::from(key),
                i32::from(definition.attributes[5]),
                first as i16 as i32,
                first as i16 as i32,
            )
            .map_err(SoundBankFault::Note)?;
        if let Some(voice) = result.voice {
            driver
                .auto_volume(i32::from(voice), first, target, 2)
                .map_err(SoundBankFault::Volume)?;
        }
        Ok(result)
    }

    pub fn stop_note(&self, driver: &mut AudioNoteDriver, handle: i32) -> Vec<AudioRegisterWrite> {
        if handle < 24 {
            driver.pool.release(handle);
            Vec::new()
        } else {
            driver.stop_all()
        }
    }
}

/// Original sound-object compatibility adapter. The caller executes device
/// commands and advances the real SPU; this adapter alone has no audio output.
pub struct SoftwareSound<'a> {
    pub bank: &'a mut SoundBankBinding,
    pub driver: &'a mut AudioNoteDriver,
    pub pitch: &'a SpuPitchTable,
    pub commands: &'a mut Vec<AudioImmediateCommand>,
    pub faults: &'a mut Vec<SoundBankFault>,
}

impl GenerationSound for SoftwareSound<'_> {
    fn stop_note(&mut self, handle: i32) -> i32 {
        self.commands.extend(
            self.bank
                .stop_note(self.driver, handle)
                .into_iter()
                .map(AudioImmediateCommand::Write),
        );
        -1
    }
    fn volume_offset(&mut self, offset: i32) {
        self.bank.set_offset_level(offset);
    }
    fn play_note(&mut self, note: i32, first: i32, target: i32) -> i32 {
        match self
            .bank
            .play_note(self.driver, self.pitch, note, first, target)
        {
            Ok(result) => {
                if let Some(mask) = result.disable_noise {
                    self.commands
                        .push(AudioImmediateCommand::DisableNoise(mask));
                }
                result.handle()
            }
            Err(error) => {
                self.faults.push(error);
                -1
            }
        }
    }
}
