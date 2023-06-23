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
    input::mouse::{MouseButton, MouseWheel},
    prelude::*,
    render::{
        camera::{Camera, Projection, ScalingMode, WindowOrigin},
        view::RenderLayers,
    },
};

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
/// The Picking layer contains an entity based color map for picking entities
/// which are drawn in screen space.
pub const PICKING_LAYER: u8 = 6;

#[derive(Resource)]
struct MouseLocation {
    previous: Vec2,
}

impl Default for MouseLocation {
    fn default() -> Self {
        MouseLocation {
            previous: Vec2::ZERO,
        }
    }
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
                visibility.is_visible = choice;
            }
        }

        if let Ok(cameras) = cameras.get_many_mut(self.orthographic_camera_entities) {
            for mut camera in cameras {
                camera.is_active = !choice;
            }
        }

        if let Ok(visibilities) = visibilities.get_many_mut(self.orthographic_camera_entities) {
            for mut visibility in visibilities {
                visibility.is_visible = !choice;
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
            v.is_visible = toggle && self.mode.is_perspective();
        }

        if let Ok(mut v) = visibility.get_mut(self.orthographic_headlight) {
            v.is_visible = toggle && self.mode.is_orthographic();
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
        .map(|(priority, layer)| {
            world
                .spawn(Camera3dBundle {
                    projection: Projection::Perspective(Default::default()),
                    camera: Camera {
                        priority,
                        ..default()
                    },
                    camera_3d: Camera3d {
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    ..default()
                })
                .insert(Visibility::VISIBLE)
                .insert(ComputedVisibility::default())
                .insert(RenderLayers::layer(layer))
                .id()
        });

        let perspective_base_camera = world
            .spawn(Camera3dBundle {
                transform: Transform::from_xyz(-10., -10., 10.).looking_at(Vec3::ZERO, Vec3::Z),
                projection: Projection::Perspective(Default::default()),
                ..default()
            })
            .insert(Visibility::VISIBLE)
            .insert(ComputedVisibility::default())
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
            window_origin: WindowOrigin::Center,
            scaling_mode: ScalingMode::FixedVertical(1.0),
            scale: 10.0,
            ..default()
        };

        let orthographic_child_cameras = [
            (1, SELECTED_OUTLINE_LAYER),
            (2, HOVERED_OUTLINE_LAYER),
            (3, XRAY_RENDER_LAYER),
        ]
        .map(|(priority, layer)| {
            world
                .spawn(Camera3dBundle {
                    camera: Camera {
                        is_active: false,
                        priority,
                        ..default()
                    },
                    camera_3d: Camera3d {
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    projection: Projection::Orthographic(ortho_projection.clone()),
                    ..default()
                })
                .insert(Visibility::VISIBLE)
                .insert(ComputedVisibility::default())
                .insert(RenderLayers::layer(XRAY_RENDER_LAYER))
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
                ..default()
            })
            .insert(Visibility::VISIBLE)
            .insert(ComputedVisibility::default())
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
    windows: Res<Windows>,
    mut ev_cursor_moved: EventReader<CursorMoved>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    input_keyboard: Res<Input<KeyCode>>,
    mut previous_mouse_location: ResMut<MouseLocation>,
    mut controls: ResMut<CameraControls>,
    mut cameras: Query<(&mut Projection, &mut Transform)>,
    mut visibility: Query<&mut Visibility>,
    headlight_toggle: Res<HeadlightToggle>,
    picking_blockers: Res<PickingBlockers>,
) {
    if headlight_toggle.is_changed() {
        controls.toggle_lights(headlight_toggle.0, &mut visibility);
    }

    // give input priority to ui elements
    if picking_blockers.ui {
        return;
    }

    let is_shifting =
        input_keyboard.pressed(KeyCode::LShift) || input_keyboard.pressed(KeyCode::LShift);
    let is_panning = input_mouse.pressed(MouseButton::Right) && !is_shifting;

    let is_orbiting = input_mouse.pressed(MouseButton::Middle)
        || (input_mouse.pressed(MouseButton::Right) && is_shifting);
    let started_orbiting = !controls.was_oribiting && is_orbiting;
    let released_orbiting = controls.was_oribiting && !is_orbiting;
    controls.was_oribiting = is_orbiting;

    // spin through all mouse cursor-moved events to find the last one
    let mut last_pos = previous_mouse_location.previous;
    if let Some(ev) = ev_cursor_moved.iter().last() {
        last_pos.x = ev.position.x;
        last_pos.y = ev.position.y;
    }

    let mut cursor_motion = Vec2::ZERO;
    if is_panning || is_orbiting {
        cursor_motion.x = last_pos.x - previous_mouse_location.previous.x;
        cursor_motion.y = last_pos.y - previous_mouse_location.previous.y;
    }

    previous_mouse_location.previous = last_pos;

    let mut scroll = 0.0;
    for ev in ev_scroll.iter() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            scroll += ev.y;
        }
        #[cfg(target_arch = "wasm32")]
        {
            // scrolling in wasm is a different beast
            scroll += 0.4 * ev.y / ev.y.abs();
        }
    }

    if controls.mode() == ProjectionMode::Orthographic {
        let (mut ortho_proj, mut ortho_transform) = cameras
            .get_mut(controls.orthographic_camera_entities[0])
            .unwrap();
        if let Projection::Orthographic(ortho_proj) = ortho_proj.as_mut() {
            if let Some(window) = windows.get_primary() {
                let window_size = Vec2::new(window.width() as f32, window.height() as f32);
                let aspect_ratio = window_size[0] / window_size[1];

                if cursor_motion.length_squared() > 0.0 {
                    cursor_motion *= 2. / window_size
                        * Vec2::new(ortho_proj.scale * aspect_ratio, ortho_proj.scale);
                    let right = -cursor_motion.x * Vec3::X;
                    let up = -cursor_motion.y * Vec3::Y;
                    ortho_transform.translation += right + up;
                }
                if scroll.abs() > 0.0 {
                    ortho_proj.scale -= scroll * ortho_proj.scale * 0.1;
                    ortho_proj.scale = f32::max(ortho_proj.scale, 0.02);
                }
            }
        }

        let proj = ortho_proj.clone();
        let mut children = cameras
            .get_many_mut(controls.orthographic_camera_entities)
            .unwrap();
        for (mut child_proj, _) in children {
            *child_proj = proj.clone();
        }
    } else {
        // perspective mode
        let (mut persp_proj, mut persp_transform) = cameras
            .get_mut(controls.perspective_camera_entities[0])
            .unwrap();
        if let Projection::Perspective(persp_proj) = persp_proj.as_mut() {
            let mut changed = false;

            if started_orbiting || released_orbiting {
                // only check for upside down when orbiting started or ended this frame
                // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
                let up = persp_transform.rotation * Vec3::Z;
                controls.orbit_upside_down = up.z <= 0.0;
            }

            if is_orbiting && cursor_motion.length_squared() > 0. {
                changed = true;
                if let Some(window) = windows.get_primary() {
                    let window_size = Vec2::new(window.width() as f32, window.height() as f32);
                    let delta_x = {
                        let delta = cursor_motion.x / window_size.x * std::f32::consts::PI * 2.0;
                        if controls.orbit_upside_down {
                            -delta
                        } else {
                            delta
                        }
                    };
                    let delta_y = -cursor_motion.y / window_size.y * std::f32::consts::PI;
                    let yaw = Quat::from_rotation_z(-delta_x);
                    let pitch = Quat::from_rotation_x(-delta_y);
                    persp_transform.rotation = yaw * persp_transform.rotation; // global y
                    persp_transform.rotation = persp_transform.rotation * pitch;
                    // local x
                }
            } else if is_panning && cursor_motion.length_squared() > 0. {
                changed = true;
                // make panning distance independent of resolution and FOV,
                if let Some(window) = windows.get_primary() {
                    let window_size = Vec2::new(window.width() as f32, window.height() as f32);

                    cursor_motion *=
                        Vec2::new(persp_proj.fov * persp_proj.aspect_ratio, persp_proj.fov)
                            / window_size;
                    // translate by local axes
                    let right = persp_transform.rotation * Vec3::X * -cursor_motion.x;
                    let up = persp_transform.rotation * Vec3::Y * -cursor_motion.y;
                    // make panning proportional to distance away from center point
                    let translation = (right + up) * controls.orbit_radius;
                    controls.orbit_center += translation;
                }
            }

            if scroll.abs() > 0.0 {
                changed = true;
                controls.orbit_radius -= scroll * controls.orbit_radius * 0.2;
                // dont allow zoom to reach zero or you get stuck
                controls.orbit_radius = f32::max(controls.orbit_radius, 0.05);
            }

            if changed {
                // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
                // parent = x and y rotation
                // child = z-offset
                let rot_matrix = Mat3::from_quat(persp_transform.rotation);
                persp_transform.translation = controls.orbit_center
                    + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, controls.orbit_radius));
            }
        }
    }
}

pub struct CameraControlsPlugin;

impl Plugin for CameraControlsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MouseLocation::default())
            .init_resource::<CameraControls>()
            .init_resource::<HeadlightToggle>()
            .add_system(camera_controls);
    }
}
