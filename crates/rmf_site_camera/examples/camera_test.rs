use bevy::prelude::*;
use bevy_color::palettes::css as Colors;
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiPlugin},
    quick::WorldInspectorPlugin,
};
use bytemuck::TransparentWrapper;
use rmf_site_camera::{
    CameraControlsBlocker, plugins::CameraSetupPlugin, resources::ProjectionMode,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            // (rydb): this crashed the last time I set this to true. Keeping as false for now
            enable_multipass_for_primary_context: false,
        })
        .init_resource::<UiHoveredExample>()
        .add_plugins(MeshPickingPlugin)
        .add_plugins(CameraSetupPlugin)
        .add_plugins(CameraControlsBlocker::<UiHoveredExample>::default())
        .add_plugins(WorldInspectorPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, camera_config)
        .add_systems(Update, window_hover_status)
        .run();
}

#[derive(Reflect, Resource, TransparentWrapper, Default)]
#[reflect(Resource)]
#[repr(transparent)]
pub struct UiHoveredExample(pub bool);

fn camera_config(mut projection_mode: ResMut<ProjectionMode>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F2) {
        *projection_mode = ProjectionMode::Orthographic;
    }

    if keys.just_pressed(KeyCode::F3) {
        *projection_mode = ProjectionMode::Perspective;
    }
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
        MeshMaterial3d(materials.add(Color::Srgba(Colors::DARK_GREEN))),
        Transform::from_xyz(0.0, 0.0, -0.5),
        Name::new("base_plate"),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Name::new("cube"),
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

/// check current window hover status over ui element.
pub fn window_hover_status(
    mut ui_hovered: ResMut<UiHoveredExample>,
    mut windows: Query<&mut EguiContext>,
) {
    let Ok(mut window) = windows
        .single_mut()
        .inspect_err(|err| warn!("can't check window hover status: {:#}", err))
    else {
        return;
    };
    ui_hovered.0 = window.get_mut().is_pointer_over_area();
}
