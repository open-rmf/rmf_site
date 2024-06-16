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
    interaction::Selection,
    recency::RecencyRanking,
    site::*,
    widgets::{
        prelude::*,
        inspector::{InspectLayer, ExInspectLayer, InspectLayerInput},
        AppEvents, Icons, MoveLayer, PropertiesPanel,
    },
    AppState,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, DragValue, ScrollArea, Ui};

#[derive(Default)]
pub struct ViewLayersPlugin {

}

impl Plugin for ViewLayersPlugin {
    fn build(&self, app: &mut App) {
        let widget = Widget::new::<ExViewLayers>(&mut app.world);
        let properties_panel = app.world.resource::<PropertiesPanel>().id;
        app.world.spawn(widget).set_parent(properties_panel);
    }
}

#[derive(SystemParam)]
pub struct ExViewLayers<'w, 's> {
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
    layer_visibility: Query<
        'w,
        's,
        (
            Option<&'static LayerVisibility>,
            &'static PreferredSemiTransparency,
        ),
    >,
    levels: Query<
        'w,
        's,
        (
            &'static GlobalFloorVisibility,
            &'static GlobalDrawingVisibility,
        ),
    >,
    site_id: Query<'w, 's, Option<&'static SiteID>>,
    icons: Res<'w, Icons>,
    selection: Res<'w, Selection>,
    current_level: Res<'w, CurrentLevel>,
    global_floor_vis: EventWriter<'w, Change<GlobalFloorVisibility>>,
    global_drawing_vis: EventWriter<'w, Change<GlobalDrawingVisibility>>,
    view_layer: ExInspectLayer<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ExViewLayers<'w, 's> {
    fn show(
        _: Tile,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
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

impl<'w, 's> ExViewLayers<'w, 's> {
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
                            self.global_floor_vis
                                .send(Change::new(shown_global, current_level));
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
                                    .clamp_range(0..=usize::MAX)
                                    .speed(0.05),
                            );
                        });
                    });

                    if shown_global != *global {
                        self.global_drawing_vis
                            .send(Change::new(shown_global, current_level));
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
                .add(DragValue::new(alpha).clamp_range(0_f32..=1_f32).speed(0.01))
                .changed()
            {
                *default_alpha = *alpha;
            }
        }
    }

    fn show_rankings(
        ranking: &Vec<Entity>,
        selection: &Selection,
        view_layer: &mut ExInspectLayer,
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
                        view_layer.show_widget(
                            InspectLayerInput::new(id).with_selecting(), ui,
                        );
                    });
                }
            });
        });
        layer_selected
    }
}

#[derive(SystemParam)]
pub struct LayersParams<'w, 's> {
    pub floors: Query<
        'w,
        's,
        (
            &'static RecencyRanking<FloorMarker>,
            &'static GlobalFloorVisibility,
        ),
    >,
    pub drawings: Query<
        'w,
        's,
        (
            &'static RecencyRanking<DrawingMarker>,
            &'static GlobalDrawingVisibility,
        ),
    >,
    pub layer_visibility: Query<
        'w,
        's,
        (
            Option<&'static LayerVisibility>,
            &'static PreferredSemiTransparency,
        ),
    >,
    pub levels: Query<
        'w,
        's,
        (
            &'static GlobalFloorVisibility,
            &'static GlobalDrawingVisibility,
        ),
    >,
    pub site_id: Query<'w, 's, Option<&'static SiteID>>,
    pub icons: Res<'w, Icons>,
    pub selection: Res<'w, Selection>,
}

pub struct ViewLayers<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a LayersParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewLayers<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a LayersParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(mut self, ui: &mut Ui) {
        let current_level = match &self.events.request.current_level.0 {
            Some(s) => *s,
            None => return,
        };

        let has_drawings = self
            .params
            .drawings
            .get(current_level)
            .ok()
            .is_some_and(|(ranking, _)| !ranking.is_empty());

