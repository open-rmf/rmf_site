use super::demo_world::demo_office;
use super::site_map::SiteMap;
use bevy::{app::AppExit, prelude::*, tasks::AsyncComputeTaskPool};
use bevy_egui::{egui, EguiContext};

use crate::building_map::BuildingMap;
use crate::AppState;

#[cfg(not(target_arch = "wasm32"))]
use {bevy::tasks::Task, futures_lite::future, rfd::AsyncFileDialog};

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut _commands: Commands,
    mut _exit: EventWriter<AppExit>,
    _thread_pool: Res<AsyncComputeTaskPool>,
    mut app_state: ResMut<State<AppState>>,
) {
    egui::Window::new("Welcome!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(egui_context.ctx_mut(), |ui| {
            ui.heading("Welcome to The RMF Sandbox!");
            ui.add_space(10.);

            ui.horizontal(|ui| {
                if ui.button("View demo map").clicked() {
                    // load the office demo that is hard-coded in demo_world.rs
                    let future = _thread_pool.spawn(async move {
                        println!("Loading site map");
                        let yaml = demo_office();
                        let data = yaml.as_bytes();
                        match BuildingMap::from_bytes(&data) {
                            Ok(map) => Some(SiteMap::from_building_map(map)),
                            Err(err) => {
                                println!("{:?}", err);
                                return None;
                            }
                        }
                    });
                    _commands.spawn().insert(future);
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("Open a map file").clicked() {
                        // load the map in a thread pool
                        let future = _thread_pool.spawn(async move {
                            let file = match AsyncFileDialog::new().pick_file().await {
                                Some(file) => file,
                                None => {
                                    println!("No file selected");
                                    return None;
                                }
                            };
                            println!("Loading site map");
                            let data = file.read().await;
                            match BuildingMap::from_bytes(&data) {
                                Ok(map) => Some(SiteMap::from_building_map(map)),
                                Err(err) => {
                                    println!("{:?}", err);
                                    return None;
                                }
                            }
                        });

                        // FIXME: This is from the bevy example, but not sure if this will leak entites.
                        // The task completion handler only removes the task component from the
                        // entity but never removes the entity itself.
                        _commands.spawn().insert(future);
                    }
                }

                if ui.button("Use building generator").clicked() {
                    println!("Entering warehouse generator");
                    app_state.set(AppState::WarehouseGenerator).unwrap();
                }
            });

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(20.);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        if ui.button("Exit").clicked() {
                            _exit.send(AppExit);
                        }
                    });
                });
            }
        });
}

/// Handles the file opening events
#[cfg(not(target_arch = "wasm32"))]
fn map_load_complete(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut Task<Option<SiteMap>>)>,
    mut app_state: ResMut<State<AppState>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut *task)) {
            println!("Site map loaded");
            // FIXME: Do we need to remove this entity and not just the component to avoid leaks?
            commands.entity(entity).remove::<Task<Option<SiteMap>>>();

            match result {
                Some(result) => {
                    println!("Entering traffic editor");
                    commands.insert_resource(result);
                    match app_state.set(AppState::TrafficEditor) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("Failed to enter traffic editor: {:?}", err);
                        }
                    }
                }
                None => {}
            }
        }
    }
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_update(AppState::MainMenu).with_system(egui_ui));

        #[cfg(not(target_arch = "wasm32"))]
        app.add_system_set(SystemSet::on_update(AppState::MainMenu).with_system(map_load_complete));
    }
}
