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
use crate::interaction::{InteractionAssets, PickingBlockers};
use bevy::{
    color::palettes::css as Colors,
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        camera::{Camera, ClearColorConfig, Exposure, Projection, ScalingMode},
        view::RenderLayers,
    },
};

mod utils;
use utils::*;

mod cursor;
use cursor::{update_cursor_command, CursorCommand};

mod keyboard;
use keyboard::{update_keyboard_command, KeyboardCommand};

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
/// The X-Ray layer is used to show visual cues that need to be rendered
/// above anything that would be obstructing them.
pub const XRAY_RENDER_LAYER: usize = 5;
/// The Model Preview layer is used by model previews to spawn and render
/// models in the engine without having them being visible to general cameras
pub const MODEL_PREVIEW_LAYER: usize = 6;

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
    pub selection_marker: Entity,
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
        // startup
        let interaction_assets = world.get_resource::<InteractionAssets>().expect(
            "make sure that the InteractionAssets resource is initialized before the camera plugin",
        );
        let selection_mesh = interaction_assets.camera_control_mesh.clone();
        let selection_marker = world
            .spawn((
                Mesh3d(selection_mesh),
                Visibility::Visible,
                Transform::default(),
                MeshMaterial3d::<StandardMaterial>::default(),
            ))
            .id();

        let perspective_headlight = world
            .spawn(DirectionalLight {
                shadows_enabled: false,
                illuminance: 50.,
                ..default()
            })
            .insert(main_view_render_layers())
            .id();

        let perspective_child_cameras = [
            (1, SELECTED_OUTLINE_LAYER),
            (2, HOVERED_OUTLINE_LAYER),
            (3, XRAY_RENDER_LAYER),
        ]
        .map(|(order, layer)| {
            world
                .spawn(Camera3d::default())
                .insert((
                    Projection::Perspective(Default::default()),
                    Camera {
                        order,
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    Tonemapping::ReinhardLuminance,
                    Exposure {
                        ev100: DEFAULT_CAMERA_EV100,
                    },
                ))
                .insert(Visibility::Inherited)
                .insert(RenderLayers::layer(layer))
                .id()
        });

        let perspective_base_camera = world
            .spawn(Camera3d::default())
            .insert((
                Transform::from_xyz(-10., -10., 10.).looking_at(Vec3::ZERO, Vec3::Z),
                Projection::Perspective(Default::default()),
                Exposure {
                    ev100: DEFAULT_CAMERA_EV100,
                },
                Tonemapping::ReinhardLuminance,
            ))
            .insert(Visibility::Inherited)
            .insert(RenderLayers::from_layers(&[
                GENERAL_RENDER_LAYER,
                VISUAL_CUE_RENDER_LAYER,
            ]))
            .add_children(&[perspective_headlight])
            .add_children(&perspective_child_cameras)
            .id();

        let orthographic_headlight = world
            .spawn((
                DirectionalLight {
                    shadows_enabled: false,
                    illuminance: 50.,
                    ..default()
                },
                Transform::from_rotation(Quat::from_axis_angle(
                    Vec3::new(1., 1., 0.).normalize(),
                    35_f32.to_radians(),
                )),
            ))
            .insert(main_view_render_layers())
            .id();

        let ortho_projection = OrthographicProjection {
            viewport_origin: Vec2::new(0.5, 0.5),
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 1.0,
            },
            scale: 10.0,
            ..OrthographicProjection::default_3d()
        };

        let orthographic_child_cameras = [
            (1, SELECTED_OUTLINE_LAYER),
            (2, HOVERED_OUTLINE_LAYER),
            (3, XRAY_RENDER_LAYER),
        ]
        .map(|(order, layer)| {
            world
                .spawn(Camera3d::default())
                .insert((
                    Camera {
                        is_active: false,
                        order,
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    Projection::Orthographic(ortho_projection.clone()),
                    Exposure {
                        ev100: DEFAULT_CAMERA_EV100,
                    },
                    Tonemapping::ReinhardLuminance,
                ))
                .insert(Visibility::Inherited)
                .insert(RenderLayers::layer(layer))
                .id()
        });

        let orthographic_camera_entity = world
            .spawn(Camera3d::default())
            .insert((
                Camera {
                    is_active: false,
                    ..default()
                },
                Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
                Projection::Orthographic(ortho_projection),
                Exposure {
                    ev100: DEFAULT_CAMERA_EV100,
                },
                Tonemapping::ReinhardLuminance,
            ))
            .insert(Visibility::Inherited)
            .insert(RenderLayers::from_layers(&[
                GENERAL_RENDER_LAYER,
                VISUAL_CUE_RENDER_LAYER,
            ]))
            .add_children(&[orthographic_headlight])
            .add_children(&orthographic_child_cameras)
            .id();

        let mut ambient_light = world
            .get_resource_mut::<AmbientLight>()
            .expect("Make sure bevy's PbrPlugin is initialized before the cameras");

        ambient_light.brightness = 2.0;

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
            selection_marker,
            orbit_center: None,
        }
    }
}

