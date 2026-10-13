//! Inspect original stage assets while gameplay and the PS1 renderer are ported.
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    camera::RenderTarget,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    prelude::*,
    render::{
        render_resource::{TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::ExitCondition,
    winit::WinitPlugin,
};
use lsd_assets::{
    formats::{Lbd, Tix, Tmd, Vram},
    is_canonical_reference, load_math_tables, sha256_hex,
};
use lsd_bevy::asset_scene::AssetSceneBuilder;
use lsd_core::{
    ChunkPlayer, CollisionBounds, CollisionChunk, PadState, PlayerControls, PsxMathTables,
};
use std::path::PathBuf;

#[derive(Resource)]
struct StageAssets {
    map: Lbd,
    vram: Vram,
    stage: u8,
    map_number: u16,
}

#[derive(Resource)]
struct Capture {
    path: PathBuf,
    target: Option<Handle<Image>>,
    requested: bool,
}

#[derive(Resource)]
struct WalkingScene {
    player: ChunkPlayer,
    collision: CollisionChunk,
    tables: PsxMathTables,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or(
        "Usage: lsd-stage <extracted-files-root> [stage] [map] [texture A-D] [--inspect]",
    )?);
    let stage: u8 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let map_number: u16 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let texture = args
        .next()
        .unwrap_or_else(|| "A".into())
        .to_ascii_uppercase();
    let mut walk = false;
    let mut walk_steps = None;
    let (inspect, capture) = match args.next() {
        None => (false, None),
        Some(value) if value == "--inspect" => (true, None),
        Some(value) if value == "--walk" => {
            walk = true;
            (false, None)
        }
        Some(value) if value == "--walk-capture" => {
            walk = true;
            let path = PathBuf::from(args.next().ok_or("--walk-capture needs a new PNG path")?);
            walk_steps = Some(
                args.next()
                    .ok_or("--walk-capture needs a tick count")?
                    .parse::<u32>()?,
            );
            (false, Some(path))
        }
        Some(value) if value == "--capture" => (
            false,
            Some(PathBuf::from(
                args.next().ok_or("--capture needs a PNG path")?,
            )),
        ),
        _ => {
            return Err(
                "Use --inspect, --capture, --walk or --walk-capture after the texture variant"
                    .into(),
            );
        }
    };
    if stage > 13 || !matches!(texture.as_str(), "A" | "B" | "C" | "D") || args.next().is_some() {
        return Err("Stage must be 0..13 and texture variant A..D".into());
    }
    if !is_canonical_reference(&std::fs::read(root.join("SLPS_015.56"))?) {
        return Err("Extraction has a different reference executable".into());
    }
    let folder = root.join(format!("CDI/STG{stage:02}"));
    let map_bytes = std::fs::read(folder.join(format!("M{map_number:03}.LBD")))?;
    let texture_bytes = std::fs::read(folder.join(format!("TEX{texture}.TIX")))?;
    let map = Lbd::parse(&map_bytes)?;
    if map.tmd.objects.iter().any(|object| object.scale != 0) {
        return Err("Nonzero TMD design scale requires original runtime scale review".into());
    }
    let tix = Tix::parse(&texture_bytes)?;
    let mut vram = Vram::default();
    for bank in &tix.banks {
        for image in &bank.images {
            vram.upload(image)?;
        }
    }
    let mut pages = std::collections::BTreeSet::new();
    for polygon in map.tmd.objects.iter().flat_map(|object| &object.polygons) {
        if let (Some(page), Some(clut)) = (polygon.texture_page, polygon.clut) {
            pages.insert((page, clut));
        }
    }
    for (page, clut) in pages {
        vram.texture_rgba(page, clut)?;
    }
    let tiles = (0..400)
        .map(|cell| {
            map.cell_chain(cell)
                .map(|chain| chain.into_iter().filter(|tile| tile.present()).count())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum::<usize>();
    let polygons: usize = map
        .tmd
        .objects
        .iter()
        .map(|object| object.polygons.len())
        .sum();
    println!(
        "stage={stage} map={map_number} texture={texture} objects={} polygons={polygons} tile_instances={tiles}\nmap_sha256={}\ntexture_sha256={}\nprofile=asset_inspection gameplay_parity_accepted=false",
        map.tmd.objects.len(),
        sha256_hex(&map_bytes),
        sha256_hex(&texture_bytes)
    );
    if inspect {
        return Ok(());
    }
    let walking = if walk {
        let tables = load_math_tables(&std::fs::read(root.join("SLPS_015.56"))?)?;
        let collision = map.collision_chunk();
        let body = Tmd::parse(&std::fs::read(root.join("CDI/ETC/DREAME5.TMD"))?)?;
        let bounds = CollisionBounds::from_points(
            &body.objects[0]
                .vertices
                .iter()
                .map(|point| point.xyz)
                .collect::<Vec<_>>(),
        )
        .ok_or("Missing original player bounds")?;
        let mut spawn = None;
        for cell in 0..400 {
            for tile in map.cell_chain(cell)? {
                if tile.present()
                    && let Some(contact) = collision.floor(tile.native_position(cell), &tables)?
                {
                    spawn = Some(contact.position);
                    break;
                }
            }
            if spawn.is_some() {
                break;
            }
        }
        let mut player = ChunkPlayer {
            controls: PlayerControls {
                eye: [0, -512, 0],
                target: [0, -512, 1024],
                ..default()
            },
            pad: PadState::default(),
            position: spawn.ok_or("No floor found for the movement prototype")?,
            bounds,
        };
        if let Some(steps) = walk_steps {
            for _ in 0..steps {
                player.step(0x1000, &collision, &tables)?;
            }
        }
        println!(
            "profile=single_chunk_movement_prototype position={:?} heading={} original_link_npc_callbacks_pending=true",
            player.position, player.controls.angles[1]
        );
        Some(WalkingScene {
            player,
            collision,
            tables,
        })
    } else {
        None
    };
    let mut app = App::new();
    if let Some(walking) = walking {
        app.insert_resource(walking)
            .insert_resource(Time::<Fixed>::from_hz(15.0))
            .add_systems(FixedUpdate, walking_controls)
            .add_systems(Update, walking_camera);
    }
    app.insert_resource(StageAssets {
        map,
        vram,
        stage,
        map_number,
    });
    let plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(WindowPlugin {
            primary_window: if capture.is_some() {
                None
            } else {
                Some(Window {
                    title: format!(
                        "LSD Bevy - STG{stage:02} M{map_number:03} | WASD, Q/E, Shift, R/F"
                    ),
                    resolution: (960, 720).into(),
                    ..default()
                })
            },
            exit_condition: ExitCondition::DontExit,
            ..default()
        });
    if let Some(path) = capture {
        if path.exists() {
            return Err("Capture output already exists".into());
        }
        app.insert_resource(Capture {
            path,
            target: None,
            requested: false,
        })
        .add_plugins(plugins.disable::<WinitPlugin>())
        .add_plugins(ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .add_systems(Update, capture_frame);
    } else {
        app.add_plugins(plugins);
    }
    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            camera_controls.run_if(not(resource_exists::<WalkingScene>)),
        )
        .run();
    Ok(())
}

fn walking_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut scene: ResMut<WalkingScene>,
    capture: Option<Res<Capture>>,
) {
    if capture.is_some() {
        return;
    }
    let mut buttons = 0;
    for (key, mask) in [
        (KeyCode::KeyW, 0x1000),
        (KeyCode::KeyS, 0x4000),
        (KeyCode::KeyA, 0x8000),
        (KeyCode::KeyD, 0x2000),
        (KeyCode::KeyQ, 1),
        (KeyCode::KeyE, 2),
        (KeyCode::ShiftLeft, 0x40),
        (KeyCode::KeyR, 0x10),
        (KeyCode::KeyF, 0x80),
        (KeyCode::Space, 0x20),
    ] {
        if keys.pressed(key) {
            buttons |= mask;
        }
    }
    let WalkingScene {
        player,
        collision,
        tables,
    } = &mut *scene;
    player
        .step(buttons, collision, tables)
        .expect("decoded chunk movement query");
}

fn walking_camera(scene: Res<WalkingScene>, mut cameras: Query<&mut Transform, With<Camera3d>>) {
    let player = &scene.player;
    let rotation = scene.tables.rotation(player.controls.angles);
    let eye = rotation.apply_short(player.controls.eye.map(|value| value as i16));
    let target = rotation.apply_short(player.controls.target.map(|value| value as i16));
    let convert = |offset: [i32; 3]| {
        let position: [i32; 3] =
            std::array::from_fn(|axis| player.position[axis].wrapping_add(offset[axis]));
        Vec3::new(
            position[0] as f32 / 2048.0,
            -position[1] as f32 / 2048.0,
            -position[2] as f32 / 2048.0,
        )
    };
    for mut camera in &mut cameras {
        *camera = Transform::from_translation(convert(eye)).looking_at(convert(target), Vec3::Y);
    }
}

fn setup(
    mut commands: Commands,
    source: Res<StageAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    capture: Option<ResMut<Capture>>,
) {
    let mut builder =
        AssetSceneBuilder::new(&source.vram, &mut meshes, &mut materials, &mut images);
    let models: Vec<_> = source
        .map
        .tmd
        .objects
        .iter()
        .map(|object| {
            (0..4)
                .map(|quarter| builder.build_object(object, quarter))
                .collect::<Vec<_>>()
        })
        .collect();
    for cell in 0..400 {
        for tile in source.map.cell_chain(cell).expect("validated cell chain") {
            if !tile.present() {
                continue;
            }
            let [x, y, z] = tile.native_position(cell);
            for (mesh, material) in
                &models[usize::from(tile.model)][usize::from(tile.quarter_turn & 3)]
            {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Name::new(format!(
                        "STG{:02}/M{:03}/cell{cell}",
                        source.stage, source.map_number
                    )),
                    Transform::from_xyz(x as f32 / 2048.0, -y as f32 / 2048.0, -z as f32 / 2048.0),
                ));
            }
        }
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.4, -0.8, 0.0)),
    ));
    let target = if let Some(mut capture) = capture {
        let mut image = Image::new_target_texture(320, 240, TextureFormat::Rgba8UnormSrgb, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let handle = images.add(image);
        capture.target = Some(handle.clone());
        RenderTarget::Image(handle.into())
    } else {
        RenderTarget::default()
    };
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        target,
        Transform::from_xyz(16.0, 13.0, 4.0).looking_at(Vec3::new(7.0, 0.8, -6.0), Vec3::Y),
    ));
}

fn capture_frame(mut commands: Commands, time: Res<Time<Real>>, mut capture: ResMut<Capture>) {
    if !capture.requested
        && time.elapsed_secs_f64() > 5.0
        && let Some(target) = &capture.target
    {
        commands
            .spawn(Screenshot::image(target.clone()))
            .observe(save_to_disk(capture.path.clone()))
            .observe(
                |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                },
            );
        capture.requested = true;
    }
}

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let dt = time.delta_secs().min(0.1);
    let pressed = |key| f32::from(u8::from(keys.pressed(key)));
    let turn = pressed(KeyCode::ArrowLeft) - pressed(KeyCode::ArrowRight);
    let ahead = pressed(KeyCode::KeyW) - pressed(KeyCode::KeyS);
    let side = pressed(KeyCode::KeyD) - pressed(KeyCode::KeyA);
    let up = pressed(KeyCode::KeyE) - pressed(KeyCode::KeyQ);
    for mut camera in &mut cameras {
        camera.rotate_y(turn * dt * 1.6);
        let direction =
            camera.forward().as_vec3() * ahead + camera.right().as_vec3() * side + Vec3::Y * up;
        camera.translation += direction.normalize_or_zero() * dt * 4.0;
    }
}
