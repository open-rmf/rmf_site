use bevy::{
    app::AppExit,
    prelude::*,
    render::camera::{ActiveCamera, Camera3d},
    tasks::AsyncComputeTaskPool,
};

#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::Task;

use super::camera_controls::{CameraControls, ProjectionMode};
use super::site_map::SpawnSiteMapYaml;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use futures_lite::future;
use rfd::AsyncFileDialog;

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<&mut CameraControls>,
    mut commands: Commands,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut _exit: EventWriter<AppExit>,
    thread_pool: Res<AsyncComputeTaskPool>,
) {
    let mut controls = query.single_mut();
    egui::TopBottomPanel::top("top_panel").show(egui_context.ctx_mut(), |ui| {
        ui.vertical(|ui| {
            egui::menu::bar(ui, |ui| {
                // File menu commands only make sense for non-web builds:
                #[cfg(not(target_arch = "wasm32"))]
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Open...").clicked() {
                        let future = thread_pool.spawn(async move {
                            let file = AsyncFileDialog::new().pick_file().await;
                            let data = match file {
                                Some(data) => Some(data.read().await),
                                None => None,
                            };
                            data
                        });
                        commands.spawn().insert(future);
                    }
                    if ui.button("Quit").clicked() {
                        _exit.send(AppExit);
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label("[toolbar buttons]");
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
                    commands.entity(entity).remove::<Task<Option<Vec<u8>>>>();
                }
                None => {}
            }
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

        #[cfg(not(target_arch = "wasm32"))]
        app.add_system(handle_file_open);
    }
}
