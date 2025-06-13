use std::marker::PhantomData;
use std::ops::Deref;

use bevy_app::prelude::*;

use crate::{resources::{CameraBlockerRegistry, CameraControlBlocked, CameraControlMesh, CameraControlPanMaterial, CameraOrbitMat}, systems::*, *};

pub struct CameraBlockerRegistration<T: Resource + Deref<Target = bool>> {
    _phantom: PhantomData<T>,
}

impl<T: Resource + Deref<Target = bool>> Default for CameraBlockerRegistration<T> {
    fn default() -> Self {
        Self { _phantom: Default::default() }
    }
}

impl<T: Resource + Deref<Target = bool>> Plugin for CameraBlockerRegistration<T> {
    fn build(&self, app: &mut App) {
        app
        .add_systems(PreUpdate, update_blocker_registry::<T>.run_if(resource_changed::<T>))
        ;
    }
}

pub struct CameraControlsPlugin;

impl Plugin for CameraControlsPlugin {
    fn build(&self, app: &mut App) {
        app
        .register_type::<CameraOrbitMat>()
        .register_type::<CameraControlMesh>()
        .register_type::<CameraControlPanMaterial>()
        .register_type::<ProjectionMode>()
        .register_type::<CameraBlockerRegistry>()
        .init_resource::<CameraControlBlocked>()
        .init_resource::<CameraBlockerRegistry>()
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
                set_block_status,
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