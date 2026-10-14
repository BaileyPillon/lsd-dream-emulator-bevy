#[allow(dead_code)]
#[path = "support/links.rs"]
mod support;
use lsd_core::{
    BodyQueryState, BodyVisit, ChunkPlayer, CollisionBounds, CollisionChunk, CollisionFace,
    CollisionModel, CollisionPolicy, CollisionTile, LinkedPlayer, MapLinkCommand, MotionAxis,
    MotionRequest, PadState, PlayerControls, PsxMathTables, SceneCollision, SceneGrid,
    SceneLinkTables, SceneSlot, SceneSlots,
};
use serde_json::Value;
fn math() -> PsxMathTables {
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
fn scene(face: [[i16; 3]; 4], translation: [i32; 3], copies: usize) -> SceneCollision {
    let mut cells = vec![Vec::new(); 400];
    for _ in 0..copies {
        cells[0].push(CollisionTile {
            model: 0,
            translation,
            heading: 0,
            category: 0,
            flags: 0x80,
        });
    }
    SceneCollision {
        slots: SceneSlots {
            grid: SceneGrid {
                columns: 1,
                rows: 7,
                mode: 1,
            },
            slots: std::array::from_fn(|i| SceneSlot {
                label: i as i16,
                map_index: i as i16,
                origin: [0; 3],
                ready: i == 0,
            }),
        },
        chunks: std::array::from_fn(|i| {
            if i == 0 {
                Some(CollisionChunk {
                    policy: CollisionPolicy::default(),
                    models: vec![CollisionModel {
                        bounds: CollisionBounds::from_points(&face),
                        faces: vec![CollisionFace::Quad(face)],
                    }],
                    cells: cells.clone(),
                })
            } else {
                None
            }
        }),
    }
}
#[test]
fn body_callbacks_change_later_geometry_like_the_complete_native_dispatch_path() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/live-body-callback.json")).unwrap();
    let face = std::array::from_fn(|i| {
        std::array::from_fn(|axis| fixture["face"][i][axis].as_i64().unwrap() as i16)
    });
    let scene = scene(face, [0; 3], 2);
    let math = math();
    for case in fixture["cases"].as_array().unwrap() {
        let mut query = BodyQueryState {
            position: [0; 3],
            heading: 0,
            bounds: CollisionBounds {
                minimum: [-256, -1024, -256],
                maximum: [256, 0, 256],
            },
            motion: MotionRequest {
                axis: MotionAxis::Forward,
                amount: 24,
                collision_and_links: true,
            },
        };
        let mut hits = Vec::new();
        scene
            .body_with(&mut query, &math, |query, event| {
                if let BodyVisit::Hit { candidate, .. } = event {
                    hits.push(candidate.tile);
                    if case["move_after_first"].as_bool().unwrap() && hits.len() == 1 {
                        query.position = [0, 0, 4096];
                    }
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(
            hits,
            case["hits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_u64().unwrap() as usize)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            query.position,
            std::array::from_fn(|i| case["final_actor_position"][i].as_i64().unwrap() as i32)
        );
    }
}
#[test]
fn loaded_scene_motion_runs_wall_policy_before_remaining_contacts_and_restores_pose() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/link-runtime.json")).unwrap();
    let mut catalog = support::catalog(&fixture["catalog"]);
    for set in [
        &mut catalog.staircase,
        &mut catalog.teleport,
        &mut catalog.tunnel,
    ] {
        set.triggers = std::array::from_fn(|_| Vec::new());
    }
    catalog.wall.triggers = std::array::from_fn(|_| Vec::new());
    catalog.wall.triggers[0].push(lsd_core::LinkTrigger {
        chunk: 0,
        tile: 0,
        target_stage: 3,
        target_spawn: 0,
    });
    let scene = scene(
        [
            [-1024, -2048, 70],
            [1024, -2048, 70],
            [-1024, 0, 70],
            [1024, 0, 70],
        ],
        [1024, 0, 1024],
        1,
    );
    let mut player = ChunkPlayer {
        controls: PlayerControls::default(),
        pad: PadState::default(),
        position: [1024, 0, 1024],
        bounds: CollisionBounds {
            minimum: [-256, -1024, -256],
            maximum: [256, 0, 256],
        },
    };
    let auto = &fixture["auto_tables"];
    let tables = SceneLinkTables {
        catalog,
        motion: lsd_core::LinkMotionRules {
            distances: [0, 24, 64, 128, 384],
            signs: [0, 1, -1, -1, 1],
        },
        auto_move: lsd_core::AutoMoveTables {
            targets: std::array::from_fn(|i| {
                std::array::from_fn(|axis| auto["targets"][i][axis].as_i64().unwrap() as i16)
            }),
            clockwise: std::array::from_fn(|i| auto["clockwise"][i].as_u64().unwrap() as u32),
            counterclockwise: std::array::from_fn(|i| {
                auto["counterclockwise"][i].as_u64().unwrap() as u32
            }),
        },
    };
    let mut linked = LinkedPlayer::new(0, &player, &scene, tables, 1).unwrap();
    let math = math();
    linked
        .step(&mut player, &scene, &math, [1; 7], 0x1000)
        .unwrap();
    assert!(linked.body_contact.is_none());
    linked
        .step(&mut player, &scene, &math, [1; 7], 0x1000)
        .unwrap();
    assert!(linked.body_contact.is_some());
    assert!(linked.floor_contact.is_none());
    assert_eq!(player.position, [1024, 0, 1024]);
    assert!(player.controls.input_locked);
    assert_eq!(
        (
            linked.links.execution.state,
            linked.links.execution.stage,
            linked.map.state
        ),
        (13, 3, 6)
    );
    assert_eq!(
        linked.commands,
        vec![MapLinkCommand::Fade {
            color: 0,
            target: 0,
            ticks: 30,
            attach: true
        }]
    );
    assert_eq!(linked.sounds, vec![[0x90, 0x6e, 0x6e]]);
    assert_eq!(linked.context.navigation[3], 1);
}
