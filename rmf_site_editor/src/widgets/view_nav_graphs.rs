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
    site::{
        NavGraph, NavGraphMarker, NameInSite, DisplayColor, Change, Delete,
        DEFAULT_NAV_GRAPH_COLORS},
    widgets::{
        inspector::color_edit,
        AppEvents, Icons,
    },
};
use bevy::{prelude::*, ecs::system::SystemParam};
use bevy_egui::egui::{Ui, ImageButton};
use smallvec::SmallVec;

pub struct NavGraphDisplay {
    pub color: Option<[f32; 4]>,
    pub name: String,
    pub removing: bool,
}

impl Default for NavGraphDisplay {
    fn default() -> Self {
        Self {
            color: None,
            name: "<Unnamed>".to_string(),
            removing: false,
        }
    }
}

#[derive(SystemParam)]
pub struct NavGraphParams<'w, 's> {
    pub graphs: Query<'w, 's, (Entity, &'static NameInSite, &'static DisplayColor, &'static Visibility), With<NavGraphMarker>>,
    pub icons: Res<'w, Icons>,
}

pub struct ViewNavGraphs<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a NavGraphParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewNavGraphs<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a NavGraphParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        let graphs = {
            let mut graphs: SmallVec<
                [(Entity, &NameInSite, &DisplayColor, &Visibility); 10]
            > = SmallVec::from_iter(self.params.graphs.iter());
            graphs.sort_by(|(a, _, _, _), (b, _, _, _)| a.cmp(b));
            graphs
        };
        let graph_count = graphs.len();

        ui.horizontal(|ui| {
            if self.events.display.nav_graph.removing {
                if ui.button("View").on_hover_text("Toggle visibility of graphs").clicked() {
                    self.events.display.nav_graph.removing = false;
                }
                ui.label("Remove");
            } else {
                ui.label("View");
                if ui.button("Remove").on_hover_text("Choose a graph to remove").clicked() {
                    self.events.display.nav_graph.removing = true;
                }
            }
        });

        for (e, name, color, vis) in graphs {
            ui.horizontal(|ui| {
                if self.events.display.nav_graph.removing {
                    if ui.add(ImageButton::new(self.params.icons.egui_trash, [18., 18.])).clicked() {
                        self.events.request.delete.send(Delete::new(e));
                        self.events.display.nav_graph.removing = false;
                    }
                } else {
                    let mut is_visible = vis.is_visible;
                    if ui.checkbox(&mut is_visible, "")
                        .on_hover_text(if vis.is_visible {
                            "Make this graph invisible"
                        } else {
                            "Make this graph visible"
                        }).changed()
                    {
                        self.events.change.visibility.send(Change::new(Visibility { is_visible }, e));
                    }
                }

                let mut new_color = color.0;
                color_edit(ui, &mut new_color);
                if new_color != color.0 {
                    self.events.change.color.send(Change::new(DisplayColor(new_color), e));
                }

                let mut new_name = name.0.clone();
                if ui.text_edit_singleline(&mut new_name).changed() {
                    self.events.change.name.send(Change::new(NameInSite(new_name), e));
                }
            });
        }

        ui.horizontal(|ui| {
            let add = ui.button("Add").clicked();
            if self.events.display.nav_graph.color.is_none() {
                let next_color_index = graph_count % DEFAULT_NAV_GRAPH_COLORS.len();
                self.events.display.nav_graph.color = Some(DEFAULT_NAV_GRAPH_COLORS[next_color_index]);
            }
            if let Some(color) = &mut self.events.display.nav_graph.color {
                color_edit(ui, color);
            }
            ui.text_edit_singleline(&mut self.events.display.nav_graph.name);
            if add {
                self.events.commands
                    .spawn_bundle(SpatialBundle::default())
                    .insert_bundle(NavGraph {
                        name: NameInSite(self.events.display.nav_graph.name.clone()),
                        color: DisplayColor(self.events.display.nav_graph.color.unwrap().clone()),
                        marker: Default::default(),
                    });
                self.events.display.nav_graph.color = None;
            }
        });
    }
}
