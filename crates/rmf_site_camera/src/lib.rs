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
use bevy_render::{camera::{Exposure, ScalingMode}, prelude::*, view::RenderLayers};

use bevy_color::palettes::css::{LIME, WHITE};
mod utils;
use bevy_ecs::resource::Resource;
use bevy_math::{Isometry3d, Quat, Rect, Vec2, Vec3};
use bevy_transform::components::Transform;
use bevy_utils::default;

use tracing::warn;
use utils::*;

mod cursor;
use cursor::{update_cursor_command, CursorCommand};

mod keyboard;
use keyboard::{update_keyboard_command, KeyboardCommand};

use crate::{components::{OrthographicCameraRoot, PerspectiveCameraRoot}, plugins::BlockerRegistration, resources::{CameraBlockerRegistry, ProjectionMode}};


pub mod plugins;
mod systems;
pub mod components;
pub mod resources;

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

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum CameraCommandType {
    Inactive,
    Pan,
    Orbit,
    TranslationZoom,
    ScaleZoom,
    FovZoom,
}

#[derive(Default, Reflect)]
struct OrbitCenterGizmo {}

#[derive(Debug, Clone, Reflect, Resource, Default)]
pub struct CameraControls {
    // pub perspective_camera_entities: [Entity; 4],
    // pub perspective_headlight: Entity,
    // pub orthographic_camera_entities: [Entity; 4],
    // pub orthographic_headlight: Entity,
    // pub selection_marker: Entity,
    pub orbit_center: Option<Vec3>,
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
    pub persp_cam: Query<'w, 's, Entity, With<PerspectiveCameraRoot>>
}
/// convienience method to get active camera and output a warning if it does't exist.
pub fn active_camera_maybe(
    active_cam: &ActiveCameraQuery
) -> Result<Entity, QuerySingleError> {
    match *active_cam.proj_mode {
        ProjectionMode::Perspective =>  active_cam.persp_cam.single()
        .inspect_err(|err| warn!("could not get active camera due to: {:#}", err))
        ,
        ProjectionMode::Orthographic => active_cam.ortho_cam.single()
        .inspect_err(|err| warn!("could not get active camera due to: {:#}", err))
        ,
    }
}

pub type CameraBlockerRegistration<T> = BlockerRegistration<T, CameraBlockerRegistry>;