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

use crate::interaction::PickingBlockers;
use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    core_pipeline::core_3d::Camera3dBundle,
    core_pipeline::tonemapping::Tonemapping,
    input::mouse::{MouseButton, MouseWheel},
    prelude::*,
    render::{
        camera::{Camera, Projection, ScalingMode},
        view::RenderLayers,
    },
    window::PrimaryWindow,
};

mod cursor;
use cursor::{CursorCommand, update_cursor_command};

/// RenderLayers are used to inform cameras which entities they should render.
/// The General render layer is for things that should be visible to all
/// cameras.
pub const GENERAL_RENDER_LAYER: u8 = 0;
/// The Physical render layer is for things that should be visible to any camera
/// that needs to capture the physical world (e.g. the physical camera sensor
/// simulator) but should not be rendered by the user's view. This allows us to
/// toggle off complex PBR lights for the user's view (which can severely slow
/// down performance) while keeping them for camera sensors.
pub const PHYSICAL_RENDER_LAYER: u8 = 1;
/// The Visual Cue layer is for things that should be shown to the user but
/// should never appear in a physical camera.
pub const VISUAL_CUE_RENDER_LAYER: u8 = 2;
/// The Selected Outline layer is where the outline of the currently selected
/// entity is shown.
pub const SELECTED_OUTLINE_LAYER: u8 = 3;
/// The Hovered Outline layer is where the outline of the currently hovered
/// entity is shown.
pub const HOVERED_OUTLINE_LAYER: u8 = 4;
/// The X-Ray layer is used to show visual cues that need to be rendered
/// above anything that would be obstructing them.
pub const XRAY_RENDER_LAYER: u8 = 5;
/// The Model Preview layer is used by model previews to spawn and render
/// models in the engine without having them being visible to general cameras
pub const MODEL_PREVIEW_LAYER: u8 = 6;


#[derive(PartialEq, Debug, Copy, Clone)]
pub enum CameraCommandType {
    Inactive,
    Pan,
    Orbit,
    ZoomOnly
}

#[derive(PartialEq, Debug, Copy, Clone, Reflect, Resource)]
pub enum ProjectionMode {
    Perspective,
    Orthographic,
}

impl ProjectionMode {
    pub fn is_perspective(&self) -> bool {
        matches!(self, Self::Perspective)
    }

    pub fn is_orthographic(&self) -> bool {
        matches!(self, Self::Orthographic)
    }
}

#[derive(Event)]
pub struct ChangeProjectionMode(pub ProjectionMode);

impl ChangeProjectionMode {
    pub fn to_perspective() -> ChangeProjectionMode {
        ChangeProjectionMode(ProjectionMode::Perspective)
    }

    pub fn to_orthographic() -> ChangeProjectionMode {
        ChangeProjectionMode(ProjectionMode::Orthographic)
    }
}

#[derive(Debug, Clone, Reflect, Resource)]
pub struct CameraControls {
    mode: ProjectionMode,
    pub perspective_camera_entities: [Entity; 4],
    pub perspective_headlight: Entity,
    pub orthographic_camera_entities: [Entity; 4],
    pub orthographic_headlight: Entity,
    pub orbit_center: Vec3,
    pub orbit_radius: f32,
    pub orbit_upside_down: bool,
    pub was_oribiting: bool,
}

/// True/false for whether the headlight should be on or off
#[derive(Clone, Copy, PartialEq, Eq, Deref, DerefMut, Resource)]
pub struct HeadlightToggle(pub bool);

impl Default for HeadlightToggle {
    fn default() -> Self {
        Self(true)
    }
}

