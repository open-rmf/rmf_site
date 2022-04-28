use bevy::{
    app::AppExit,
    prelude::*,
    render::camera::{ActiveCamera, Camera3d},
    tasks::AsyncComputeTaskPool,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use super::camera_controls::{CameraControls, ProjectionMode};
use super::site_map::SpawnSiteMapYaml;

// todo: use asset-server or something more sophisticated eventually.
// for now, just hack it up and toss the office-demo YAML into a big string
use super::demo_world::demo_office;

#[cfg(not(target_arch = "wasm32"))]
use {
    bevy::tasks::Task,
    futures_lite::future,
    rfd::AsyncFileDialog,
};

pub struct VisibleWindows {
    pub welcome: bool,
}

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<&mut CameraControls>,
    mut _commands: Commands,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut _exit: EventWriter<AppExit>,
    _thread_pool: Res<AsyncComputeTaskPool>,
    mut visible_windows: ResMut<VisibleWindows>,
    mut spawn_yaml_writer: EventWriter<SpawnSiteMapYaml>,
) {
    let mut controls = query.single_mut();
    egui::TopBottomPanel::top("top").show(egui_context.ctx_mut(), |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button("Return to main menu").clicked() {
                    visible_windows.welcome = true;
                }
                ui.separator();
                if ui
                    .add(egui::SelectableLabel::new(
                        controls.mode == ProjectionMode::Orthographic,
                        "2D",
                    ))
                    .clicked()
                {
                    controls.set_mode(ProjectionMode::Orthographic);
                    active_camera_3d.set(controls.orthographic_camera_entity);
                }
                if ui
                    .add(egui::SelectableLabel::new(
                        controls.mode == ProjectionMode::Perspective,
                        "3D",
                    ))
                    .clicked()
                {
                    controls.set_mode(ProjectionMode::Perspective);
                    active_camera_3d.set(controls.perspective_camera_entity);
                }
            });
        });
    });

    egui::TopBottomPanel::bottom("bottom")
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                if visible_windows.welcome {
                    ui.label("Welcome!");
                }
                else if controls.mode == ProjectionMode::Orthographic {
                    ui.label("Left-drag: pan. Scroll wheel: zoom.");
                }
                else if controls.mode == ProjectionMode::Perspective {
                    ui.label("Left-drag: pan. Right-drag: rotate. Scroll wheel: zoom.");
                }
            });
        });

    if visible_windows.welcome {
        egui::Window::new("Welcome!")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
            .show(egui_context.ctx_mut(), |ui| {
                ui.heading("Welcome to The RMF Sandbox!");
                ui.add_space(10.);

                if ui.button("Open a demonstration map").clicked() {
                    // load the office demo that is hard-coded in demo_world.rs
                    let result: serde_yaml::Result<serde_yaml::Value> = serde_yaml::from_str(&demo_office());
                    if result.is_err() {
                        println!("serde threw an error: {:?}", result.err());
                    }
                    else {
                        let doc: serde_yaml::Value = serde_yaml::from_str(&demo_office()).ok().unwrap();
                        spawn_yaml_writer.send(SpawnSiteMapYaml { yaml_doc: doc });
                    }
                    visible_windows.welcome = false;
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("Open a local map file").clicked() {
                        let future = _thread_pool.spawn(async move {
                            let file = AsyncFileDialog::new().pick_file().await;
                            let data = match file {
                                Some(f) => Some(f.read().await),
                                None => None
                            };
                            data
                        });
                        _commands.spawn().insert(future);
                        visible_windows.welcome = false;
                    }
                    ui.add_space(10.);
                    if ui.button("Quit").clicked() {
                        _exit.send(AppExit);
                    }
                }

                /*
                if ui.button("Close").clicked() {
                  visible_windows.welcome = false;
                }
                */
            });
    }
}

/// Handles the file opening events
#[cfg(not(target_arch = "wasm32"))]
fn handle_file_open(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut Task<Option<Vec<u8>>>)>,
    mut spawn_yaml_writer: EventWriter<SpawnSiteMapYaml>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut *task)) {
            match result {
                Some(result) => {
                    // success! Now, try to parse this file as YAML and spawn
                    let yaml_result: serde_yaml::Result<serde_yaml::Value> =
                        serde_yaml::from_slice(&result);
                    match yaml_result {
                        Ok(doc) => spawn_yaml_writer.send(SpawnSiteMapYaml { yaml_doc: doc }),
                        Err(e) => println!("error parsing file: {:?}", e),
                    }
                },
                None => {}
            }
            commands.entity(entity).remove::<Task<Option<Vec<u8>>>>();
        }
    }
}

pub struct UIWidgetsPlugin;

impl Plugin for UIWidgetsPlugin {
    fn build(&self, app: &mut App) {
        // avoid conflict with bevy-inspect-egui
        if !app.world.contains_resource::<EguiContext>() {
            app.add_plugin(EguiPlugin);
        }
        app.add_system(egui_ui);
        app.insert_resource(VisibleWindows { welcome: true });

        #[cfg(not(target_arch = "wasm32"))]
        app.add_system(handle_file_open);
    }
}
