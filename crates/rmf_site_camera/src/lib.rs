use std::{
    any::{TypeId, type_name},
    collections::HashMap,
};

/*
 * Copyright (C) 2022 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/
use bevy_core_pipeline::{prelude::*, tonemapping::Tonemapping};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, query::QuerySingleError, system::SystemParam};
use bevy_gizmos::gizmos::Gizmos;
use bevy_pbr::{AmbientLight, DirectionalLight, MeshMaterial3d, StandardMaterial};
use bevy_reflect::Reflect;
use bevy_render::{
    camera::{Exposure, ScalingMode},
    prelude::*,
    view::RenderLayers,
};

use bevy_color::palettes::css::{LIME, WHITE};
use bevy_ecs::resource::Resource;
use bevy_math::{Isometry3d, Quat, Rect, Vec2, Vec3};
use bevy_transform::components::Transform;
use bevy_utils::default;

use bytemuck::TransparentWrapper;
use tracing::warn;
use utils::*;

mod cursor;
use cursor::{CursorCommand, update_cursor_command};

mod keyboard;
use keyboard::{KeyboardCommand, update_keyboard_command};

pub mod components;
pub use components::*;

pub mod plugins;
pub use plugins::*;

pub mod resources;
pub use resources::*;

mod systems;
pub(crate) use systems::*;

pub(crate) mod utils;

/// RenderLayers are used to inform cameras which entities they should render.
/// The General render layer is for things that should be visible to all
/// cameras.
pub const GENERAL_RENDER_LAYER: usize = 0;
/// The Physical render layer is for things that should be visible to any camera
/// that needs to capture the physical world (e.g. the physical camera sensor
/// simulator) but should not be rendered by the user's view. This allows us to
/// toggle off complex PBR lights for the user's view (which can severely slow
/// down performance) while keeping them for camera sensors.
pub const PHYSICAL_RENDER_LAYER: usize = 1;
/// The Visual Cue layer is for things that should be shown to the user but
/// should never appear in a physical camera.
pub const VISUAL_CUE_RENDER_LAYER: usize = 2;
/// The Selected Outline layer is where the outline of the currently selected
/// entity is shown.
pub const SELECTED_OUTLINE_LAYER: usize = 3;
/// The Hovered Outline layer is where the outline of the currently hovered
/// entity is shown.
pub const HOVERED_OUTLINE_LAYER: usize = 4;
/// The Model Preview layer is used by model previews to spawn and render
/// models in the engine without having them being visible to general cameras
pub const MODEL_PREVIEW_LAYER: usize = 6;

/// The X-Ray layer is used to show visual cues that need to be rendered
/// above anything that would be obstructing them.
pub const XRAY_RENDER_LAYER: usize = 5;

/// This resource keeps track of the region that the user camera display is
/// occupying in the window.
#[derive(Resource, Clone, Default)]
pub struct UserCameraDisplay {
    pub region: Rect,
}

// Creates all the layers visible in the main camera view (excluding, for example
// the model preview which is on a separate view). The main lights will affect these.
pub fn main_view_render_layers() -> RenderLayers {
    RenderLayers::from_layers(&[
        GENERAL_RENDER_LAYER,
        PHYSICAL_RENDER_LAYER,
        VISUAL_CUE_RENDER_LAYER,
        SELECTED_OUTLINE_LAYER,
        HOVERED_OUTLINE_LAYER,
        XRAY_RENDER_LAYER,
    ])
}

/// Camera exposure, adjusted for indoor lighting, in ev100 units
pub const DEFAULT_CAMERA_EV100: f32 = 3.5;

/// Camera limits
pub const MIN_FOV: f32 = 5.0;
pub const MAX_FOV: f32 = 120.0;
pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 500.0;
pub const MAX_PITCH: f32 = 85.0;
pub const MIN_SELECTION_DIST: f32 = 10.0;
pub const MAX_SELECTION_DIST: f32 = 30.0;

#[derive(PartialEq, Debug, Copy, Clone, Reflect)]
pub enum CameraCommandType {
    Inactive,
    Pan,
    Orbit,
    TranslationZoom,
    ScaleZoom,
    FovZoom,
}

#[derive(Component)]
#[component(immutable)]
pub struct CameraTarget {
    pub point: Vec3,
}

#[derive(Clone, Copy, Resource)]
pub struct PanToElement {
    pub target: Option<Entity>,
    pub interruptible: bool,
    pub persistent: bool,
}

impl Default for PanToElement {
    fn default() -> Self {
        Self {
            target: None,
            interruptible: true,
            persistent: false,
        }
    }
}

/// True/false for whether the headlight should be on or off
#[derive(Clone, Copy, PartialEq, Eq, Deref, DerefMut, Resource)]
pub struct HeadlightToggle(pub bool);

impl Default for HeadlightToggle {
    fn default() -> Self {
        Self(true)
    }
}
/// convienience [`SystemParam`] set for [`active_camera_maybe`]
#[derive(SystemParam)]
pub struct ActiveCameraQuery<'w, 's> {
    pub proj_mode: Res<'w, ProjectionMode>,
    pub ortho_cam: Query<'w, 's, Entity, With<OrthographicCameraRoot>>,
    pub persp_cam: Query<'w, 's, Entity, With<PerspectiveCameraRoot>>,
}
/// convienience method to get active camera and output a warning if it does't exist.
pub fn active_camera_maybe(active_cam: &ActiveCameraQuery) -> Result<Entity, QuerySingleError> {
    match *active_cam.proj_mode {
        ProjectionMode::Perspective => active_cam
            .persp_cam
            .single()
            .inspect_err(|err| warn!("could not get active camera due to: {:#}", err)),
        ProjectionMode::Orthographic => active_cam
            .ortho_cam
            .single()
            .inspect_err(|err| warn!("could not get active camera due to: {:#}", err)),
    }
}

/// convenience struct for associating type info and type name.
#[derive(Reflect, Hash, Clone, PartialEq, Eq, Debug)]
#[reflect(Clone)]
pub struct TypeInfo {
    /// set to `#[reflect(ignore)]` until <https://github.com/jakobhellermann/bevy-inspector-egui/issues/267>
    /// is resolved
    ///
    /// set to `Option` so `#[reflect(ignore)]` stops complaining about no [`TypeId`] [`Default`] impl.
    ///
    /// TODO: Set this back to just [`TypeId`] when mentioned issue is fixed.
    #[reflect(ignore)]
    type_id: Option<TypeId>,
    /// full crate path of the type
    type_name: String,
    /// short name of the type
    ///
    /// TODO: Remove this in favor of a proper inspector reflection impl for type_name.
    type_name_short: String,
}

impl TypeInfo {
    pub fn new<T: 'static>() -> Self {
        let type_name = type_name::<T>();
        let type_name_short = type_name.split("::").last().unwrap_or("???");
        Self {
            type_id: Some(TypeId::of::<T>()),
            type_name: type_name.to_string(),
            type_name_short: type_name_short.to_string(),
        }
    }
    pub fn type_id(&self) -> TypeId {
        // should never result in a panic.
        self.type_id.unwrap()
    }
    pub fn type_name(&self) -> &String {
        &self.type_name
    }
    pub fn type_name_short(&self) -> &String {
        &self.type_name_short
    }
}

pub type CameraControlsBlocker<T> = BlockerRegistration<T, CameraControlBlockers>;

/// checks if a camera blocking [T] is currently enabled, and block camera if it is.
pub(crate) fn update_blocker_registry<T, U>(blocker_registry: ResMut<U>, camera_blocker: Res<T>)
where
    T: Resource + TransparentWrapper<bool>,
    U: Resource + TransparentWrapper<HashMap<TypeInfo, bool>>,
{
    let blocker_registry = U::peel_mut(blocker_registry.into_inner());
    let blocker = T::peel_ref(camera_blocker.into_inner());

    let type_info = TypeInfo::new::<T>();

    if blocker == &true {
        let blocked = blocker_registry.entry(type_info).or_insert(true);
        *blocked = true;
    } else {
        let blocked = blocker_registry.entry(type_info).or_insert(false);
        *blocked = false;
    }
}

/// check if blocker registry has toggled blockers, unblock if it doesn't.
pub(crate) fn set_block_status<U>(block_status: ResMut<BlockStatus<U>>, blocker_registry: Res<U>)
where
    U: Resource + TransparentWrapper<HashMap<TypeInfo, bool>>,
{
    let block_status = BlockStatus::<U>::peel_mut(block_status.into_inner());
    let blocker_registry = U::peel_ref(blocker_registry.into_inner());

    *block_status = blocker_registry.iter().any(|(_, blocker)| blocker == &true);
}
