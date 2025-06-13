use bevy_app::prelude::*;

use crate::{resources::{CameraControlMesh, CameraControlPanMaterial, CameraOrbitMat}, systems::*, *};

pub struct CameraControlsPlugin;

impl Plugin for CameraControlsPlugin {
    fn build(&self, app: &mut App) {
        app
        .register_type::<CameraOrbitMat>()
        .register_type::<CameraControlMesh>()
        .register_type::<CameraControlPanMaterial>()
        .register_type::<ProjectionMode>()
        .init_resource::<CursorCommand>()
        .init_resource::<KeyboardCommand>()
        .init_resource::<HeadlightToggle>()
        .init_resource::<CameraControls>()
        .init_resource::<CameraOrbitMat>()
        .init_resource::<CameraControlMesh>()
        .init_resource::<CameraControlPanMaterial>()
        .init_resource::<ProjectionMode>()
        .add_systems(PostStartup, init_cameras)
        .add_systems(Update, toggle_headlights.run_if(resource_changed::<HeadlightToggle>))
        .add_systems(Update, change_projection_mode.run_if(resource_changed::<ProjectionMode>))
        .add_systems(
            Update,
            (
                update_cursor_command,
                update_keyboard_command,
                camera_controls,
            )
                .chain(),
        )
        .add_systems(Update, update_orbit_center_marker)
        ;
    }
}