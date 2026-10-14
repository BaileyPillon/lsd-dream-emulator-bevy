//! Owned-color GPU comparison for the original untextured box compositor.
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    asset::RenderAssetUsages,
    camera::{RenderTarget, ScalingMode},
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    image::ImageSampler,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::ExitCondition,
    winit::WinitPlugin,
};
use lsd_bevy::box_material::{BoxMaterial, BoxMaterialPlugin, BoxUniform};
use lsd_core::{BoxFramebuffer, GenerationLayer};
use std::{path::PathBuf, time::Duration};
#[derive(Resource)]
struct Check {
    path: PathBuf,
    target: Handle<Image>,
    requested: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Usage: lsd-box-check <new PNG path>")?,
    );
    let profile = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "blend".to_string());
    if !matches!(profile.as_str(), "blend" | "opaque") {
        return Err("Box profile must be blend or opaque".into());
    }
    let expected = path.with_extension("expected.rgba");
    if path.exists() || expected.exists() {
        return Err("Box comparison output already exists".into());
    }
    let mut frame = BoxFramebuffer::new(320, 240, [0; 3]);
    let pixels: Vec<u16> = (0..240)
        .flat_map(|y| {
            (0..320).map(move |x| {
                ((x * 5 + y * 3) & 31) as u16
                    | (((x * 7 + y * 2) & 31) as u16) << 5
                    | (((x * 11 + y * 13) & 31) as u16) << 10
            })
        })
        .collect();
    frame.set_pixels(&pixels)?;
    let source = frame.rgba();
    let mut packets = Vec::new();
    let mut coverage = vec![false; 3 * 4 * 32 * 32];
    for front in 0..32u32 {
        for mode in 0..4u32 {
            let index = front * 4 + mode;
            let x = (index % 16) * 20;
            let y = (index / 16) * 30;
            let f = [front, 31 - front, (front * 7) & 31];
            let layer = GenerationLayer {
                offset: [x as i32 - 160, y as i32 - 120],
                size: [20, 30],
                color: f.map(|v| (v << 3) as u8),
                flags: if profile == "blend" {
                    (1 << 30) | (mode << 28)
                } else {
                    mode << 28
                },
                depth: 0,
                parent: None,
                relative: false,
            };
            let packet = layer.gpu_box_packet([320, 240]).unwrap();
            for py in y..y + 30 {
                for px in x..x + 20 {
                    let b = pixels[(py * 320 + px) as usize];
                    for channel in 0..3 {
                        coverage[channel * 4096
                            + mode as usize * 1024
                            + ((b >> (channel * 5)) & 31) as usize * 32
                            + f[channel] as usize] = true;
                    }
                }
            }
            frame.draw_box(packet);
            packets.push(packet);
        }
    }
    assert!(coverage.iter().all(|&value| value));
    std::fs::write(&expected, frame.rgba())?;
    println!(
        "owned_boxes=128 packet_profile={profile} blend_modes=4 back_front_pairs_per_channel=4096 compared_pixel_count=76800 expected_rgba={} original_gpu_framebuffer_acceptance=false",
        expected.display()
    );
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>(),
    )
    .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .add_plugins(BoxMaterialPlugin);
    let mut image = Image::new(
        Extent3d {
            width: 320,
            height: 240,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        source,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let scene = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let mut target = Image::new_target_texture(320, 240, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(target);
    let mut boxes = BoxUniform::default();
    boxes.set_packets(&packets);
    let material = app
        .world_mut()
        .resource_mut::<Assets<BoxMaterial>>()
        .add(BoxMaterial { boxes, scene });
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(320.0, 240.0));
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
    app.world_mut().spawn((
        Camera3d::default(),
        RenderTarget::Image(target.clone().into()),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 320.0,
                height: 240.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        Transform::from_xyz(0.0, 0.0, 100.0),
    ));
    app.insert_resource(Check {
        path,
        target,
        requested: false,
    })
    .add_systems(Update, capture)
    .run();
    Ok(())
}
fn capture(mut commands: Commands, time: Res<Time<Real>>, mut check: ResMut<Check>) {
    if !check.requested && time.elapsed_secs_f64() > 5.0 {
        commands
            .spawn(Screenshot::image(check.target.clone()))
            .observe(save_to_disk(check.path.clone()))
            .observe(
                |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                },
            );
        check.requested = true;
    }
}
