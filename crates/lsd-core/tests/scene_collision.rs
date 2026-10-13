use lsd_core::{
    ChunkPlayer, CollisionBounds, CollisionChunk, CollisionFace, CollisionModel, CollisionPolicy,
    CollisionTile, PadState, PlayerControls, PsxMathTables, SceneCollision, SceneCollisionError,
    SceneGrid, SceneSlot, SceneSlots,
};

fn tables() -> PsxMathTables {
    let mut tables = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    tables.sin_cos[0] = 4096 << 16;
    for (index, value) in tables.sqrt_window[320..].iter_mut().enumerate() {
        *value = (((index + 64) as f64).sqrt() * 512.0) as i16;
    }
    tables
}
fn floor(y: i16) -> CollisionChunk {
    let points = [
        [-1024, y, -1024],
        [1024, y, -1024],
        [-1024, y, 1024],
        [1024, y, 1024],
    ];
    let mut cells = vec![Vec::new(); 400];
    cells[0].push(CollisionTile {
        model: 0,
        translation: [1024, 0, 1024],
        heading: 0,
    });
    CollisionChunk {
        policy: CollisionPolicy::for_stage(0),
        models: vec![CollisionModel {
            bounds: CollisionBounds::from_points(&points),
            faces: vec![CollisionFace::Quad(points)],
        }],
        cells,
    }
}
fn scene() -> SceneCollision {
    SceneCollision {
        slots: SceneSlots {
            grid: SceneGrid {
                columns: 1,
                rows: 3,
                mode: 1,
            },
            slots: std::array::from_fn(|i| SceneSlot {
                label: i as i16,
                map_index: i as i16,
                origin: [0, 0, 0],
                ready: i < 3,
            }),
        },
        chunks: std::array::from_fn(|i| match i {
            0 => Some(floor(256)),
            1 => Some(floor(-128)),
            2 => Some(floor(-2064)),
            _ => None,
        }),
    }
}

#[test]
fn floor_readiness_and_layer_order_take_priority_over_nearest_surface() {
    let tables = tables();
    let mut scene = scene();
    let current = scene.floor([1024, 0, 1024], &tables).unwrap().unwrap();
    assert_eq!(
        (current.slot, current.geometry.position),
        (0, [1024, 256, 1024])
    );
    scene.slots.slots[0].ready = false;
    let next = scene.floor([1024, 0, 1024], &tables).unwrap().unwrap();
    assert_eq!((next.slot, next.geometry.position), (1, [1024, -128, 1024]));
    scene.slots.slots[0].ready = true;
    scene.chunks[0] = Some(floor(-2032));
    scene.chunks[1] = Some(CollisionChunk {
        policy: CollisionPolicy::default(),
        models: vec![],
        cells: vec![vec![]; 400],
    });
    let next = scene.floor([1024, -2048, 1024], &tables).unwrap().unwrap();
    assert_eq!(
        (next.slot, next.geometry.position),
        (2, [1024, -2064, 1024])
    );
    scene.slots.slots[2].ready = false;
    let previous = scene.floor([1024, -2048, 1024], &tables).unwrap().unwrap();
    assert_eq!(
        (previous.slot, previous.geometry.position),
        (0, [1024, -2032, 1024])
    );
}

#[test]
fn scene_movement_applies_neighbor_floor_and_rejects_inconsistent_ready_state() {
    let tables = tables();
    let mut scene = scene();
    scene.slots.slots[0].ready = false;
    scene.chunks[0] = None;
    let mut player = ChunkPlayer {
        controls: PlayerControls::default(),
        pad: PadState::default(),
        position: [1024, 0, 1024],
        bounds: CollisionBounds {
            minimum: [-256, -1024, -256],
            maximum: [256, 0, 256],
        },
    };
    player.step_in_scene(0x1000, &scene, &tables).unwrap();
    let step = player.step_in_scene(0x1000, &scene, &tables).unwrap();
    assert_eq!(step.floor_contact.unwrap().slot, 1);
    assert_eq!(player.position, [1024, -128, 1048]);
    scene.slots.slots[0].ready = true;
    assert_eq!(
        scene.floor([1024, 0, 1024], &tables),
        Err(SceneCollisionError::MissingReadyChunk(0))
    );
    assert!(scene.floor([-1, 0, 1024], &tables).unwrap().is_none());
}
