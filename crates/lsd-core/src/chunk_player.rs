use crate::{
    CollisionBounds, CollisionChunk, CollisionFault, FloorEffectStep, MotionAxis, MovementStep,
    PAD_MASKS, PadState, PlayerControls, PsxMathTables, SceneCollision, SceneCollisionError,
    SceneContact, WorldContact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPlayer {
    pub controls: PlayerControls,
    pub pad: PadState,
    pub position: [i32; 3],
    pub bounds: CollisionBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkPlayerStep<Contact = WorldContact> {
    pub floor_effect: Option<FloorEffectStep>,
    pub body_contact: Option<Contact>,
    pub floor_contact: Option<Contact>,
}

impl ChunkPlayer {
    fn input_and_motion(&mut self, buttons: u32, tables: &PsxMathTables) -> Option<MovementStep> {
        self.pad.poll(buttons);
        for code in self.pad.notifications(&PAD_MASKS) {
            self.controls.notify_pad(code);
        }
        self.controls.update_view();
        let movement = self.controls.update_movement();
        if let Some(request) = movement.and_then(|step| step.motion) {
            let amount = request.amount as i16;
            let local = if request.axis == MotionAxis::Forward {
                [0, 0, amount]
            } else {
                [amount, 0, 0]
            };
            let delta = tables.rotation(self.controls.angles).apply_short(local);
            for (position, delta) in self.position.iter_mut().zip(delta) {
                *position = position.wrapping_add(delta);
            }
        }
        movement
    }
    /// One supplied-input step in a loaded chunk. Scene/link/NPC callbacks are separate.
    pub fn step(
        &mut self,
        buttons: u32,
        chunk: &CollisionChunk,
        tables: &PsxMathTables,
    ) -> Result<ChunkPlayerStep, CollisionFault> {
        let movement = self.input_and_motion(buttons, tables);
        let mut result = ChunkPlayerStep {
            floor_effect: movement.map(|step| step.floor_effect),
            body_contact: None,
            floor_contact: None,
        };
        if let Some(request) = movement.and_then(|step| step.motion) {
            if request.collision_and_links {
                result.body_contact = chunk.body(
                    self.position,
                    self.controls.angles[1],
                    self.bounds,
                    request,
                    tables,
                )?;
            }
            if result.body_contact.is_none() {
                result.floor_contact = chunk.floor(self.position, tables)?;
                if let Some(contact) = result.floor_contact {
                    self.position = contact.position;
                }
            }
        }
        Ok(result)
    }

    /// Supplied scene slot graph; original stage lifecycle, link and NPC callbacks remain separate.
    pub fn step_in_scene(
        &mut self,
        buttons: u32,
        scene: &SceneCollision,
        tables: &PsxMathTables,
    ) -> Result<ChunkPlayerStep<SceneContact>, SceneCollisionError> {
        let movement = self.input_and_motion(buttons, tables);
        let mut result = ChunkPlayerStep {
            floor_effect: movement.map(|step| step.floor_effect),
            body_contact: None,
            floor_contact: None,
        };
        if let Some(request) = movement.and_then(|step| step.motion) {
            if request.collision_and_links {
                result.body_contact = scene.body(
                    self.position,
                    self.controls.angles[1],
                    self.bounds,
                    request,
                    tables,
                )?;
            }
            if result.body_contact.is_none() {
                result.floor_contact = scene.floor(self.position, tables)?;
                if let Some(contact) = result.floor_contact {
                    self.position = contact.geometry.position;
                }
            }
        }
        Ok(result)
    }
}
