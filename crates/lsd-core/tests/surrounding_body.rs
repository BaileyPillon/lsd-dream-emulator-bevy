use lsd_core::{
    CollisionBounds, CollisionChunk, CollisionFace, CollisionModel, CollisionPolicy, CollisionTile,
    MotionAxis, MotionRequest, PsxMathTables, SceneCollision, SceneGrid, SceneSlot, SceneSlots,
};

fn wall(cell: usize, x: i16) -> CollisionChunk {
    let points = [
        [x, -2048, -1024],
        [x, 0, -1024],
        [x, -2048, 1024],
        [x, 0, 1024],
    ];
    let mut cells = vec![Vec::new(); 400];
    cells[cell].push(CollisionTile {
        model: 0,
        translation: [
            (cell % 20) as i32 * 2048 + 1024,
            0,
            (cell / 20) as i32 * 2048 + 1024,
        ],
        heading: 0,
        category: 7,
        flags: 0x80,
    });
    CollisionChunk {
        policy: CollisionPolicy::default(),
        models: vec![CollisionModel {
            bounds: CollisionBounds::from_points(&points),
            faces: vec![CollisionFace::Quad(points)],
        }],
        cells,
    }
}
#[test]
fn adjacent_map_body_contacts_follow_dispatch_order_and_attribute_gates() {
    let mut tables = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    tables.sin_cos[0] = 4096 << 16;
    for (index, value) in tables.sqrt_window[320..].iter_mut().enumerate() {
        *value = (((index + 64) as f64).sqrt() * 512.0) as i16;
    }
    let mut scene = SceneCollision {
        slots: SceneSlots {
            grid: SceneGrid {
                columns: 3,
                rows: 3,
                mode: 0,
            },
            slots: std::array::from_fn(|index| SceneSlot {
                label: index as i16,
                map_index: index as i16,
                origin: if index == 3 {
                    [0; 3]
                } else if index == 4 {
                    [40960, 0, 0]
                } else {
                    [-200000, 0, -200000]
                },
                ready: index == 3 || index == 4,
            }),
        },
        chunks: std::array::from_fn(|index| match index {
            3 => Some(wall(219, 960)),
            4 => Some(wall(200, -1000)),
            _ => None,
        }),
    };
    let position = [40940, 0, 21504];
    let bounds = CollisionBounds {
        minimum: [-256, -1024, -256],
        maximum: [256, 0, 256],
    };
    let motion = MotionRequest {
        axis: MotionAxis::Strafe,
        amount: 24,
        collision_and_links: true,
    };
    let hit = scene
        .body(position, 0, bounds, motion, &tables)
        .unwrap()
        .unwrap();
    assert_eq!(
        (hit.slot, hit.geometry.cell, hit.geometry.tile),
        (4, 200, 0)
    );
    scene.chunks[4].as_mut().unwrap().cells[200][0].flags = 0;
    let hit = scene
        .body(position, 0, bounds, motion, &tables)
        .unwrap()
        .unwrap();
    assert_eq!((hit.slot, hit.geometry.cell), (3, 219));
    scene.chunks[3].as_mut().unwrap().cells[219][0].flags = 0;
    assert!(
        scene
            .body(position, 0, bounds, motion, &tables)
            .unwrap()
            .is_none()
    );
}
