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
    interaction::Selection,
    recency::RecencyRanking,
    site::{
        Change, Delete, DisplayColor, ImportNavGraphs, NameInSite, NameOfSite, NavGraph,
        NavGraphMarker, SaveNavGraphs, SiteID, DEFAULT_NAV_GRAPH_COLORS,
    },
    widgets::{
        inspector::{color_edit},
        AppEvents, Icons, MoveLayerButton, SelectionWidget,
    },
    Autoload, CurrentWorkspace,
};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::egui::{ImageButton, Ui};
use futures_lite::future;

#[cfg(not(target_arch = "wasm32"))]
use rfd::AsyncFileDialog;

#[derive(Resource)]
pub struct NavGraphDisplay {
    pub color: Option<[f32; 4]>,
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

#[derive(SystemParam)]
pub struct NavGraphParams<'w, 's> {
    pub ranking: Query<'w, 's, &'static RecencyRanking<NavGraphMarker>>,
    pub graphs: Query<
        'w,
        's,
        (
            &'static NameInSite,
            &'static DisplayColor,
            &'static Visibility,
            Option<&'static SiteID>,
        ),
        With<NavGraphMarker>,
    >,
    pub icons: Res<'w, Icons>,
    pub selection: Res<'w, Selection>,
}

pub struct ViewNavGraphs<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a NavGraphParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewNavGraphs<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a NavGraphParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(self, ui: &mut Ui, open_sites: &Query<Entity, With<NameOfSite>>) {
        let ranking = match self.events.request.current_workspace.root {
            Some(c) => match self.params.ranking.get(c) {
                Ok(r) => r,
                Err(_) => return,
            },
            None => return,
        };
        let graph_count = ranking.len();

        ui.horizontal(|ui| {
            if self.events.display.nav_graph.removing {
                if ui
                    .button("View")
                    .on_hover_text("Toggle visibility of graphs")
                    .clicked()
                {
                    self.events.display.nav_graph.removing = false;
                }
                ui.label("Remove");
            } else {
                ui.label("View");
                if ui
                    .button("Remove")
                    .on_hover_text("Choose a graph to remove")
                    .clicked()
                {
                    self.events.display.nav_graph.removing = true;
                }
            }
        });

        ui.horizontal(|ui| {
            let add = ui.button("Add").clicked();
            if self.events.display.nav_graph.color.is_none() {
                let next_color_index = graph_count % DEFAULT_NAV_GRAPH_COLORS.len();
                self.events.display.nav_graph.color =
                    Some(DEFAULT_NAV_GRAPH_COLORS[next_color_index]);
            }
            if let Some(color) = &mut self.events.display.nav_graph.color {
                color_edit(ui, color);
            }
            ui.text_edit_singleline(&mut self.events.display.nav_graph.name);
            if add {
                self.events
                    .commands
                    .spawn(SpatialBundle::default())
                    .insert(NavGraph {
                        name: NameInSite(self.events.display.nav_graph.name.clone()),
                        color: DisplayColor(self.events.display.nav_graph.color.unwrap().clone()),
                        marker: Default::default(),
                    });
                self.events.display.nav_graph.color = None;
            }
        });

        let mut selected_graph = None;
        for e in ranking.iter().rev() {
            let e = *e;
            if self.params.selection.0.is_some_and(|sel| sel == e) {
                selected_graph = Some(e);
            }
            let (name, color, vis, site_id) = match self.params.graphs.get(e) {
                Ok(g) => g,
                Err(_) => continue,
            };
            ui.horizontal(|ui| {
                if self.events.display.nav_graph.removing {
                    if ui
                        .add(ImageButton::new(self.params.icons.trash.egui()))
                        .clicked()
                    {
                        self.events.request.delete.send(Delete::new(e));
                        self.events.display.nav_graph.removing = false;
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
                        self.events
                            .change
                            .visibility
                            .send(Change::new(visibility, e));
                    }
                }

                SelectionWidget::new(e, site_id.copied(), &self.params.icons, self.events).show(ui);

                let mut new_color = color.0;
                color_edit(ui, &mut new_color);
                if new_color != color.0 {
                    self.events
                        .change
                        .color
                        .send(Change::new(DisplayColor(new_color), e));
                }

                let mut new_name = name.0.clone();
                if ui.text_edit_singleline(&mut new_name).changed() {
                    self.events
                        .change
                        .name
                        .send(Change::new(NameInSite(new_name), e));
                }
            });
        }

        if let Some(e) = selected_graph {
            ui.horizontal(|ui| {
                MoveLayerButton::to_top(e, &mut self.events.layers.nav_graphs, &self.params.icons)
                    .show(ui);

                MoveLayerButton::up(e, &mut self.events.layers.nav_graphs, &self.params.icons)
                    .show(ui);

                MoveLayerButton::down(e, &mut self.events.layers.nav_graphs, &self.params.icons)
                    .show(ui);

                MoveLayerButton::to_bottom(
                    e,
                    &mut self.events.layers.nav_graphs,
                    &self.params.icons,
                )
                .show(ui);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.separator();
            if ui.button("Import Graphs...").clicked() {
                match self.events.request.current_workspace.to_site(open_sites) {
                    Some(into_site) => {
                        match &self.events.display.nav_graph.choosing_file_to_import {
                            Some(_) => {
                                warn!("A file is already being chosen!");
                            }
                            None => {
                                let future = AsyncComputeTaskPool::get().spawn(async move {
                                    let file = match AsyncFileDialog::new().pick_file().await {
                                        Some(file) => file,
                                        None => return None,
                                    };

                                    match rmf_site_format::Site::from_bytes(&file.read().await) {
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
                                self.events.display.nav_graph.choosing_file_to_import =
                                    Some(future);
                            }
                        }
                    }
                    None => {
                        error!("No current site??");
                    }
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(export_file) = &self.events.display.nav_graph.export_file {
                    if ui.button("Export").clicked() {
                        if let Some(current_site) =
                            self.events.request.current_workspace.to_site(open_sites)
                        {
                            self.events.request.save_nav_graphs.send(SaveNavGraphs {
                                site: current_site,
                                to_file: export_file.clone(),
                            })
                        } else {
                            error!("No current site??");
                        }
                    }
                }
                if ui.button("Export Graphs As...").clicked() {
                    match &self.events.display.nav_graph.choosing_file_for_export {
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
                            self.events.display.nav_graph.choosing_file_for_export = Some(future);
                        }
                    }
                }
            });
            if let Some(export_file) = &self.events.display.nav_graph.export_file {
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
