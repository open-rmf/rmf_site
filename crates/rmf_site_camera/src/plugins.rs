use std::{collections::HashMap, marker::PhantomData};
use std::ops::Deref;

use bevy_app::prelude::*;
use bytemuck::TransparentWrapper;

use crate::{resources::{CameraBlockerRegistry, CameraControlBlocked, CameraControlMesh, CameraControlPanMaterial, CameraOrbitMat, TypeInfo}, systems::*, *};

///plugin to add a blocker to a registry of blockers. 
/// 
/// E.G: [`UiHovered`] -> CameraControlBlockers to block camera controls on ui hovered.
pub struct BlockerRegistration<Blocker, Registry> 
    where
        Blocker: Resource + TransparentWrapper<bool>,
        Registry: Resource + TransparentWrapper<HashMap<TypeInfo, bool>>,

{
    _a: PhantomData<Blocker>,
    _b: PhantomData<Registry>,
}

impl<Blocker, Registry> Default for BlockerRegistration<Blocker, Registry> 
    where
        Blocker: Resource + TransparentWrapper<bool>,
        Registry: Resource + TransparentWrapper<HashMap<TypeInfo, bool>>,
{
    fn default() -> Self {
        Self { _a: Default::default(), _b: Default::default() }
    }
}

impl<Blocker, Registry> Plugin for BlockerRegistration<Blocker, Registry> 
    where
        Blocker: Resource + TransparentWrapper<bool>,
        Registry: Resource + TransparentWrapper<HashMap<TypeInfo, bool>>,
{
    fn build(&self, app: &mut App) {
        app
        .add_systems(PreUpdate, update_blocker_registry::<Blocker, Registry>.run_if(resource_changed::<Blocker>))
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
        .register_type::<TypeInfo>()
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