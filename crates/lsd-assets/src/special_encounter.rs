use crate::{PsxExeHeader, formats::Mom, is_canonical_reference};
use lsd_core::PsxAimTables;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Shared special model; each scene owns one dormant instance of entity 98.
#[derive(Debug, Clone)]
pub struct SpecialEncounterAssets {
    pub model: Arc<Mom>,
    pub path: PathBuf,
    pub offset: [i32; 3],
    pub aim: PsxAimTables,
}
pub fn load_special_encounter_assets(
    executable: &[u8],
    files: &Path,
) -> Result<SpecialEncounterAssets, Box<dyn std::error::Error>> {
    if !is_canonical_reference(executable) {
        return Err("Special encounter requires canonical SLPS_015.56".into());
    }
    let header = PsxExeHeader::parse(executable)?;
    let bytes = |address: u32, count: usize| {
        let at = address.checked_sub(header.load_address)? as usize + 0x800;
        executable.get(at..at.checked_add(count)?)
    };
    let path_bytes = bytes(0x8001_186c, 64).ok_or("Invalid special model path extent")?;
    let end = path_bytes
        .iter()
        .position(|&v| v == 0)
        .ok_or("Unterminated special model path")?;
    let relative = std::str::from_utf8(&path_bytes[..end])?.replace('\\', "/");
    let path = PathBuf::from("CDI").join(relative);
    let offset = bytes(0x8008_8d30, 12).ok_or("Invalid special model placement extent")?;
    let offset =
        std::array::from_fn(|i| i32::from_le_bytes(offset[i * 4..i * 4 + 4].try_into().unwrap()));
    let table = bytes(0x8006_b69c, 1025 * 2).ok_or("Invalid aim table extent")?;
    let aim = PsxAimTables {
        atan: Box::new(std::array::from_fn(|i| {
            i16::from_le_bytes(table[i * 2..i * 2 + 2].try_into().unwrap())
        })),
    };
    Ok(SpecialEncounterAssets {
        model: Arc::new(Mom::parse(&std::fs::read(files.join(&path))?)?),
        path,
        offset,
        aim,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct SpecialEncounter {
    pub assets: SpecialEncounterAssets,
    pub serial: Option<u64>,
    pub player_position: [i32; 3],
    pub player_matrix: lsd_core::FixedMatrix,
}
impl SpecialEncounter {
    pub fn activate(
        &mut self,
        actors: &mut [crate::SceneActor],
        members: &mut lsd_core::FrameSubscribers<u64>,
        changes: &mut Vec<crate::EncounterSubscription>,
        sound: &mut impl lsd_core::GenerationSound,
    ) -> Result<(), lsd_core::AimFault> {
        let serial = self.serial.ok_or(lsd_core::AimFault::MissingActor)?;
        let actor = actors
            .iter_mut()
            .find(|v| v.serial == serial)
            .ok_or(lsd_core::AimFault::MissingActor)?;
        if members.contains(serial) {
            actor.behavior.disable_link(&mut actor.actor, sound);
            actor.behavior_active = false;
            members.remove(serial);
            changes.push(crate::EncounterSubscription::Detach(serial));
        }
        let position = self.player_matrix.apply_long(self.assets.offset);
        let position = std::array::from_fn(|i| position[i].wrapping_add(self.player_position[i]));
        actor.actor.translate(position, true);
        actor
            .behavior
            .start(actor.properties.fields, &mut actor.actor);
        actor.behavior_active = actor.behavior.active != 0;
        members.subscribe(serial);
        changes.push(crate::EncounterSubscription::Attach(serial));
        actor.actor.pose.angles =
            self.assets
                .aim
                .object_angles(position, self.player_position, true, false)?;
        actor.actor.animation.actor_coordinate_cache = 0;
        Ok(())
    }
}
