/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
    site::*,
    widgets::{
        inspector::{InspectLayer, InspectLayerInput},
        prelude::*,
        Icons, MoveLayer,
    },
    AppState,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, DragValue, ScrollArea, Ui};
use rmf_site_egui::{PropertiesTilePlugin, Tile, WidgetSystem};
use rmf_site_picking::Selection;

/// Add a widget for viewing a list of layers
#[derive(Default)]
pub struct ViewLayersPlugin {}

impl Plugin for ViewLayersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PropertiesTilePlugin::<ViewLayers>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewLayers<'w, 's> {
    commands: Commands<'w, 's>,
    floors: Query<
        'w,
        's,
        (
            &'static RecencyRanking<FloorMarker>,
            &'static GlobalFloorVisibility,
        ),
    >,
    drawings: Query<
        'w,
        's,
        (
            &'static RecencyRanking<DrawingMarker>,
            &'static GlobalDrawingVisibility,
        ),
    >,
    icons: Res<'w, Icons>,
    selection: Res<'w, Selection>,
    current_level: Res<'w, CurrentLevel>,
    view_layer: InspectLayer<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewLayers<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        ui.separator();
        CollapsingHeader::new("Layers")
            .default_open(false)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewLayers<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let Some(current_level) = self.current_level.0 else {
            return;
        };

        let has_drawings = self
            .drawings
            .get(current_level)
            .ok()
            .is_some_and(|(ranking, _)| !ranking.is_empty());

        if let Ok((ranking, global)) = self.floors.get(current_level) {
            CollapsingHeader::new("Floors")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut shown_global = global.clone();
                        let (text, vis) = if has_drawings {
                            ("Global", &mut shown_global.general)
                        } else {
                            (
                                "Global (without drawings)",
                                &mut shown_global.without_drawings,
                            )
                        };
                        let default_alpha = &mut shown_global.preferred_semi_transparency;
                        Self::show_global(text, &self.icons, vis, default_alpha, ui);

                        if shown_global != *global {
                            self.commands
                                .trigger(Change::new(shown_global, current_level));
                        }
                    });
                    ui.separator();
                    if let Some(selected) = Self::show_rankings(
                        ranking.entities(),
                        &self.selection,
                        &mut self.view_layer,
                        ui,
                    ) {
                        ui.horizontal(|ui| {
                            MoveLayer::new(
                                selected,
                                &mut self.view_layer.floor_change_rank,
                                &self.icons,
                            )
                            .show(ui);
                        });
                    }
                    ui.separator();
                });
        }

        if let Ok((ranking, global)) = self.drawings.get(current_level) {
            CollapsingHeader::new("Drawings")
                .default_open(true)
                .show(ui, |ui| {
                    let mut shown_global = global.clone();
                    ui.horizontal(|ui| {
                        let vis = &mut shown_global.general;
                        let default_alpha = &mut shown_global.preferred_general_semi_transparency;
                        Self::show_global("Global (general)", &self.icons, vis, default_alpha, ui);
                    });
                    ui.horizontal(|ui| {
                        let vis = &mut shown_global.bottom;
                        let default_alpha = &mut shown_global.preferred_bottom_semi_transparency;
                        Self::show_global("Global (bottom)", &self.icons, vis, default_alpha, ui);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Bottom Count").on_hover_text(
                            "How many of the lowest layer drawings are part of the bottom?",
                        );
                        // ui.with_layer_id(, add_contents)
                        ui.push_id("Bottom Drawing Count", |ui| {
                            ui.add(
                                DragValue::new(&mut shown_global.bottom_count)
                                    .range(0..=usize::MAX)
                                    .speed(0.05),
                            );
                        });
                    });

                    if shown_global != *global {
                        self.commands
                            .trigger(Change::new(shown_global, current_level));
                    }

                    if let Some(selected) = Self::show_rankings(
                        ranking.entities(),
                        &self.selection,
                        &mut self.view_layer,
                        ui,
                    ) {
                        ui.horizontal(|ui| {
                            MoveLayer::new(
                                selected,
                                &mut self.view_layer.drawing_change_rank,
                                &self.icons,
                            )
                            .show(ui);
                        });
                    }
                });
        }
    }

    fn show_global(
        text: &str,
        icons: &Icons,
        vis: &mut LayerVisibility,
        default_alpha: &mut f32,
        ui: &mut Ui,
    ) {
        let icon = icons.layer_visibility_of(Some(*vis));
        if ui
            .add(Button::image_and_text(icon, text))
            .on_hover_text(format!("Change to {}", vis.next(*default_alpha).label()))
            .clicked()
        {
            *vis = vis.next(*default_alpha);
        }

        if let LayerVisibility::Alpha(alpha) = vis {
            if ui
                .add(DragValue::new(alpha).range(0_f32..=1_f32).speed(0.01))
                .changed()
            {
                *default_alpha = *alpha;
            }
        }
    }

    fn show_rankings(
        ranking: &Vec<Entity>,
        selection: &Selection,
        view_layer: &mut InspectLayer,
        ui: &mut Ui,
    ) -> Option<Entity> {
        let mut layer_selected = None;
        ui.vertical(|ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for id in ranking.iter().rev().copied() {
                    if selection.0.is_some_and(|s| s == id) {
                        layer_selected = Some(id);
                    }
                    ui.horizontal(|ui| {
                        view_layer.show_widget(InspectLayerInput::new(id).with_selecting(), ui);
                    });
                }
            });
        });
        layer_selected
    }
}
