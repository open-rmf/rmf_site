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
        Category, Change, CurrentLevel, Delete, DrawingMarker, FloorMarker, LevelElevation,
        LevelProperties, NameInSite,
    },
    widgets::{prelude::*, Icons},
    AppState, CurrentWorkspace, RecencyRanking,
};
use bevy::{
    ecs::{hierarchy::ChildOf, relationship::AncestorIter, system::SystemParam},
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, DragValue, ImageButton, TextEdit, Ui, Widget};
use rmf_site_egui::*;
use std::cmp::{Ordering, Reverse};

/// Add a plugin for viewing and editing a list of all levels
#[derive(Default)]
pub struct ViewLevelsPlugin {}

impl Plugin for ViewLevelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelDisplay>()
            .add_plugins(PropertiesTilePlugin::<ViewLevels>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewLevels<'w, 's> {
    levels: Query<'w, 's, (Entity, &'static NameInSite, &'static LevelElevation)>,
    child_of: Query<'w, 's, &'static ChildOf>,
    icons: Res<'w, Icons>,
    display_levels: ResMut<'w, LevelDisplay>,
    current_level: ResMut<'w, CurrentLevel>,
    current_workspace: ResMut<'w, CurrentWorkspace>,
    change_name: EventWriter<'w, Change<NameInSite>>,
    change_level_elevation: EventWriter<'w, Change<LevelElevation>>,
    delete: EventWriter<'w, Delete>,
    commands: Commands<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewLevels<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        CollapsingHeader::new("Levels")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewLevels<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let editing = match self.app_state.get() {
            AppState::SiteEditor => true,
            AppState::SiteVisualizer => false,
            _ => return,
        };

        if !editing {
            self.display_levels.removing = false;
        }

        if editing {
            ui.horizontal(|ui| {
                let make_new_level = ui.button("Add").clicked();
                let mut show_elevation = self.display_levels.new_elevation;
                ui.add(DragValue::new(&mut show_elevation).suffix("m"))
                    .on_hover_text("Elevation for the new level");

                let mut show_name = self.display_levels.new_name.clone();

                TextEdit::singleline(&mut show_name)
                    .desired_width(ui.available_width())
                    .ui(ui)
                    .on_hover_text("Name for the new level");

                if make_new_level {
                    let new_level = self
                        .commands
                        .spawn((
                            Transform::default(),
                            Visibility::default(),
                            LevelProperties {
                                elevation: LevelElevation(show_elevation),
                                name: NameInSite(show_name.clone()),
                                ..Default::default()
                            },
                            Category::Level,
                            RecencyRanking::<DrawingMarker>::default(),
                            RecencyRanking::<FloorMarker>::default(),
                        ))
                        .id();
                    self.current_level.0 = Some(new_level);
                }

                self.display_levels.new_elevation = show_elevation;
                self.display_levels.new_name = show_name;
            });
        }

        if !self.display_levels.freeze {
            let mut ordered_level_list: Vec<_> = self
                .levels
                .iter()
                .filter(|(e, _, _)| {
                    AncestorIter::new(&self.child_of, *e)
                        .any(|e| Some(e) == self.current_workspace.root)
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

            self.display_levels.order = ordered_level_list.into_iter().map(|(_, e)| e).collect();
        }

        if self.display_levels.removing {
            ui.horizontal(|ui| {
                if ui.button("Select").clicked() {
                    self.display_levels.removing = false;
                }
                ui.label("Remove");
            });
        } else if editing {
            ui.horizontal(|ui| {
                ui.label("Select");
                if ui.button("Remove").clicked() {
                    self.display_levels.removing = true;
                }
            });
        }

        let mut any_dragging = false;
        let mut any_deleted = false;
        for e in self.display_levels.order.iter().copied() {
            if let Ok((_, name, elevation)) = self.levels.get(e) {
                let mut shown_elevation = elevation.clone().0;
                let mut shown_name = name.clone().0;
                ui.horizontal(|ui| {
                    if self.display_levels.removing {
                        if ui
                            .add(ImageButton::new(self.icons.trash.egui()))
                            .on_hover_text("Remove this level")
                            .clicked()
                        {
                            self.delete.write(Delete::new(e).and_dependents());
                            any_deleted = true;
                        }
                    } else if editing {
                        if ui.radio(Some(e) == **self.current_level, "").clicked() {
                            self.current_level.0 = Some(e);
                        }
                    }

                    let r = ui
                        .add(DragValue::new(&mut shown_elevation).suffix("m"))
                        .on_hover_text("Elevation of the level");
                    if r.dragged() || r.has_focus() {
                        any_dragging = true;
                    }

                    TextEdit::singleline(&mut shown_name)
                        .desired_width(ui.available_width())
                        .ui(ui)
                        .on_hover_text("Name of the level");
                });

                if shown_name != name.0 {
                    self.change_name
                        .write(Change::new(NameInSite(shown_name), e));
                }

                if shown_elevation != elevation.0 {
                    self.change_level_elevation
                        .write(Change::new(LevelElevation(shown_elevation), e));
                }
            }
        }

        self.display_levels.freeze = any_dragging;
        if any_deleted {
            self.display_levels.removing = false;
        }
    }
}

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
