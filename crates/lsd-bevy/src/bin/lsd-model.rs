//! Original animation playback in an asset inspector; original NPC/game systems remain open.
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
    formats::{Lbd, Mom, Tix, Vram},
    is_canonical_reference, sha256_hex,
};
use lsd_bevy::asset_scene::{AssetSceneBuilder, MeshBinding};
use lsd_core::{AnimationParent, AnimationPart, AnimationPlayer};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Resource)]
struct ModelScene {
    model: Mom,
    player: AnimationPlayer,
    vram: Vram,
    root: Option<Entity>,
    bones: Vec<Entity>,
    meshes: Vec<Vec<MeshBinding>>,
    bound: Vec<Option<usize>>,
    parent_bound: Vec<bool>,
    focus: Vec3,
    radius: f32,
    paused: bool,
}

#[derive(Component)]
struct Bone(usize);

#[derive(Resource)]
struct Capture {
    path: PathBuf,
    target: Option<Handle<Image>>,
    requested: bool,
}

// Float presentation adapter. Native rotation matrices, quantization and GTE remain open.
fn pose(part: &AnimationPart) -> Transform {
    let [x, y, z] = part
        .coordinate_translation
        .map(|value| value as f32 / 2048.0);
    let [rx, ry, rz] = part
        .angles
        .map(|angle| f32::from(angle) * std::f32::consts::TAU / 4096.0);
    Transform {
        translation: Vec3::new(x, -y, -z),
        rotation: Quat::from_euler(EulerRot::XYZ, rx, -ry, -rz),
        scale: Vec3::from_array(part.scale.map(|value| value as f32 / 4096.0)),
    }
}

fn framing(model: &Mom, player: &AnimationPlayer) -> Result<(Vec3, f32), &'static str> {
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for (index, part) in player.parts.iter().enumerate() {
        let Some(primitive) = part.primitive else {
            continue;
        };
        let mut path = Vec::new();
        let mut current = Some(index);
        while let Some(index) = current {
            if path.contains(&index) {
                return Err("Cyclic model parent graph");
            }
            path.push(index);
            current = match player.parts[index].parent {
                Some(AnimationParent::Part(owner)) => Some(owner),
                _ => None,
            };
        }
        let mut world = Mat4::IDENTITY;
        for &index in path.iter().rev() {
            world *= pose(&player.parts[index]).to_matrix();
        }
        for vertex in &model.tmd.objects[primitive].vertices {
            let [x, y, z] = vertex.xyz.map(|value| f32::from(value) / 2048.0);
            let position = world.transform_point3(Vec3::new(x, -y, -z));
            low = low.min(position);
            high = high.max(position);
        }
    }
    if !low.is_finite() {
        return Err("Animation has no bound geometry to display");
    }
    Ok(((low + high) * 0.5, ((high - low).length() * 0.5).max(0.1)))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or("Usage: lsd-model <files-root> [stage=0] [map=0] [model=0] [clip=0] [texture=A] [--inspect | --capture new.png ticks]")?);
    let stage: u8 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let map: u16 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let model_index: usize = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let clip: usize = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let texture = args
        .next()
        .unwrap_or_else(|| "A".into())
        .to_ascii_uppercase();
    let mut inspect = false;
    let mut capture = None;
    let mut ticks = 0u32;
    if let Some(option) = args.next() {
        match option.as_str() {
            "--inspect" => inspect = true,
            "--capture" => {
                let path = PathBuf::from(args.next().ok_or("--capture requires a new PNG path")?);
                if path.exists() {
                    return Err("Capture output already exists".into());
                }
                ticks = args
                    .next()
                    .ok_or("--capture requires an explicit tick count")?
                    .parse()?;
                capture = Some(path);
            }
            _ => return Err("Unknown model viewer option".into()),
        }
    }
    if stage > 13 || !matches!(texture.as_str(), "A" | "B" | "C" | "D") || args.next().is_some() {
        return Err("Stage must be 0..13 and texture variant A..D".into());
    }
    if !is_canonical_reference(&std::fs::read(root.join("SLPS_015.56"))?) {
        return Err("Extraction has a different reference executable".into());
    }
    let folder = root.join(format!("CDI/STG{stage:02}"));
    let map_bytes = std::fs::read(folder.join(format!("M{map:03}.LBD")))?;
    let parsed = Lbd::parse(&map_bytes)?;
    let model = parsed
        .model_library
        .ok_or("Map has no streamed model library")?
        .models
        .into_iter()
        .nth(model_index)
        .ok_or("Model index outside streamed library")?;
    if model.tmd.objects.iter().any(|object| object.scale != 0) {
        return Err("Nonzero TMD design scale requires original runtime review".into());
    }
    let mut player = AnimationPlayer::new(&model.initial_part_types, model.tmd.objects.len());
    player.select_clip(&model.animations, clip)?;
    let (focus, radius) = framing(&model, &player)?;
    player.enabled = true;
    for _ in 0..ticks {
        player.tick(&model.animations, |_| {
            panic!("asset inspector has no NPC action")
        })?;
    }
    let tix_bytes = std::fs::read(folder.join(format!("TEX{texture}.TIX")))?;
    let tix = Tix::parse(&tix_bytes)?;
    let mut vram = Vram::default();
    for bank in &tix.banks {
        for image in &bank.images {
            vram.upload(image)?;
        }
    }
    let pages: BTreeSet<_> = model
        .tmd
        .objects
        .iter()
        .flat_map(|object| &object.polygons)
        .filter_map(|polygon| polygon.texture_page.zip(polygon.clut))
        .collect();
    for (page, clut) in pages {
        vram.texture_rgba(page, clut)?;
    }
    println!(
        "stage={stage} map={map} model={model_index} clip={clip} parts={} frames={} ticks={} frame={}\nmap_sha256={}\ntexture_sha256={}\nprofile=animation_asset_inspection gameplay_parity_accepted=false",
        player.parts.len(),
        model.animations[clip].frames.len(),
        player.ticks,
        player.frame_index,
        sha256_hex(&map_bytes),
        sha256_hex(&tix_bytes)
    );
    for (index, part) in player.parts.iter().enumerate() {
        println!(
            "part={index} type={} position={:?} angles={:?} scale={:?} primitive={:?} parent={:?}",
            part.part_type,
            part.coordinate_translation,
            part.angles,
            part.scale,
            part.primitive,
            part.parent
        );
    }
    if inspect {
        return Ok(());
    }
    let mut app = App::new();
    let offline = capture.is_some();
    app.insert_resource(ModelScene {
        bound: vec![None; player.parts.len()],
        parent_bound: vec![false; player.parts.len()],
        focus,
        radius,
        model,
        player,
        vram,
        root: None,
        bones: Vec::new(),
        meshes: Vec::new(),
        paused: offline,
    });
    let plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(WindowPlugin {
            primary_window: if offline {
                None
            } else {
                Some(Window {
                    title: "LSD Bevy - original model animation | Space pause, N step, WASD camera"
                        .into(),
                    resolution: (960, 720).into(),
                    ..default()
                })
            },
            exit_condition: ExitCondition::DontExit,
            ..default()
        });
    if let Some(path) = capture {
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
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, animate)
        .add_systems(Update, (controls, synchronize).chain())
        .run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    mut scene: ResMut<ModelScene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    capture: Option<ResMut<Capture>>,
) {
    scene.meshes = {
        let mut builder =
            AssetSceneBuilder::new(&scene.vram, &mut meshes, &mut materials, &mut images);
        scene
            .model
            .tmd
            .objects
            .iter()
            .map(|object| builder.build_object(object, 0))
            .collect()
    };
    scene.root = Some(
        commands
            .spawn((
                Name::new("Original actor"),
                Transform::default(),
                Visibility::Inherited,
            ))
            .id(),
    );
    scene.bones = scene
        .player
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            commands
                .spawn((
                    Bone(index),
                    Name::new(format!("Part {index} type {}", part.part_type)),
                    pose(part),
                    Visibility::Inherited,
                ))
                .id()
        })
        .collect();
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
        Transform::from_translation(scene.focus + Vec3::new(1.5, 1.0, 2.5) * scene.radius * 1.4)
            .looking_at(scene.focus, Vec3::Y),
    ));
}

