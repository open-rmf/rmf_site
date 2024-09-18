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

use crate::{
    recency::RecencyRanking,
    site::{
        Change, Delete, DisplayColor, ImportNavGraphs, NameInSite, NameOfSite, NavGraph,
        NavGraphMarker, SaveNavGraphs, DEFAULT_NAV_GRAPH_COLORS,
    },
    widgets::{inspector::color_edit, prelude::*, Icons, MoveLayerButton, SelectorWidget},
    AppState, Autoload, ChangeRank, CurrentWorkspace,
};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::egui::{CollapsingHeader, ImageButton, Ui};
use futures_lite::future;

#[cfg(not(target_arch = "wasm32"))]
use rfd::AsyncFileDialog;

/// Add a widget for viewing and editing navigation graphs.
#[derive(Default)]
pub struct ViewNavGraphsPlugin {}

impl Plugin for ViewNavGraphsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavGraphDisplay>()
            .add_plugins(PropertiesTilePlugin::<ViewNavGraphs>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewNavGraphs<'w, 's> {
    ranking: Query<'w, 's, &'static RecencyRanking<NavGraphMarker>>,
    graphs: Query<
        'w,
        's,
        (
            &'static NameInSite,
            &'static DisplayColor,
            &'static Visibility,
        ),
        With<NavGraphMarker>,
    >,
    icons: Res<'w, Icons>,
    open_sites: Query<'w, 's, Entity, With<NameOfSite>>,
    current_workspace: ResMut<'w, CurrentWorkspace>,
    display_nav_graph: ResMut<'w, NavGraphDisplay>,
    delete: EventWriter<'w, Delete>,
    change_visibility: EventWriter<'w, Change<Visibility>>,
    change_name: EventWriter<'w, Change<NameInSite>>,
    change_color: EventWriter<'w, Change<DisplayColor>>,
    change_rank: EventWriter<'w, ChangeRank<NavGraphMarker>>,
    save_nav_graphs: EventWriter<'w, SaveNavGraphs>,
    selector: SelectorWidget<'w, 's>,
    commands: Commands<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewNavGraphs<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        CollapsingHeader::new("Navigation")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewNavGraphs<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let ranking = match self.current_workspace.root {
            Some(c) => match self.ranking.get(c) {
                Ok(r) => r,
                Err(_) => return,
            },
            None => return,
        };
        let graph_count = ranking.len();

        ui.horizontal(|ui| {
            if self.display_nav_graph.removing {
                if ui
                    .button("View")
                    .on_hover_text("Toggle visibility of graphs")
                    .clicked()
                {
                    self.display_nav_graph.removing = false;
                }
                ui.label("Remove");
            } else {
                ui.label("View");
                if ui
                    .button("Remove")
                    .on_hover_text("Choose a graph to remove")
                    .clicked()
                {
                    self.display_nav_graph.removing = true;
                }
            }
        });

        ui.horizontal(|ui| {
            let add = ui.button("Add").clicked();
            if self.display_nav_graph.color.is_none() {
                let next_color_index = graph_count % DEFAULT_NAV_GRAPH_COLORS.len();
                self.display_nav_graph.color = Some(DEFAULT_NAV_GRAPH_COLORS[next_color_index]);
            }
            if let Some(color) = &mut self.display_nav_graph.color {
                color_edit(ui, color);
            }
            ui.text_edit_singleline(&mut self.display_nav_graph.name);
            if add {
                self.commands
                    .spawn(SpatialBundle::default())
                    .insert(NavGraph {
                        name: NameInSite(self.display_nav_graph.name.clone()),
                        color: DisplayColor(self.display_nav_graph.color.unwrap().clone()),
                        marker: Default::default(),
                    });
                self.display_nav_graph.color = None;
            }
        });

        let mut selected_graph = None;
        for e in ranking.iter().rev() {
            let e = *e;
            if self.selector.selection.0.is_some_and(|sel| sel == e) {
                selected_graph = Some(e);
            }
            let (name, color, vis) = match self.graphs.get(e) {
                Ok(g) => g,
                Err(_) => continue,
            };
            ui.horizontal(|ui| {
                if self.display_nav_graph.removing {
                    if ui.add(ImageButton::new(self.icons.trash.egui())).clicked() {
                        self.delete.send(Delete::new(e));
                        self.display_nav_graph.removing = false;
                    }
                } else {
                    let mut is_visible = !matches!(vis, Visibility::Hidden);
                    if ui
                        .checkbox(&mut is_visible, "")
                        .on_hover_text(if is_visible {
                            "Make this graph invisible"
                        } else {
                            "Make this graph visible"
                        })
                        .changed()
                    {
                        let visibility = if is_visible {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        };
                        self.change_visibility.send(Change::new(visibility, e));
                    }
                }

                self.selector.show_widget(e, ui);

                let mut new_color = color.0;
                color_edit(ui, &mut new_color);
                if new_color != color.0 {
                    self.change_color
                        .send(Change::new(DisplayColor(new_color), e));
                }

                let mut new_name = name.0.clone();
                if ui.text_edit_singleline(&mut new_name).changed() {
                    self.change_name.send(Change::new(NameInSite(new_name), e));
                }
            });
        }

        if let Some(e) = selected_graph {
            ui.horizontal(|ui| {
                MoveLayerButton::to_top(e, &mut self.change_rank, &self.icons).show(ui);
                MoveLayerButton::up(e, &mut self.change_rank, &self.icons).show(ui);
                MoveLayerButton::down(e, &mut self.change_rank, &self.icons).show(ui);
                MoveLayerButton::to_bottom(e, &mut self.change_rank, &self.icons).show(ui);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.separator();
            if ui.button("Import Graphs...").clicked() {
                match self.current_workspace.to_site(&self.open_sites) {
                    Some(into_site) => match &self.display_nav_graph.choosing_file_to_import {
                        Some(_) => {
                            warn!("A file is already being chosen!");
                        }
                        None => {
                            let future = AsyncComputeTaskPool::get().spawn(async move {
                                let file = match AsyncFileDialog::new().pick_file().await {
                                    Some(file) => file,
                                    None => return None,
                                };

                                match rmf_site_format::Site::from_bytes_ron(&file.read().await) {
                                    Ok(from_site) => Some((
                                        file.path().to_owned(),
                                        ImportNavGraphs {
                                            into_site,
                                            from_site,
                                        },
                                    )),
                                    Err(err) => {
                                        error!("Unable to parse file:\n{err}");
                                        None
                                    }
                                }
                            });
                            self.display_nav_graph.choosing_file_to_import = Some(future);
                        }
                    },
                    None => {
                        error!("No current site??");
                    }
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(export_file) = &self.display_nav_graph.export_file {
                    if ui.button("Export").clicked() {
                        if let Some(current_site) = self.current_workspace.to_site(&self.open_sites)
                        {
                            self.save_nav_graphs.send(SaveNavGraphs {
                                site: current_site,
                                to_file: export_file.clone(),
                            });
                        } else {
                            error!("No current site??");
                        }
                    }
                }
                if ui.button("Export Graphs As...").clicked() {
                    match &self.display_nav_graph.choosing_file_for_export {
                        Some(_) => {
                            warn!("A file is already being chosen!");
                        }
                        None => {
                            let future = AsyncComputeTaskPool::get().spawn(async move {
                                let file = match AsyncFileDialog::new().save_file().await {
                                    Some(file) => file,
                                    None => return None,
                                };
                                Some(file.path().to_path_buf())
                            });
                            self.display_nav_graph.choosing_file_for_export = Some(future);
                        }
                    }
                }
            });
            if let Some(export_file) = &self.display_nav_graph.export_file {
                if let Some(export_file) = export_file.as_os_str().to_str() {
                    ui.horizontal(|ui| {
                        ui.label("Chosen file:");
                        ui.label(export_file);
                    });
                }
            }
        }
    }
}

#[derive(Resource)]
pub struct NavGraphDisplay {
    pub color: Option<[f32; 3]>,
    pub name: String,
    pub removing: bool,
    pub choosing_file_for_export: Option<Task<Option<std::path::PathBuf>>>,
    pub export_file: Option<std::path::PathBuf>,
    pub choosing_file_to_import: Option<Task<Option<(std::path::PathBuf, ImportNavGraphs)>>>,
}

impl FromWorld for NavGraphDisplay {
    fn from_world(world: &mut World) -> Self {
        let export_file = world
            .get_resource::<Autoload>()
            .map(|a| a.import.clone())
            .flatten();
        Self {
            color: None,
            name: "<Unnamed>".to_string(),
            removing: false,
            choosing_file_for_export: None,
            export_file,
            choosing_file_to_import: None,
        }
    }
}

pub fn resolve_nav_graph_import_export_files(
    mut nav_graph_display: ResMut<NavGraphDisplay>,
    mut save_nav_graphs: EventWriter<SaveNavGraphs>,
    mut import_nav_graphs: EventWriter<ImportNavGraphs>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    if 'resolved: {
        if let Some(task) = &mut nav_graph_display.choosing_file_for_export {
            if let Some(result) = future::block_on(future::poll_once(task)) {
                if let Some(result) = result {
                    if let Some(current_site) = current_workspace.to_site(&open_sites) {
                        save_nav_graphs.send(SaveNavGraphs {
                            site: current_site,
                            to_file: result.clone(),
                        });
                    }
                    nav_graph_display.export_file = Some(result)
                }

                break 'resolved true;
            }
        }
        false
    } {
        nav_graph_display.choosing_file_for_export = None;
    }

    if 'resolved: {
        if let Some(task) = &mut nav_graph_display.choosing_file_to_import {
            if let Some(result) = future::block_on(future::poll_once(task)) {
                if let Some((path, request)) = result {
                    import_nav_graphs.send(request);
                    nav_graph_display.export_file = Some(path);
                }

                break 'resolved true;
            }
        }
        false
    } {
        nav_graph_display.choosing_file_to_import = None;
    }
}
