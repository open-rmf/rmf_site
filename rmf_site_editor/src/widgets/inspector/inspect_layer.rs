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
    interaction::{Hover, Selection},
    site::{
        BeginEditDrawing, Change, LayerVisibility, PreferredSemiTransparency, SiteID,
        VisibilityCycle, FloorMarker, DrawingMarker,
    },
    widgets::{
        inspector::{SelectionWidget, Inspect},
        MoveLayer, SelectorWidget, AppEvents, Icons, prelude::*,
    },
    ChangeRank,
};
use bevy::prelude::*;
use bevy_egui::egui::{DragValue, ImageButton, Ui};

#[derive(SystemParam)]
pub struct ExInspectLayer<'w, 's> {
    pub drawings: Query<'w, 's, (), With<DrawingMarker>>,
    pub floors: Query<'w, 's, (), With<FloorMarker>>,
    pub layer: Query<
        'w,
        's,
        (
            Option<&'static LayerVisibility>,
            &'static PreferredSemiTransparency,
        ),
        Or<(With<FloorMarker>, With<DrawingMarker>)>,
    >,
    pub icons: Res<'w, Icons>,
    pub selection: Res<'w, Selection>,
    pub begin_edit_drawing: EventWriter<'w, BeginEditDrawing>,
    pub change_layer_visibility: EventWriter<'w, Change<LayerVisibility>>,
    pub change_preferred_alpha: EventWriter<'w, Change<PreferredSemiTransparency>>,
    pub floor_change_rank: EventWriter<'w, ChangeRank<FloorMarker>>,
    pub drawing_change_rank: EventWriter<'w, ChangeRank<DrawingMarker>>,
    pub commands: Commands<'w, 's>,
    pub selector: SelectorWidget<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for ExInspectLayer<'w, 's> {
    fn show(
        Inspect { selection: id, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(InspectLayerInput::new(id).with_moving(), ui);
    }
}

impl<'w, 's> WidgetSystem<InspectLayerInput> for ExInspectLayer<'w, 's> {
    fn show(input: InspectLayerInput, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        params.show_widget(input, ui);
    }
}

pub struct InspectLayerInput {
    pub id: Entity,
    pub with_selecting: bool,
    pub with_moving: bool,
}

impl InspectLayerInput {
    pub fn new(id: Entity) -> Self {
        Self { id, with_selecting: false, with_moving: false }
    }

    pub fn with_selecting(mut self) -> Self {
        self.with_selecting = true;
        self
    }

    pub fn with_moving(mut self) -> Self {
        self.with_moving = true;
        self
    }
}

impl<'w, 's> ExInspectLayer<'w, 's> {
    pub fn show_widget(
        &mut self,
        InspectLayerInput { id, with_selecting, with_moving }: InspectLayerInput,
        ui: &mut Ui
    ) {
        if !self.layer.contains(id) {
            return;
        }

        if with_moving {
            if self.drawings.contains(id) {
                ui.horizontal(|ui| {
                    MoveLayer::<DrawingMarker>::new(
                        id,
                        &mut self.drawing_change_rank,
                        &self.icons,
                    ).show(ui);
                });
            }

            if self.floors.contains(id) {
                ui.horizontal(|ui| {
                    MoveLayer::<FloorMarker>::new(
                        id,
                        &mut self.floor_change_rank,
                        &self.icons,
                    ).show(ui);
                });
            }
        }

        ui.horizontal(|ui| {
            if with_selecting {
                self.selector.show_widget(id, ui);
            }

            if with_selecting{
                if self.drawings.contains(id) {
                    let response = ui
                        .add(ImageButton::new(self.icons.edit.egui()))
                        .on_hover_text("Edit Drawing");

                    if response.hovered() {
                        self.selector.hover.send(Hover(Some(id)));
                    }

                    if response.clicked() {
                        self.begin_edit_drawing.send(BeginEditDrawing(id));
                    }
                }
            }

            let Ok((vis, default_alpha)) = self.layer.get(id) else {
                return;
            };
            let vis = vis.copied();
            let default_alpha = default_alpha.0;

            let icon = self.icons.layer_visibility_of(vis);
            let resp = ui.add(ImageButton::new(icon)).on_hover_text(format!(
                "Change to {}",
                vis.next(default_alpha).label()
            ));
            if resp.hovered() {
                self.selector.hover.send(Hover(Some(id)));
            }
            if resp.clicked() {
                match vis.next(default_alpha) {
                    Some(v) => {
                        self.change_layer_visibility.send(Change::new(v, id).or_insert());
                    }
                    None => {
                        self.commands.entity(id).remove::<LayerVisibility>();
                    }
                }
            }

            if let Some(LayerVisibility::Alpha(mut alpha)) = vis {
                if ui.add(
                    DragValue::new(&mut alpha)
                        .clamp_range(0_f32..=1_f32)
                        .speed(0.01),
                ).changed() {
                    self.change_layer_visibility.send(
                        Change::new(LayerVisibility::Alpha(alpha), id)
                    );
                    self.change_preferred_alpha.send(Change::new(
                        PreferredSemiTransparency(alpha), id,
                    ));
                }
            }
        });

    }
}

pub struct InspectLayer<'a, 'w, 's> {
    pub entity: Entity,
    pub icons: &'a Icons,
    /// Does the layer have a custom visibility setting?
    pub layer_vis: Option<LayerVisibility>,
    pub default_alpha: f32,
    // TODO(luca) make this an enum
    pub is_floor: bool,
    pub as_selected: bool,
    /// Outer Option: Can this be selected?
    /// Inner Option: Does this have a SiteID?
    pub site_id: Option<Option<SiteID>>,
    pub events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> InspectLayer<'a, 'w, 's> {
    pub fn new(
        entity: Entity,
        icons: &'a Icons,
        events: &'a mut AppEvents<'w, 's>,
        layer_vis: Option<LayerVisibility>,
        default_alpha: f32,
        is_floor: bool,
    ) -> Self {
        Self {
            entity,
            icons,
            events,
            layer_vis,
            default_alpha,
            is_floor,
            as_selected: false,
            site_id: None,
        }
    }
    pub fn with_selecting(mut self, site_id: Option<SiteID>) -> Self {
        self.site_id = Some(site_id);
        self
    }

    pub fn as_selected(mut self, as_selected: bool) -> Self {
        self.as_selected = as_selected;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        if let Some(site_id) = self.site_id {
            SelectionWidget::new(self.entity, site_id, self.icons, self.events)
                .as_selected(self.as_selected)
                .show(ui);

            if !self.is_floor {
                let response = ui
                    .add(ImageButton::new(self.events.layers.icons.edit.egui()))
                    .on_hover_text("Edit Drawing");

                if response.hovered() {
                    self.events.request.hover.send(Hover(Some(self.entity)));
                }

                if response.clicked() {
                    self.events
                        .layers
                        .begin_edit_drawing
                        .send(BeginEditDrawing(self.entity));
                }
            }
        }

        let icon = self.icons.layer_visibility_of(self.layer_vis);
        let resp = ui.add(ImageButton::new(icon)).on_hover_text(format!(
            "Change to {}",
            self.layer_vis.next(self.default_alpha).label()
        ));
        if resp.hovered() {
            self.events.request.hover.send(Hover(Some(self.entity)));
        }
        if resp.clicked() {
            match self.layer_vis.next(self.default_alpha) {
                Some(v) => {
                    self.events
                        .layers
                        .layer_vis
                        .send(Change::new(v, self.entity).or_insert());
                }
                None => {
                    self.events
                        .commands
                        .entity(self.entity)
                        .remove::<LayerVisibility>();
                }
            }
        }

        if let Some(LayerVisibility::Alpha(mut alpha)) = self.layer_vis {
            if ui
                .add(
                    DragValue::new(&mut alpha)
                        .clamp_range(0_f32..=1_f32)
                        .speed(0.01),
                )
                .changed()
            {
                self.events
                    .layers
                    .layer_vis
                    .send(Change::new(LayerVisibility::Alpha(alpha), self.entity));
                self.events
                    .layers
                    .preferred_alpha
                    .send(Change::new(PreferredSemiTransparency(alpha), self.entity));
            }
        }
    }
}
