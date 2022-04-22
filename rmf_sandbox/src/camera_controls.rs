use bevy::{
    input::mouse::{MouseButton, MouseWheel},
    prelude::*,
    render::camera::{ActiveCamera, Camera3d, ScalingMode, WindowOrigin},
};

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

#[derive(PartialEq, Debug, Clone, Reflect)]
pub enum ProjectionMode {
    Perspective,
    Orthographic,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct CameraControls {
    pub mode: ProjectionMode,
    pub perspective_camera_entity: Entity,
    pub orthographic_camera_entity: Entity,
    pub orbit_center: Vec3,
    pub orbit_radius: f32,
    pub orbit_upside_down: bool,
}

impl CameraControls {
    pub fn set_mode(&mut self, mode: ProjectionMode) {
        //, &mut active_camera_3d: ActiveCamera<Camera3d>) {

        self.mode = mode;
        if self.mode == ProjectionMode::Orthographic {
            // active_camera_3d.set(self.orthographic_camera_entity);
        } else if self.mode == ProjectionMode::Perspective {
            // active_camera_3d.set(self.perspective_camera_entity);
        }
    }
}

#[derive(Bundle)]
pub struct CameraControlsBundle {
    pub controls: CameraControls,
}

fn camera_controls(
    windows: Res<Windows>,
    mut ev_cursor_moved: EventReader<CursorMoved>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut previous_mouse_location: ResMut<MouseLocation>,
    mut controls_query: Query<&mut CameraControls>,
    mut ortho_query: Query<
        (&mut OrthographicProjection, &mut Transform),
        Without<PerspectiveProjection>,
    >,
    mut persp_query: Query<
        (&mut PerspectiveProjection, &mut Transform),
        Without<OrthographicProjection>,
    >,
) {
    let pan_button = MouseButton::Left;
    let orbit_button = MouseButton::Right;

    // spin through all mouse cursor-moved events to find the last one
    let mut last_pos = previous_mouse_location.previous;
    for ev in ev_cursor_moved.iter() {
        last_pos.x = ev.position.x;
        last_pos.y = ev.position.y;
    }

    let mut cursor_motion = Vec2::ZERO;
    if input_mouse.pressed(pan_button) || input_mouse.pressed(orbit_button) {
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

    let mut controls = controls_query.single_mut();

    if controls.mode == ProjectionMode::Orthographic {
        let (mut ortho_proj, mut ortho_transform) = ortho_query.single_mut();

        let window = windows.get_primary().unwrap();
        let window_size = Vec2::new(window.width() as f32, window.height() as f32);
        let aspect_ratio = window_size[0] / window_size[1];

        if cursor_motion.length_squared() > 0.0 {
            cursor_motion *=
                2. / window_size * Vec2::new(ortho_proj.scale * aspect_ratio, ortho_proj.scale);
            let right = -cursor_motion.x * Vec3::X;
            let up = -cursor_motion.y * Vec3::Y;
            ortho_transform.translation += right + up;
        }
        if scroll.abs() > 0.0 {
            ortho_proj.scale -= scroll * ortho_proj.scale * 0.1;
            ortho_proj.scale = f32::max(ortho_proj.scale, 0.02);
        }
    } else {
        // perspective mode
        let (persp_proj, mut persp_transform) = persp_query.single_mut();

        let mut changed = false;

        if input_mouse.just_released(orbit_button) || input_mouse.just_pressed(orbit_button) {
            // only check for upside down when orbiting started or ended this frame
            // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
            let up = persp_transform.rotation * Vec3::Z;
            controls.orbit_upside_down = up.z <= 0.0;
        }

        if input_mouse.pressed(orbit_button) && cursor_motion.length_squared() > 0. {
            changed = true;
            let window = windows.get_primary().unwrap();
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
            persp_transform.rotation = persp_transform.rotation * pitch; // local x
        } else if input_mouse.pressed(MouseButton::Left) && cursor_motion.length_squared() > 0. {
            changed = true;
            // make panning distance independent of resolution and FOV,
            let window = windows.get_primary().unwrap();
            let window_size = Vec2::new(window.width() as f32, window.height() as f32);

            cursor_motion *=
                Vec2::new(persp_proj.fov * persp_proj.aspect_ratio, persp_proj.fov) / window_size;
            // translate by local axes
            let right = persp_transform.rotation * Vec3::X * -cursor_motion.x;
            let up = persp_transform.rotation * Vec3::Y * -cursor_motion.y;
            // make panning proportional to distance away from center point
            let translation = (right + up) * controls.orbit_radius;
            controls.orbit_center += translation;
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

fn handle_keyboard(
    keyboard_input: Res<Input<KeyCode>>,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut query: Query<&mut CameraControls>,
) {
    let mut controls = query.single_mut();
    if keyboard_input.just_pressed(KeyCode::Key2) {
        controls.set_mode(ProjectionMode::Orthographic);
        active_camera_3d.set(controls.orthographic_camera_entity);
    }

    if keyboard_input.just_pressed(KeyCode::Key3) {
        controls.set_mode(ProjectionMode::Perspective);
        active_camera_3d.set(controls.perspective_camera_entity);
    }
}

fn camera_controls_setup(mut commands: Commands) {
    let proj_entity = commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .id();

    let ortho_entity = commands
        .spawn_bundle(OrthographicCameraBundle {
            transform: Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
            orthographic_projection: OrthographicProjection {
                window_origin: WindowOrigin::Center,
                scaling_mode: ScalingMode::FixedVertical,
                scale: 10.0,
                ..default()
            },
            ..OrthographicCameraBundle::new_3d()
        })
        .id();
    commands.spawn_bundle(CameraControlsBundle {
        controls: CameraControls {
            mode: ProjectionMode::Perspective,
            perspective_camera_entity: proj_entity,
            orthographic_camera_entity: ortho_entity,
            orbit_center: Vec3::ZERO,
            orbit_radius: 20.0,
            orbit_upside_down: false,
        },
    });
}

pub struct CameraControlsPlugin;

impl Plugin for CameraControlsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MouseLocation::default())
            .add_startup_system(camera_controls_setup)
            .add_system(handle_keyboard)
            .add_system(camera_controls);
    }
}