fn animate(mut scene: ResMut<ModelScene>) {
    if !scene.paused {
        let ModelScene { model, player, .. } = &mut *scene;
        player
            .tick(&model.animations, |_| {
                panic!("asset inspector has no NPC action")
            })
            .expect("validated animation");
    }
}

fn synchronize(
    mut commands: Commands,
    mut scene: ResMut<ModelScene>,
    mut bones: Query<(&Bone, &mut Transform)>,
) {
    for (Bone(index), mut transform) in &mut bones {
        *transform = pose(&scene.player.parts[*index]);
    }
    for index in 0..scene.bones.len() {
        if let Some(parent) = scene.player.parts[index].parent
            && !scene.parent_bound[index]
        {
            commands
                .entity(scene.bones[index])
                .insert(ChildOf(match parent {
                    AnimationParent::Actor => scene.root.unwrap(),
                    AnimationParent::Part(owner) => scene.bones[owner],
                }));
            scene.parent_bound[index] = true;
        }
        if let Some(primitive) = scene.player.parts[index].primitive
            && scene.bound[index].is_none()
        {
            for (mesh, material) in &scene.meshes[primitive] {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::default(),
                    ChildOf(scene.bones[index]),
                ));
            }
            scene.bound[index] = Some(primitive);
        }
    }
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut scene: ResMut<ModelScene>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        scene.paused = !scene.paused;
    }
    if keys.just_pressed(KeyCode::KeyN) && scene.paused {
        let ModelScene { model, player, .. } = &mut *scene;
        player
            .tick(&model.animations, |_| {
                panic!("asset inspector has no NPC action")
            })
            .expect("validated animation");
    }
    for mut camera in &mut cameras {
        let speed = time.delta_secs() * 4.0;
        let forward = *camera.forward();
        let right = *camera.right();
        if keys.pressed(KeyCode::KeyW) {
            camera.translation += forward * speed;
        }
        if keys.pressed(KeyCode::KeyS) {
            camera.translation -= forward * speed;
        }
        if keys.pressed(KeyCode::KeyA) {
            camera.translation -= right * speed;
        }
        if keys.pressed(KeyCode::KeyD) {
            camera.translation += right * speed;
        }
    }
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
