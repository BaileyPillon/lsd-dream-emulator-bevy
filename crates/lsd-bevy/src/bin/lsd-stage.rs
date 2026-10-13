//! Inspect original stage assets while gameplay and the PS1 renderer are ported.
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    asset::RenderAssetUsages,
    camera::RenderTarget,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    image::ImageSampler,
    mesh::Indices,
    prelude::*,
    render::{
        render_resource::{
            Extent3d, Face, PrimitiveTopology, TextureDimension, TextureFormat, TextureUsages,
        },
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::ExitCondition,
    winit::WinitPlugin,
};
use lsd_assets::{
    formats::{Lbd, Polygon, Tix, TmdObject, Vram},
    is_canonical_reference, sha256_hex,
};
use std::{collections::BTreeMap, path::PathBuf};

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

#[derive(Default)]
struct MeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

type MaterialKey = (Option<u16>, Option<u16>, u8, u8);

fn rotate(vector: [i16; 3], quarter: u8) -> [f32; 3] {
    let [x, y, z] = vector.map(i32::from);
    let (x, z) = match quarter & 3 {
        0 => (x, z),
        1 => (z, -x),
        2 => (-x, -z),
        _ => (-z, x),
    };
    // PS1 +Y points down and +Z points forward; Bevy uses +Y up and camera -Z.
    [x as f32 / 2048.0, -y as f32 / 2048.0, -z as f32 / 2048.0]
}

fn append_polygon(data: &mut MeshData, object: &TmdObject, polygon: &Polygon, quarter: u8) {
    let base = data.positions.len() as u32;
    let positions: Vec<_> = polygon
        .vertices
        .iter()
        .map(|&index| rotate(object.vertices[usize::from(index)].xyz, quarter))
        .collect();
    let face_normal = (Vec3::from_array(positions[2]) - Vec3::from_array(positions[0]))
        .cross(Vec3::from_array(positions[1]) - Vec3::from_array(positions[0]))
        .normalize_or_zero()
        .to_array();
    for (i, position) in positions.iter().enumerate() {
        data.positions.push(*position);
        let normal = if polygon.normals.is_empty() {
            face_normal
        } else {
            let index = polygon.normals[if polygon.normals.len() == 1 { 0 } else { i }];
            Vec3::from_array(rotate(object.normals[usize::from(index)].xyz, quarter))
                .normalize_or_zero()
                .to_array()
        };
        data.normals.push(normal);
        let color = if polygon.colors.is_empty()
            || (polygon.texture_page.is_some() && polygon.texture_raw())
        {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            let rgba = polygon.colors[if polygon.colors.len() == 1 { 0 } else { i }];
            Color::srgb_u8(rgba[0], rgba[1], rgba[2])
                .to_linear()
                .to_f32_array()
        };
        data.colors.push(color);
        data.uv.push(if polygon.uv.is_empty() {
            [0.0, 0.0]
        } else {
            [
                (f32::from(polygon.uv[i][0]) + 0.5) / 256.0,
                (f32::from(polygon.uv[i][1]) + 0.5) / 256.0,
            ]
        });
    }
    data.indices.extend([base, base + 2, base + 1]);
    if positions.len() == 4 {
        data.indices.extend([base + 1, base + 2, base + 3]);
    }
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
    let (inspect, capture) = match args.next() {
        None => (false, None),
        Some(value) if value == "--inspect" => (true, None),
        Some(value) if value == "--capture" => (
            false,
            Some(PathBuf::from(
                args.next().ok_or("--capture needs a PNG path")?,
            )),
        ),
        _ => {
            return Err("Only --inspect or --capture is accepted after the texture variant".into());
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
    let mut app = App::new();
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
                        "LSD Bevy - STG{stage:02} M{map_number:03} asset viewer | WASD, arrows, Q/E"
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
        .add_systems(Update, camera_controls)
        .run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    source: Res<StageAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    capture: Option<ResMut<Capture>>,
) {
    let mut textures = BTreeMap::new();
    let mut models = Vec::new();
    for object in &source.map.tmd.objects {
        let mut rotations = Vec::new();
        for quarter in 0..4 {
            let mut groups: BTreeMap<MaterialKey, MeshData> = BTreeMap::new();
            for polygon in &object.polygons {
                let key = (
                    polygon.texture_page,
                    polygon.clut,
                    polygon.mode,
                    polygon.flag,
                );
                append_polygon(groups.entry(key).or_default(), object, polygon, quarter);
            }
            let mut parts = Vec::new();
            for (key, data) in groups {
                let texture = match (key.0, key.1) {
                    (Some(page), Some(clut)) => Some(
                        textures
                            .entry((page, clut))
                            .or_insert_with(|| {
                                let rgba = source
                                    .vram
                                    .texture_rgba(page, clut)
                                    .expect("reviewed page must fit VRAM");
                                let mut image = Image::new(
                                    Extent3d {
                                        width: 256,
                                        height: 256,
                                        depth_or_array_layers: 1,
                                    },
                                    TextureDimension::D2,
                                    rgba,
                                    TextureFormat::Rgba8UnormSrgb,
                                    RenderAssetUsages::default(),
                                );
                                image.sampler = ImageSampler::nearest();
                                images.add(image)
                            })
                            .clone(),
                    ),
                    _ => None,
                };
                let mut mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                );
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals);
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colors);
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uv);
                mesh.insert_indices(Indices::U32(data.indices));
                let material = StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: texture,
                    unlit: key.3 & 1 != 0,
                    cull_mode: if key.3 & 2 != 0 {
                        None
                    } else {
                        Some(Face::Back)
                    },
                    alpha_mode: if key.2 & 2 != 0 {
                        AlphaMode::Blend
                    } else {
                        AlphaMode::Mask(0.5)
                    },
                    ..default()
                };
                parts.push((meshes.add(mesh), materials.add(material)));
            }
            rotations.push(parts);
        }
        models.push(rotations);
    }
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
