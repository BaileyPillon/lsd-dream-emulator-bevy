//! Procedural development geometry; no original level or game assets are claimed.
use bevy::prelude::*;
use lsd_core::CellBounds;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LSD Bevy - development grid | WASD move, arrows turn".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, move_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tile = meshes.add(Cuboid::new(0.97, 0.04, 0.97));
    let inside = materials.add(Color::srgb(0.22, 0.55, 0.42));
    let outside = materials.add(Color::srgb(0.19, 0.22, 0.32));
    for x in 0..20 {
        for z in 0..20 {
            let material = if CellBounds::STAGE_ZERO.contains([x, z]) {
                &inside
            } else {
                &outside
            };
            commands.spawn((
                Mesh3d(tile.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(f32::from(x) + 0.5, 0.0, f32::from(z) + 0.5),
            ));
        }
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.4, -0.8, 0.0)),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 2.0, 10.0).looking_at(Vec3::new(10.0, 2.0, 8.0), Vec3::Y),
    ));
}

fn move_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let dt = time.delta_secs().min(0.1);
    let pressed = |key| f32::from(u8::from(keys.pressed(key)));
    let turn = pressed(KeyCode::ArrowLeft) - pressed(KeyCode::ArrowRight);
    let ahead = pressed(KeyCode::KeyW) - pressed(KeyCode::KeyS);
    let sideways = pressed(KeyCode::KeyD) - pressed(KeyCode::KeyA);
    for mut camera in &mut cameras {
        camera.rotate_y(turn * dt * 1.6);
        let direction = camera.forward().as_vec3() * ahead + camera.right().as_vec3() * sideways;
        camera.translation += direction.normalize_or_zero() * dt * 4.0;
        camera.translation.x = camera.translation.x.clamp(0.2, 19.8);
        camera.translation.z = camera.translation.z.clamp(0.2, 19.8);
    }
}
