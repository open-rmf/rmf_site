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
    site::{Category, Change, Delete, LevelElevation, LevelProperties, NameInSite},
    widgets::{AppEvents, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, ImageButton, Ui};
use std::cmp::{Ordering, Reverse};

#[derive(Resource)]
pub struct LevelDisplay {
    pub new_elevation: f32,
    pub new_name: String,
    pub order: Vec<Entity>,
    pub freeze: bool,
    pub removing: bool,
}

impl Default for LevelDisplay {
    fn default() -> Self {
        Self {
            new_elevation: 0.0,
            new_name: "<Unnamed>".to_string(),
            order: Vec::new(),
            freeze: false,
            removing: false,
        }
    }
}

#[derive(SystemParam)]
pub struct LevelParams<'w, 's> {
    pub levels: Query<'w, 's, (Entity, &'static NameInSite, &'static LevelElevation)>,
    pub parents: Query<'w, 's, &'static Parent>,
    pub icons: Res<'w, Icons>,
}

pub struct ViewLevels<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a LevelParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
    edit_visibility: bool,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewLevels<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a LevelParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self {
            params,
            events,
            edit_visibility: false,
        }
    }

    pub fn for_editing_visibility(mut self) -> Self {
        self.edit_visibility = true;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let make_new_level = ui.button("Add").clicked();
            let mut show_elevation = self.events.display.level.new_elevation;
            ui.add(DragValue::new(&mut show_elevation).suffix("m"))
                .on_hover_text("Elevation for the new level");

            let mut show_name = self.events.display.level.new_name.clone();
            ui.text_edit_singleline(&mut show_name)
                .on_hover_text("Name for the new level");

            if make_new_level {
                let new_level = self
                    .events
                    .commands
                    .spawn(SpatialBundle::default())
                    .insert(LevelProperties {
                        elevation: LevelElevation(show_elevation),
                        name: NameInSite(show_name.clone()),
                        ..Default::default()
                    })
                    .insert(Category::Level)
                    .id();
                self.events.request.current_level.0 = Some(new_level);
            }

            self.events.display.level.new_elevation = show_elevation;
            self.events.display.level.new_name = show_name;
        });

        if !self.events.display.level.freeze {
            let mut ordered_level_list: Vec<_> = self
                .params
                .levels
                .iter()
                .filter(|(e, _, _)| {
                    AncestorIter::new(&self.params.parents, *e)
                        .any(|e| Some(e) == self.events.request.current_workspace.root)
                })
                .map(|(e, _, elevation)| (Reverse(elevation.0), e))
                .collect();

            ordered_level_list.sort_by(|(h_a, e_a), (h_b, e_b)| {
                match h_a.partial_cmp(&h_b) {
                    None | Some(Ordering::Equal) => {
                        // Break elevation ties with an arbitrary but perfectly
                        // stable and consistent comparison metric.
                        e_b.cmp(&e_a)
                    }
                    Some(other) => other,
                }
            });

            self.events.display.level.order =
                ordered_level_list.into_iter().map(|(_, e)| e).collect();
        }

        if self.events.display.level.removing {
            ui.horizontal(|ui| {
                if ui.button("Select").clicked() {
                    self.events.display.level.removing = false;
                }
                ui.label("Remove");
            });
        } else {
            ui.horizontal(|ui| {
                ui.label("Select");
                if ui.button("Remove").clicked() {
                    self.events.display.level.removing = true;
                }
            });
        }

        let mut any_dragging = false;
        let mut any_deleted = false;
        for e in self.events.display.level.order.iter().copied() {
            if let Ok((_, name, elevation)) = self.params.levels.get(e) {
                let mut shown_elevation = elevation.clone().0;
                let mut shown_name = name.clone().0;
                ui.horizontal(|ui| {
                    if self.events.display.level.removing {
                        if ui
                            .add(ImageButton::new(self.params.icons.trash.egui()))
                            .on_hover_text("Remove this level")
                            .clicked()
                        {
                            self.events
                                .request
                                .delete
                                .send(Delete::new(e).and_dependents());
                            any_deleted = true;
                        }
                    } else if self.edit_visibility == true {
                        if ui
                            .radio(Some(e) == **self.events.request.current_level, "")
                            .clicked()
                        {
                            self.events.request.current_level.0 = Some(e);
                        }
                    }

                    let r = ui
                        .add(DragValue::new(&mut shown_elevation).suffix("m"))
                        .on_hover_text("Elevation of the level");
                    if r.dragged() || r.has_focus() {
                        any_dragging = true;
                    }

                    ui.text_edit_singleline(&mut shown_name)
                        .on_hover_text("Name of the level");
                });

                if shown_name != name.0 {
                    self.events
                        .change
                        .name
                        .send(Change::new(NameInSite(shown_name), e));
                }

                if shown_elevation != elevation.0 {
                    self.events
                        .change
                        .level_elevation
                        .send(Change::new(LevelElevation(shown_elevation), e));
                }
            }
        }

        self.events.display.level.freeze = any_dragging;
        if any_deleted {
            self.events.display.level.removing = false;
        }
    }
}
