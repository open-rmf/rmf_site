//! Example of setting up picking
//! TODO: Add a picking example here.

use bevy::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use rmf_site_camera::plugins::CameraSetupPlugin;
use rmf_site_picking::{InspectorService, InspectorServicePlugin, SelectionPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false
        })
        .add_plugins(CameraSetupPlugin)
        .add_plugins(InspectorServicePlugin)
        .add_plugins(SelectionPlugin::<InspectorService>::default())
        .add_plugins(WorldInspectorPlugin::default())
        .add_systems(Startup, setup)
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
        MeshMaterial3d(materials.add(Color::Srgba(Srgba::GREEN * 0.25) )),
        Transform::from_xyz(0.0, 0.0, -0.5),
        Name::new("base_plate"),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}