impl CameraControls {
    pub fn use_perspective(
        &mut self,
        choice: bool,
        cameras: &mut Query<&mut Camera>,
        visibilities: &mut Query<&mut Visibility>,
        headlights_on: bool,
    ) {
        if let Ok(cameras) = cameras.get_many_mut(self.perspective_camera_entities) {
            for mut camera in cameras {
                camera.is_active = choice;
            }
        }

        if let Ok(visibilities) = visibilities.get_many_mut(self.perspective_camera_entities) {
            for mut visibility in visibilities {
                *visibility = if choice {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }

        if let Ok(cameras) = cameras.get_many_mut(self.orthographic_camera_entities) {
            for mut camera in cameras {
                camera.is_active = !choice;
            }
        }

        if let Ok(visibilities) = visibilities.get_many_mut(self.orthographic_camera_entities) {
            for mut visibility in visibilities {
                *visibility = if choice {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                };
            }
        }

        if choice {
            self.mode = ProjectionMode::Perspective;
        } else {
            self.mode = ProjectionMode::Orthographic;
        }

        self.toggle_lights(headlights_on, visibilities);
    }

    pub fn use_orthographic(
        &mut self,
        choice: bool,
        cameras: &mut Query<&mut Camera>,
        visibilities: &mut Query<&mut Visibility>,
        headlights_on: bool,
    ) {
        self.use_perspective(!choice, cameras, visibilities, headlights_on);
    }

    pub fn use_mode(
        &mut self,
        mode: ProjectionMode,
        cameras: &mut Query<&mut Camera>,
        visibilities: &mut Query<&mut Visibility>,
        headlights_on: bool,
    ) {
        match mode {
            ProjectionMode::Perspective => {
                self.use_perspective(true, cameras, visibilities, headlights_on);
            }
            ProjectionMode::Orthographic => {
                self.use_orthographic(true, cameras, visibilities, headlights_on);
            }
        }
    }

    pub fn mode(&self) -> ProjectionMode {
        self.mode
    }

    pub fn active_camera(&self) -> Entity {
        match self.mode {
            ProjectionMode::Perspective => self.perspective_camera_entities[0],
            ProjectionMode::Orthographic => self.orthographic_camera_entities[0],
        }
    }

    pub fn toggle_lights(&self, toggle: bool, visibility: &mut Query<&mut Visibility>) {
        if let Ok(mut v) = visibility.get_mut(self.perspective_headlight) {
            *v = if toggle && self.mode.is_perspective() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }

        if let Ok(mut v) = visibility.get_mut(self.orthographic_headlight) {
            *v = if toggle && self.mode.is_orthographic() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

impl FromWorld for CameraControls {
    fn from_world(world: &mut World) -> Self {
        let perspective_headlight = world
            .spawn(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    shadows_enabled: false,
                    illuminance: 20000.,
                    ..default()
                },
                ..default()
            })
            .id();

        let perspective_child_cameras = [
            (1, SELECTED_OUTLINE_LAYER),
            (2, HOVERED_OUTLINE_LAYER),
            (3, XRAY_RENDER_LAYER),
        ]
        .map(|(order, layer)| {
            world
                .spawn(Camera3dBundle {
                    projection: Projection::Perspective(Default::default()),
                    camera: Camera { order, ..default() },
                    camera_3d: Camera3d {
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    tonemapping: Tonemapping::ReinhardLuminance,
                    ..default()
                })
                .insert(VisibilityBundle {
                    visibility: Visibility::Inherited,
                    ..default()
                })
                .insert(RenderLayers::layer(layer))
                .id()
        });

        let perspective_base_camera = world
            .spawn(Camera3dBundle {
                transform: Transform::from_xyz(-10., -10., 10.).looking_at(Vec3::ZERO, Vec3::Z),
                projection: Projection::Perspective(Default::default()),
                tonemapping: Tonemapping::ReinhardLuminance,
                ..default()
            })
            .insert(VisibilityBundle {
                visibility: Visibility::Inherited,
                ..default()
            })
            .insert(RenderLayers::from_layers(&[
                GENERAL_RENDER_LAYER,
                VISUAL_CUE_RENDER_LAYER,
            ]))
            .push_children(&[perspective_headlight])
            .push_children(&perspective_child_cameras)
            .id();

        let orthographic_headlight = world
            .spawn(DirectionalLightBundle {
                transform: Transform::from_rotation(Quat::from_axis_angle(
                    Vec3::new(1., 1., 0.).normalize(),
                    35_f32.to_radians(),
                )),
                directional_light: DirectionalLight {
                    shadows_enabled: false,
                    illuminance: 20000.,
                    ..default()
                },
                ..default()
            })
            .id();

        let ortho_projection = OrthographicProjection {
            viewport_origin: Vec2::new(0.5, 0.5),
            scaling_mode: ScalingMode::FixedVertical(1.0),
            scale: 10.0,
            ..default()
        };

        let orthographic_child_cameras = [
            (1, SELECTED_OUTLINE_LAYER),
            (2, HOVERED_OUTLINE_LAYER),
            (3, XRAY_RENDER_LAYER),
        ]
        .map(|(order, layer)| {
            world
                .spawn(Camera3dBundle {
                    camera: Camera {
                        is_active: false,
                        order,
                        ..default()
                    },
                    camera_3d: Camera3d {
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    projection: Projection::Orthographic(ortho_projection.clone()),
                    tonemapping: Tonemapping::ReinhardLuminance,
                    ..default()
                })
                .insert(VisibilityBundle {
                    visibility: Visibility::Inherited,
                    ..default()
                })
                .insert(RenderLayers::layer(layer))
                .id()
        });

        let orthographic_camera_entity = world
            .spawn(Camera3dBundle {
                camera: Camera {
                    is_active: false,
                    ..default()
                },
                transform: Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
                projection: Projection::Orthographic(ortho_projection),
                tonemapping: Tonemapping::ReinhardLuminance,
                ..default()
            })
            .insert(VisibilityBundle {
                visibility: Visibility::Inherited,
                ..default()
            })
            .insert(RenderLayers::from_layers(&[
                GENERAL_RENDER_LAYER,
                VISUAL_CUE_RENDER_LAYER,
            ]))
            .push_children(&[orthographic_headlight])
            .push_children(&orthographic_child_cameras)
            .id();

        CameraControls {
            mode: ProjectionMode::Perspective,
            perspective_camera_entities: [
                perspective_base_camera,
                perspective_child_cameras[0],
                perspective_child_cameras[1],
                perspective_child_cameras[2],
            ],
            perspective_headlight,
            orthographic_camera_entities: [
                orthographic_camera_entity,
                orthographic_child_cameras[0],
                orthographic_child_cameras[1],
                orthographic_child_cameras[2],
            ],
            orthographic_headlight,
            orbit_center: Vec3::ZERO,
            orbit_radius: (3.0 * 10.0 * 10.0 as f32).sqrt(),
            orbit_upside_down: false,
            was_oribiting: false,
        }
    }
}

fn camera_controls(
    cursor_command: ResMut<CursorCommand>,
    mut controls: ResMut<CameraControls>,
    mut cameras: Query<(&mut Projection, &mut Transform)>,
    mut bevy_cameras: Query<&mut Camera>,
    mut visibility: Query<&mut Visibility>,
    headlight_toggle: Res<HeadlightToggle>,
    picking_blockers: Res<PickingBlockers>,
    mut change_mode: EventReader<ChangeProjectionMode>,
) {
    if let Some(mode) = change_mode.read().last() {
        controls.use_mode(
            mode.0,
            &mut bevy_cameras,
            &mut visibility,
            headlight_toggle.0,
        );
    }

    if headlight_toggle.is_changed() {
        controls.toggle_lights(headlight_toggle.0, &mut visibility);
    }

    // give input priority to ui elements
    if picking_blockers.ui {
        return;
    }

    if controls.mode() == ProjectionMode::Perspective {
        let (mut persp_proj, mut persp_transform) = cameras
            .get_mut(controls.perspective_camera_entities[0])
            .unwrap();
        if let Projection::Perspective(persp_proj) = persp_proj.as_mut() {
            persp_transform.translation += cursor_command.translation_delta;
            persp_transform.rotation *= cursor_command.rotation_delta;
        }
    }

    if controls.mode() == ProjectionMode::Orthographic {
        return;
    }
}

pub struct CameraControlsPlugin;

impl Plugin for CameraControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraControls>()
            .init_resource::<CursorCommand>()
            .init_resource::<HeadlightToggle>()
            .add_event::<ChangeProjectionMode>()
            .add_systems(Update, update_cursor_command)
            .add_systems(Update, camera_controls);
    }
}
