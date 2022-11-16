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
    site::{Category, Change, Delete, LevelProperties},
    widgets::{AppEvents, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, ImageButton, Ui};
use std::cmp::{Ordering, Reverse};

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
    pub levels: Query<'w, 's, (Entity, &'static LevelProperties)>,
    pub icons: Res<'w, Icons>,
}

pub struct ViewLevels<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a LevelParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewLevels<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a LevelParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let make_new_level = ui.button("Add").clicked();
            let mut show_elevation = self.events.level_display.new_elevation;
            ui.add(DragValue::new(&mut show_elevation).suffix("m"))
                .on_hover_text("Elevation for the new level");

            let mut show_name = self.events.level_display.new_name.clone();
            ui.text_edit_singleline(&mut show_name)
                .on_hover_text("Name for the new level");

            if make_new_level {
                let new_level = self
                    .events
                    .commands
                    .spawn_bundle(SpatialBundle::default())
                    .insert(LevelProperties {
                        elevation: show_elevation,
                        name: show_name.clone(),
                    })
                    .insert(Category::Level)
                    .id();
                self.events.current_level.0 = Some(new_level);
            }

            self.events.level_display.new_elevation = show_elevation;
            self.events.level_display.new_name = show_name;
        });

        if !self.events.level_display.freeze {
            let mut ordered_level_list: Vec<_> = self
                .params
                .levels
                .iter()
                .map(|(e, props)| (Reverse(props.elevation), e))
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

            self.events.level_display.order =
                ordered_level_list.into_iter().map(|(_, e)| e).collect();
        }

        if self.events.level_display.removing {
            ui.horizontal(|ui| {
                if ui.button("Select").clicked() {
                    self.events.level_display.removing = false;
                }
                ui.label("Remove");
            });
        } else {
            ui.horizontal(|ui| {
                ui.label("Select");
                if ui.button("Remove").clicked() {
                    self.events.level_display.removing = true;
                }
            });
        }

        let mut any_dragging = false;
        for e in self.events.level_display.order.iter().copied() {
            if let Ok((_, props)) = self.params.levels.get(e) {
                let mut shown_props = props.clone();
                ui.horizontal(|ui| {
                    if self.events.level_display.removing {
                        if ui
                            .add(ImageButton::new(self.params.icons.egui_trash, [18., 18.]))
                            .on_hover_text("Remove this level")
                            .clicked()
                        {
                            self.events.delete.send(Delete::new(e).and_dependents());
                        }
                    } else {
                        if ui
                            .radio(Some(e) == **self.events.current_level, "")
                            .clicked()
                        {
                            self.events.current_level.0 = Some(e);
                        }
                    }

                    let r = ui
                        .add(DragValue::new(&mut shown_props.elevation).suffix("m"))
                        .on_hover_text("Elevation of the level");
                    if r.dragged() || r.has_focus() {
                        any_dragging = true;
                    }

                    ui.text_edit_singleline(&mut shown_props.name)
                        .on_hover_text("Name of the level");
                });

                if shown_props != *props {
                    self.events
                        .change_level_props
                        .send(Change::new(shown_props, e));
                }
            }
        }

        self.events.level_display.freeze = any_dragging;
    }
}
