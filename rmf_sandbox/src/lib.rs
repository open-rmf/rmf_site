use bevy::{
    app::AppExit,
    // diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::{
        mouse::{MouseButton, MouseWheel},
    },
    pbr::{
        DirectionalLight,
        DirectionalLightShadowMap,
    },
    prelude::*,
    render::{
        camera::{ActiveCamera, Camera3d, ScalingMode, WindowOrigin},
    },
};
use wasm_bindgen::prelude::*;

// a few more imports needed for wasm32 only
#[cfg(target_arch = "wasm32")]
use bevy::{
    core::{
        FixedTimestep,
        //Time
    },
    window::{Windows},
};

extern crate web_sys;
mod demo_world;

use bevy_egui::{egui, EguiContext, EguiPlugin};

mod site_map;
use site_map::{SiteMap, SiteMapPlugin};

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

#[derive(PartialEq, Debug, Clone, Reflect)]
pub enum ProjectionMode { Perspective, Orthographic }

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
    pub fn set_mode(&mut self, mode: ProjectionMode) { //, &mut active_camera_3d: ActiveCamera<Camera3d>) {

        self.mode = mode;
        if self.mode == ProjectionMode::Orthographic {
            // active_camera_3d.set(self.orthographic_camera_entity);
        }
        else if self.mode == ProjectionMode::Perspective {
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
    mut ortho_query: Query<(&mut OrthographicProjection, &mut Transform), Without<PerspectiveProjection>>,
    mut persp_query: Query<(&mut PerspectiveProjection, &mut Transform), Without<OrthographicProjection>>,
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
            scroll += 4. * ev.y / ev.y.abs();
        }
    }

    /*
    if scroll.abs() > 0.0 {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("scroll = {}", scroll).into());
        //println!("scroll = {}", scroll);
    }
    */

    #[cfg(target_arch = "wasm32")]
    {
      scroll = scroll * 0.1; // not sure why, but web scrolling seems SO FAST
    }

    let mut controls = controls_query.single_mut();

    /*
    if proj.mode_switched {
        proj.mode_switched = false;
        transform.translation = initial_position.position.clone();
        transform.rotation = Quat::default();
        /*
        if proj.mode == ProjectionMode::Perspective {
            transform.translation.z = proj.orbit_radius;
            println!("set transform translation to {}", transform.translation);
        }
        */
    }
    */

    if controls.mode == ProjectionMode::Orthographic {
        let (mut ortho_proj, mut ortho_transform) = ortho_query.single_mut();

        let window = windows.get_primary().unwrap();
        let window_size = Vec2::new(
            window.width() as f32,
            window.height() as f32);
        let aspect_ratio = window_size[0] / window_size[1];

        if cursor_motion.length_squared() > 0.0 {
            cursor_motion *= 2. / window_size * Vec2::new(
                ortho_proj.scale * aspect_ratio,
                ortho_proj.scale
            );
            let right = -cursor_motion.x * Vec3::X;
            let up = -cursor_motion.y * Vec3::Y;
            ortho_transform.translation += right + up;
        }
        if scroll.abs() > 0.0 {
            ortho_proj.scale -= scroll * ortho_proj.scale * 0.1;
            ortho_proj.scale = f32::max(ortho_proj.scale, 0.02);
        }
    }
    else {
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
                if controls.orbit_upside_down { -delta } else { delta }
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
                Vec2::new(
                    persp_proj.fov * persp_proj.aspect_ratio,
                    persp_proj.fov
                ) / window_size;
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
            persp_transform.translation =
                controls.orbit_center
                + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, controls.orbit_radius));
        }
    }
}

