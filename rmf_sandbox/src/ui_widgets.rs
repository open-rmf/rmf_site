use bevy::{
    app::AppExit,
    prelude::*,
    render::{
        camera::{ActiveCamera, Camera3d},
    },
    tasks::{AsyncComputeTaskPool}
};

#[cfg(not(target_arch = "wasm32"))]
use bevy::{
    tasks::Task
};

use bevy_egui::{egui, EguiContext, EguiPlugin};
use super::camera_controls::{CameraControls, ProjectionMode};
use super::site_map::{SiteMap};
use rfd::AsyncFileDialog;
use futures_lite::future;

fn egui_ui(
    mut sm: ResMut<SiteMap>,
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<&mut CameraControls>,
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut active_camera_3d: ResMut<ActiveCamera<Camera3d>>,
    mut _exit: EventWriter<AppExit>,
    thread_pool: Res<AsyncComputeTaskPool>,
    mesh_query: Query<(Entity, &Handle<Mesh>)>,
) {
    let mut controls = query.single_mut();
    egui::TopBottomPanel::top("top_panel")
        .show(egui_context.ctx_mut(), |ui| {
            ui.vertical(|ui| {

                egui::menu::bar(ui, |ui| {
                    egui::menu::menu_button(ui, "File", |ui| {
                        if ui.button("Load demo").clicked() {
                            sm.clear();
                            sm.load_demo();
                            sm.spawn(commands, meshes, materials, asset_server, mesh_query);
                        }
                        else {
                            // menu commands that only make sense for non-web builds:
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if ui.button("Open...").clicked() {
                                    let future = thread_pool.spawn(async move {
                                        let file = AsyncFileDialog::new().pick_file().await;
                                        let data = match file {
                                            Some(data) => Some(data.read().await),
                                            None => None
                                        };
                                        data
                                    });
                                    commands.spawn().insert(future);
                                }
                                if ui.button("Quit").clicked() {
                                    _exit.send(AppExit);
                                }
                            }
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

/// Handles the file opening events
#[cfg(not(target_arch = "wasm32"))]
fn handle_file_open(
    mut sm: ResMut<SiteMap>,
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _asset_server: Res<AssetServer>,
    mut tasks: Query<(Entity, &mut Task<Option<Vec<u8>>>)>,
    _mesh_query: Query<(Entity, &Handle<Mesh>)>,
) {

    let mut assets_changed = false;
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut *task)) {
            match result {
                Some(result) => {
                    sm.clear();
                    sm.load_yaml_from_data(&result);
                    assets_changed =true;
                    commands.entity(entity).remove::<Task<Option<Vec<u8>>>>();
                },
                None => {}
            }
        }
    }

    if assets_changed {
        sm.spawn(commands, _meshes, _materials, _asset_server, _mesh_query);
    }
}

pub struct UIWidgetsPlugin;

impl Plugin for UIWidgetsPlugin{
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
           .add_system(egui_ui);

        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_system(handle_file_open);
        }
    }
}