fn camera_controls(
    mut cursor_command: ResMut<CursorCommand>,
    mut keyboard_command: ResMut<KeyboardCommand>,
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

    let translation_delta: Vec3;
    let rotation_delta: Quat;
    let fov_delta: f32;
    let scale_delta: f32;
    if cursor_command.command_type != CameraCommandType::Inactive {
        translation_delta = cursor_command.take_translation_delta();
        rotation_delta = cursor_command.take_rotation_delta();
        fov_delta = cursor_command.take_fov_delta();
        scale_delta = cursor_command.take_scale_delta();
    } else {
        translation_delta = keyboard_command.take_translation_delta();
        rotation_delta = keyboard_command.take_rotation_delta();
        fov_delta = keyboard_command.take_fov_delta();
        scale_delta = keyboard_command.take_scale_delta();
    }

    if controls.mode() == ProjectionMode::Perspective {
        let (mut persp_proj, mut persp_transform) = cameras
            .get_mut(controls.perspective_camera_entities[0])
            .unwrap();
        if let Projection::Perspective(persp_proj) = persp_proj.as_mut() {
            persp_transform.translation += translation_delta;
            persp_transform.rotation *= rotation_delta;
            persp_proj.fov += fov_delta;
            persp_proj.fov = persp_proj
                .fov
                .clamp(MIN_FOV.to_radians(), MAX_FOV.to_radians());

            // Ensure upright
            let forward = persp_transform.forward();
            persp_transform.look_to(*forward, Vec3::Z);
        }

        let proj = persp_proj.clone();
        let children = cameras
            .get_many_mut(controls.perspective_camera_entities)
            .unwrap();
        for (mut child_proj, _) in children {
            *child_proj = proj.clone();
        }
    }

    if controls.mode() == ProjectionMode::Orthographic {
        let (mut ortho_proj, mut ortho_transform) = cameras
            .get_mut(controls.orthographic_camera_entities[0])
            .unwrap();
        if let Projection::Orthographic(ortho_proj) = ortho_proj.as_mut() {
            ortho_transform.translation += translation_delta;
            ortho_transform.rotation *= rotation_delta;
            ortho_proj.scale += scale_delta;
        }

        let proj = ortho_proj.clone();
        let children = cameras
            .get_many_mut(controls.orthographic_camera_entities)
            .unwrap();
        for (mut child_proj, _) in children {
            *child_proj = proj.clone();
        }
    }
}

fn update_orbit_center_marker(
    controls: Res<CameraControls>,
    keyboard_command: Res<KeyboardCommand>,
    cursor_command: Res<CursorCommand>,
    interaction_assets: Res<InteractionAssets>,
    mut gizmo: Gizmos,
    mut marker_query: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<Projection>,
    >,
) {
    if let Ok((mut marker_transform, mut marker_visibility, mut marker_material)) =
        marker_query.get_mut(controls.selection_marker)
    {
        // Orbitting
        if (cursor_command.command_type == CameraCommandType::Orbit
            || keyboard_command.command_type == CameraCommandType::Orbit)
            && controls.mode() == ProjectionMode::Perspective
        {
            if let Some(orbit_center) = controls.orbit_center {
                *marker_visibility = Visibility::Visible;
                *marker_material =
                    MeshMaterial3d(interaction_assets.camera_control_orbit_material.clone());
                marker_transform.translation = orbit_center;
                gizmo.sphere(
                    Isometry3d::new(orbit_center, Quat::IDENTITY),
                    0.1,
                    Colors::LIME,
                );
            }
        // Panning
        } else if cursor_command.command_type == CameraCommandType::Pan {
            if let Some(cursor_selection) = cursor_command.cursor_selection {
                *marker_visibility = Visibility::Visible;
                *marker_material =
                    MeshMaterial3d(interaction_assets.camera_control_pan_material.clone());
                marker_transform.translation = cursor_selection;
                gizmo.sphere(
                    Isometry3d::new(cursor_selection, Quat::IDENTITY),
                    0.1,
                    Colors::WHITE,
                );
            }
        } else {
            *marker_visibility = Visibility::Hidden;
        }
    }
}

pub struct CameraControlsPlugin;

impl Plugin for CameraControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraControls>()
            .init_resource::<CursorCommand>()
            .init_resource::<KeyboardCommand>()
            .init_resource::<HeadlightToggle>()
            .add_event::<ChangeProjectionMode>()
            .add_systems(
                Update,
                (
                    update_cursor_command,
                    update_keyboard_command,
                    camera_controls,
                )
                    .chain(),
            )
            .add_systems(Update, update_orbit_center_marker);
    }
}
