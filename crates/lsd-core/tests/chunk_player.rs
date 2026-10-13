use lsd_core::{
    ChunkPlayer, CollisionBounds, CollisionChunk, CollisionFace, CollisionModel, CollisionTile,
    PadState, PlayerControls, PsxMathTables,
};

#[test]
fn input_motion_and_body_contacts_preserve_callback_handoff_order() {
    let mut tables = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    tables.sin_cos[0] = 4096 << 16;
    for (index, value) in tables.sqrt_window[320..].iter_mut().enumerate() {
        *value = (((index + 64) as f64).sqrt() * 512.0) as i16;
    }
    let floor = CollisionFace::Quad([
        [-1024, 0, -1024],
        [1024, 0, -1024],
        [-1024, 0, 1024],
        [1024, 0, 1024],
    ]);
    let wall = CollisionFace::Quad([
        [-1024, -2048, 70],
        [1024, -2048, 70],
        [-1024, 0, 70],
        [1024, 0, 70],
    ]);
    let mut cells = vec![Vec::new(); 400];
    cells[0].push(CollisionTile {
        model: 0,
        translation: [1024, 0, 1024],
        heading: 0,
    });
    let chunk = CollisionChunk {
        models: vec![CollisionModel {
            bounds: Some(CollisionBounds {
                minimum: [-1024, -2048, -1024],
                maximum: [1024, 0, 1024],
            }),
            faces: vec![floor, wall],
        }],
        cells,
    };
    let mut player = ChunkPlayer {
        controls: PlayerControls::default(),
        pad: PadState::default(),
        position: [1024, 0, 1024],
        bounds: CollisionBounds {
            minimum: [-256, -1024, -256],
            maximum: [256, 0, 256],
        },
    };
    let first = player.step(0x1000, &chunk, &tables).unwrap();
    assert_eq!(player.position, [1024, 0, 1024]);
    assert!(first.body_contact.is_none());
    let second = player.step(0x1000, &chunk, &tables).unwrap();
    assert_eq!(player.position, [1024, 0, 1048]);
    assert!(second.body_contact.is_some());
    assert!(second.floor_contact.is_none());
    // Body contact is handed to the scene/link layer; this primitive does not invent a push-out rule.
}
