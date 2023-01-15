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
/// The user's viewport will see all layers, but physical cameras will only see
/// entities in the default layer 0. In the site editor, each layer value has a
/// specific semantic meaning so we need to make sure we never reuse these
/// values.
pub const PHYSICAL_RENDER_LAYER: u8 = 0;
pub const VISUAL_CUE_RENDER_LAYER: u8 = 1;
/// VISUAL_CUE_XRAY_LAYER is used to show visual cues that need to be rendered
/// above anything that would be obstructing them.
pub const VISUAL_CUE_XRAY_LAYER: u8 = 2;

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
#[derive(PartialEq, Debug, Copy, Clone, Reflect)]
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

#[derive(Debug, Clone, Reflect)]
pub struct CameraControls {
    mode: ProjectionMode,
    pub perspective_camera_entity: Entity,
    pub perspective_xray_entity: Entity,
    pub perspective_headlight: Entity,
    pub orthographic_camera_entity: Entity,
    pub orthographic_xray_entity: Entity,
    pub orthographic_headlight: Entity,
    pub orbit_center: Vec3,
    pub orbit_radius: f32,
    pub orbit_upside_down: bool,
    pub was_oribiting: bool,
}

/// True/false for whether the headlight should be on or off
#[derive(Clone, Copy, PartialEq, Eq, Deref, DerefMut)]
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
        if let Ok(cameras) =
            cameras.get_many_mut([self.perspective_camera_entity, self.perspective_xray_entity])
        {
            for mut camera in cameras {
                camera.is_active = choice;
            }
        }

        if let Ok(visibilities) = visibilities
            .get_many_mut([self.perspective_camera_entity, self.perspective_xray_entity])
        {
            for mut visibility in visibilities {
                visibility.is_visible = choice;
            }
        }

        if let Ok(cameras) = cameras.get_many_mut([
            self.orthographic_camera_entity,
            self.orthographic_xray_entity,
        ]) {
            for mut camera in cameras {
                camera.is_active = !choice;
            }
        }

        if let Ok(visibilities) = visibilities.get_many_mut([
            self.orthographic_camera_entity,
            self.orthographic_xray_entity,
        ]) {
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
            ProjectionMode::Perspective => self.perspective_camera_entity,
            ProjectionMode::Orthographic => self.orthographic_camera_entity,
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
            .spawn()
            .insert_bundle(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    shadows_enabled: false,
                    illuminance: 20000.,
                    ..default()
                },
                ..default()
            })
            .id();

        let perspective_xray_entity = world
            .spawn()
            .insert_bundle(Camera3dBundle {
                projection: Projection::Perspective(Default::default()),
                camera: Camera {
                    // Make this render after the main perspective camera
                    priority: 1,
                    ..default()
                },
                camera_3d: Camera3d {
                    // Do not clear the background when rendering this
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                ..default()
            })
            .insert(Visibility::visible())
            .insert(ComputedVisibility::default())
            .insert(RenderLayers::layer(VISUAL_CUE_XRAY_LAYER))
            .id();

        let perspective_camera_entity = world
            .spawn()
            .insert_bundle(Camera3dBundle {
                transform: Transform::from_xyz(-10., -10., 10.).looking_at(Vec3::ZERO, Vec3::Z),
                projection: Projection::Perspective(Default::default()),
                ..default()
            })
            .insert(Visibility::visible())
            .insert(ComputedVisibility::default())
            .insert(RenderLayers::from_layers(&[
                PHYSICAL_RENDER_LAYER,
                VISUAL_CUE_RENDER_LAYER,
            ]))
            .push_children(&[perspective_headlight, perspective_xray_entity])
            .id();

        let orthographic_headlight = world
            .spawn()
            .insert_bundle(DirectionalLightBundle {
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

        let orthographic_xray_entity = world
            .spawn()
            .insert_bundle(Camera3dBundle {
                camera: Camera {
                    is_active: false,
                    // Make this render after the main orthographic camera
                    priority: 1,
                    ..default()
                },
                camera_3d: Camera3d {
                    // Do not clear the background when rendering this
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                projection: Projection::Orthographic(ortho_projection.clone()),
                ..default()
            })
            .insert(Visibility::visible())
            .insert(ComputedVisibility::default())
            .insert(RenderLayers::layer(VISUAL_CUE_XRAY_LAYER))
            .id();

        let orthographic_camera_entity = world
            .spawn()
            .insert_bundle(Camera3dBundle {
                camera: Camera {
                    is_active: false,
                    ..default()
                },
                transform: Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
                projection: Projection::Orthographic(ortho_projection),
                ..default()
            })
            .insert(Visibility::visible())
            .insert(ComputedVisibility::default())
            .insert(RenderLayers::from_layers(&[
                PHYSICAL_RENDER_LAYER,
                VISUAL_CUE_RENDER_LAYER,
            ]))
            .push_children(&[orthographic_headlight, orthographic_xray_entity])
            .id();

        CameraControls {
            mode: ProjectionMode::Perspective,
            perspective_camera_entity,
            perspective_xray_entity,
            perspective_headlight,
            orthographic_camera_entity,
            orthographic_xray_entity,
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
            .get_mut(controls.orthographic_camera_entity)
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
        let (mut xray_proj, _) = cameras.get_mut(controls.orthographic_xray_entity).unwrap();
        *xray_proj = proj;
    } else {
        // perspective mode
        let (mut persp_proj, mut persp_transform) =
            cameras.get_mut(controls.perspective_camera_entity).unwrap();
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