fn egui_ui(
    mut sm: ResMut<SiteMap>,
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<&mut CameraControls>,
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut exit: EventWriter<AppExit>,
) {
    let mut controls = query.single_mut();
    egui::TopBottomPanel::top("top_panel")
        .show(egui_context.ctx_mut(), |ui| {
            ui.vertical(|ui| {

                egui::menu::bar(ui, |ui| {
                    egui::menu::menu_button(ui, "File", |ui| {
                        if ui.button("Load demo").clicked() {
                            sm.load_demo();
                            sm.spawn(commands, meshes, materials, asset_server);
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        if ui.button("Quit").clicked() {
                            exit.send(AppExit);
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label("[toolbar buttons]");
                    ui.separator();
                    if ui.add(egui::SelectableLabel::new(controls.mode == ProjectionMode::Orthographic, "2D")).clicked() {
                        controls.set_mode(ProjectionMode::Orthographic);
                        active_camera_3d.set(controls.orthographic_camera_entity);
                    }
                    if ui.add(egui::SelectableLabel::new(controls.mode == ProjectionMode::Perspective, "3D")).clicked() {
                        controls.set_mode(ProjectionMode::Perspective);
                        active_camera_3d.set(controls.perspective_camera_entity);
                    }
                });
            });
        });
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("entering setup() startup system...");

    /*
    // this is useful for debugging lighting... spheres are very forgiving
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: 20.,
            ..Default::default()
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::LIME_GREEN,
            ..Default::default()
        }),
        transform: Transform::from_xyz(0., 0., 0.),
        ..Default::default()
    });
    */

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
        //material: materials.add(Color::rgb(0.3, 0.7, 0.3).into()),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.3, 0.3, 0.3),
            ..Default::default()
        }),
        transform: Transform {
            rotation: Quat::from_rotation_x(1.57),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.001,
    });

    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            illuminance: 20000.,
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0., 0., 50.),
            rotation: Quat::from_rotation_x(0.4),
            ..Default::default()
        },
        ..Default::default()
    });

    let proj_entity = commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }).id();

    let ortho_entity = commands.spawn_bundle(OrthographicCameraBundle {
        transform: Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
        orthographic_projection: OrthographicProjection {
            window_origin: WindowOrigin::Center,
            scaling_mode: ScalingMode::FixedVertical,
            scale: 10.0,
            ..default()
        },
        ..OrthographicCameraBundle::new_3d()
    }).id();

    commands.spawn_bundle(CameraControlsBundle {
        controls: CameraControls {
            mode: ProjectionMode::Perspective,
            perspective_camera_entity: proj_entity,
            orthographic_camera_entity: ortho_entity,
            orbit_center: Vec3::ZERO,
            orbit_radius: 20.0,
            orbit_upside_down: false,
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn check_browser_window_size(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    let wasm_window = web_sys::window().unwrap();
    let target_width = wasm_window.inner_width().unwrap().as_f64().unwrap() as f32;
    let target_height = wasm_window.inner_height().unwrap().as_f64().unwrap() as f32;
    let w_diff = (target_width - window.width()).abs();
    let h_diff = (target_height - window.height()).abs();

    if w_diff > 3. || h_diff > 3. {
        // web_sys::console::log_1(&format!("window = {} {} canvas = {} {}", window.width(), window.height(), target_width, target_height).into());
        window.set_resolution(target_width, target_height);
    }
}

#[wasm_bindgen]
pub fn run() {

    #[cfg(target_arch = "wasm32")]
    App::new()
        .insert_resource(WindowDescriptor {
            title: "RMF Sandbox".to_string(),
            canvas: Some(String::from("#rmf_sandbox_canvas")),
            //vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource( DirectionalLightShadowMap {
            size: 1024
        })
        //.add_plugin(SuperCameraPlugin)
        .add_startup_system(setup)
        .add_plugin(SiteMapPlugin)
        .add_system(handle_keyboard)
        .add_plugin(EguiPlugin)
        .add_system(egui_ui)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.5))
                .with_system(check_browser_window_size)
            )
        .run();

    #[cfg(not(target_arch = "wasm32"))]
    App::new()
        .insert_resource(WindowDescriptor {
            title: "RMF Sandbox".to_string(),
            width: 800.,
            height: 800.,
            //vsync: false,
            ..Default::default()
        })
        .insert_resource( DirectionalLightShadowMap {
            size: 2048
        })
        .insert_resource(MouseLocation::default())
        .add_plugins(DefaultPlugins)
        //.add_plugin(FrameTimeDiagnosticsPlugin::default())
        //.add_plugin(LogDiagnosticsPlugin::default())
        //.insert_resource(Msaa { samples: 4})
        //.add_plugin(SuperCameraPlugin)
        .add_startup_system(setup)
        .add_plugin(SiteMapPlugin)
        .add_system(handle_keyboard)
        .add_plugin(EguiPlugin)
        .add_system(camera_controls)
        .add_system(egui_ui)
        .run();
}