        if let Ok((ranking, global)) = self.params.floors.get(current_level) {
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
                        self.show_global(text, vis, default_alpha, ui);

                        if shown_global != *global {
                            self.events
                                .layers
                                .global_floor_vis
                                .send(Change::new(shown_global, current_level));
                        }
                    });
                    ui.separator();
                    if let Some(selected) = self.show_rankings(ranking.entities(), true, ui) {
                        ui.horizontal(|ui| {
                            MoveLayer::new(
                                selected,
                                &mut self.events.layers.floors,
                                &self.events.layers.icons,
                            )
                            .show(ui);
                        });
                    }
                    ui.separator();
                });
        }

        if let Ok((ranking, global)) = self.params.drawings.get(current_level) {
            CollapsingHeader::new("Drawings")
                .default_open(true)
                .show(ui, |ui| {
                    let mut shown_global = global.clone();
                    ui.horizontal(|ui| {
                        let vis = &mut shown_global.general;
                        let default_alpha = &mut shown_global.preferred_general_semi_transparency;
                        self.show_global("Global (general)", vis, default_alpha, ui);
                    });
                    ui.horizontal(|ui| {
                        let vis = &mut shown_global.bottom;
                        let default_alpha = &mut shown_global.preferred_bottom_semi_transparency;
                        self.show_global("Global (bottom)", vis, default_alpha, ui);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Bottom Count").on_hover_text(
                            "How many of the lowest layer drawings are part of the bottom?",
                        );
                        // ui.with_layer_id(, add_contents)
                        ui.push_id("Bottom Drawing Count", |ui| {
                            ui.add(
                                DragValue::new(&mut shown_global.bottom_count)
                                    .clamp_range(0..=usize::MAX)
                                    .speed(0.05),
                            );
                        });
                    });

                    if shown_global != *global {
                        self.events
                            .layers
                            .global_drawing_vis
                            .send(Change::new(shown_global, current_level));
                    }

                    if let Some(selected) = self.show_rankings(ranking.entities(), false, ui) {
                        ui.horizontal(|ui| {
                            MoveLayer::new(
                                selected,
                                &mut self.events.layers.drawings,
                                &self.events.layers.icons,
                            )
                            .show(ui);
                        });
                    }
                });
        }
    }

    fn show_global(
        &self,
        text: &str,
        vis: &mut LayerVisibility,
        default_alpha: &mut f32,
        ui: &mut Ui,
    ) {
        let icon = self.params.icons.layer_visibility_of(Some(*vis));
        if ui
            .add(Button::image_and_text(icon, text))
            .on_hover_text(format!("Change to {}", vis.next(*default_alpha).label()))
            .clicked()
        {
            *vis = vis.next(*default_alpha);
        }

        if let LayerVisibility::Alpha(alpha) = vis {
            if ui
                .add(DragValue::new(alpha).clamp_range(0_f32..=1_f32).speed(0.01))
                .changed()
            {
                *default_alpha = *alpha;
            }
        }
    }

    fn show_rankings(
        &mut self,
        ranking: &Vec<Entity>,
        is_floor: bool,
        ui: &mut Ui,
    ) -> Option<Entity> {
        let mut layer_selected = None;
        ui.vertical(|ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for e in ranking.iter().rev() {
                    let mut as_selected = false;
                    if self.params.selection.0.is_some_and(|sel| sel == *e) {
                        as_selected = true;
                        layer_selected = Some(*e);
                    }
                    let Ok((vis, alpha)) = self.params.layer_visibility.get(*e) else {
                        continue;
                    };
                    ui.horizontal(|ui| {
                        InspectLayer::new(
                            *e,
                            &self.params.icons,
                            &mut self.events,
                            vis.copied(),
                            alpha.0,
                            is_floor,
                        )
                        .with_selecting(self.params.site_id.get(*e).ok().flatten().copied())
                        .as_selected(as_selected)
                        .show(ui);
                    });
                }
            });
        });
        layer_selected
    }
}
