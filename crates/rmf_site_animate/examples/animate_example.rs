//! scene to demonstrate different animate component effects.

use bevy::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use rmf_site_animate::{Bobbing, VisualCueAnimationsPlugin, Spinning};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })
        .add_plugins(VisualCueAnimationsPlugin)
        .add_plugins(WorldInspectorPlugin::default())
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let center = Transform::from_xyz(0.0, 1.0, 0.0).translation;
    let cube = Mesh3d(meshes.add(Cuboid::from_size((1.0, 1.0, 1.0).into())));
    // spinning cube
    commands.spawn((
        cube.clone(),
        MeshMaterial3d::<StandardMaterial>::default(),
        Transform::from_translation(center + Vec3::new(-1.0, 0.0, 0.0)),
        Spinning::default(),
        Name::new("Spinning Cube"),
    ));

    // bobbing cube
    commands.spawn((
        cube,
        MeshMaterial3d::<StandardMaterial>::default(),
        Transform::from_translation(center + Vec3::new(1.0, 0.0, 0.0)),
        Bobbing::default(),
        Name::new("Bobbing Cube"),
    ));
}
