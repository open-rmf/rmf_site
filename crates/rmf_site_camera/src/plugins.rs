use std::{collections::HashMap, marker::PhantomData};

use bevy_app::prelude::*;
use bytemuck::TransparentWrapper;

use crate::resources::{BlockStatus, CameraConfig, CameraControls};
use crate::{
    resources::{CameraControlBlockers, OrbitMarkerMaterial, PanMarkerMaterial, PickMarkerMesh},
    systems::*,
    *,
};

/// Plugin to add a blocker to a registry of blockers.
///
/// E.G: [`UiFocused`] -> CameraControlBlockers to block camera controls on ui hovered.
#[derive(Default)]
pub struct BlockerRegistration<Blocker, Registry>
where
    Blocker: Resource + TransparentWrapper<bool> + Default,
    Registry: Resource + TransparentWrapper<HashMap<TypeInfo, bool>> + Default,
{
    _a: PhantomData<Blocker>,
    _b: PhantomData<Registry>,
}

impl<Blocker, Registry> Plugin for BlockerRegistration<Blocker, Registry>
where
    Blocker: Resource + TransparentWrapper<bool> + Default,
    Registry: Resource + TransparentWrapper<HashMap<TypeInfo, bool>> + Default,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            update_blocker_registry::<Blocker, Registry>.run_if(resource_changed::<Blocker>),
        );
    }
}

/// Plugin to initialize initailize blocker registry's and their associated toggles.
#[derive(Default)]
pub struct BlockerRegistryPlugin<Registry>
where
    Registry: Resource + TransparentWrapper<HashMap<TypeInfo, bool>> + Default,
{
    _a: PhantomData<Registry>,
}

impl<Registry> Plugin for BlockerRegistryPlugin<Registry>
where
    Registry: Resource + TransparentWrapper<HashMap<TypeInfo, bool>> + Default,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<Registry>()
            .init_resource::<BlockStatus<Registry>>()
            .add_systems(PreUpdate, set_block_status::<Registry>);
    }
}

/// Plugin for project's camera setup.
pub struct CameraSetupPlugin;

impl Plugin for CameraSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BlockerRegistryPlugin::<CameraControlBlockers>::default())
            .register_type::<OrbitMarkerMaterial>()
            .register_type::<PickMarkerMesh>()
            .register_type::<PanMarkerMaterial>()
            .register_type::<ProjectionMode>()
            .register_type::<CameraControlBlockers>()
            .register_type::<TypeInfo>()
            .register_type::<KeyboardCommand>()
            .register_type::<CursorCommand>()
            .init_resource::<CursorCommand>()
            .init_resource::<KeyboardCommand>()
            .init_resource::<HeadlightToggle>()
            .init_resource::<CameraConfig>()
            .init_resource::<OrbitMarkerMaterial>()
            .init_resource::<PickMarkerMesh>()
            .init_resource::<PanMarkerMaterial>()
            .init_resource::<ProjectionMode>()
            .init_resource::<CameraControls>()
            .add_systems(PostStartup, init_cameras)
            .add_systems(
                Update,
                toggle_headlights.run_if(resource_changed::<HeadlightToggle>),
            )
            .add_systems(
                Update,
                change_projection_mode.run_if(resource_changed::<ProjectionMode>),
            )
            .add_systems(
                Update,
                (
                    update_cursor_command,
                    update_keyboard_command,
                    camera_config,
                )
                    .chain(),
            )
            .add_systems(Update, update_orbit_center_marker);
    }
}
