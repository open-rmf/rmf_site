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
    inspector::{InspectAssetSource, InspectScale},
    interaction::{ChangeMode, SelectAnchor, SelectAnchor3D},
    site::{Change, DefaultFile, DrawingBundle, DrawingMarker, Recall},
    AppEvents, AppState, CurrentWorkspace, SuppressRecencyRank,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, Ui};

use rmf_site_format::{
    AssetSource, DrawingProperties, Geometry, Model, ModelMarker, Pending, RecallAssetSource,
    Scale, WorkcellModel,
};

#[derive(SystemParam)]
pub struct CreateParams<'w, 's> {
    pub default_file: Query<'w, 's, &'static DefaultFile>,
}

pub struct CreateWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub params: &'a CreateParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> CreateWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(params: &'a CreateParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        ui.vertical(|ui| {
            match self.events.app_state.current() {
                AppState::MainMenu | AppState::SiteVisualizer => {
                    return;
                }
                AppState::SiteEditor => {
                    if ui.button("Lane").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_edge_sequence().for_lane().into(),
                        ));
                    }

                    if ui.button("Location").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_point().for_location().into(),
                        ));
                    }

                    if ui.button("Wall").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_edge_sequence().for_wall().into(),
                        ));
                    }

                    if ui.button("Door").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_one_new_edge().for_door().into(),
                        ));
                    }

                    if ui.button("Lift").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_one_new_edge().for_lift().into(),
                        ));
                    }

                    if ui.button("Floor").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_path().for_floor().into(),
                        ));
                    }
                    if ui.button("Fiducial").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_point().for_site_fiducial().into(),
                        ));
                    }

                    ui.add_space(10.0);
                    CollapsingHeader::new("New drawing")
                        .default_open(false)
                        .show(ui, |ui| {
                            let default_file = self
                                .events
                                .request
                                .current_workspace
                                .root
                                .map(|e| self.params.default_file.get(e).ok())
                                .flatten();
                            if let Some(new_asset_source) = InspectAssetSource::new(
                                &self.events.display.pending_drawings.source,
                                &self.events.display.pending_drawings.recall_source,
                                default_file,
                            )
                            .show(ui)
                            {
                                self.events
                                    .display
                                    .pending_drawings
                                    .recall_source
                                    .remember(&new_asset_source);
                                self.events.display.pending_drawings.source = new_asset_source;
                            }
                            ui.add_space(5.0);
                            if ui.button("Add Drawing").clicked() {
                                self.events
                                    .commands
                                    .spawn(DrawingBundle::new(DrawingProperties {
                                        source: self.events.display.pending_drawings.source.clone(),
                                        ..default()
                                    }));
                            }
                        });
                }
                AppState::SiteDrawingEditor => {
                    if ui.button("Fiducial").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_point()
                                .for_drawing_fiducial()
                                .into(),
                        ));
                    }
                    if ui.button("Measurement").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_one_new_edge().for_measurement().into(),
                        ));
                    }
                }
                AppState::WorkcellEditor => {
                    if ui.button("Frame").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor3D::create_new_point().for_anchor(None).into(),
                        ));
                    }
                }
            }
            match self.events.app_state.current() {
                AppState::MainMenu | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {}
                AppState::SiteEditor | AppState::WorkcellEditor => {
                    ui.add_space(10.0);
                    CollapsingHeader::new("New model")
                        .default_open(false)
                        .show(ui, |ui| {
                            let default_file = self
                                .events
                                .request
                                .current_workspace
                                .root
                                .map(|e| self.params.default_file.get(e).ok())
                                .flatten();
                            if let Some(new_asset_source) = InspectAssetSource::new(
                                &self.events.display.pending_model.source,
                                &self.events.display.pending_model.recall_source,
                                default_file,
                            )
                            .show(ui)
                            {
                                self.events
                                    .display
                                    .pending_model
                                    .recall_source
                                    .remember(&new_asset_source);
                                self.events.display.pending_model.source = new_asset_source;
                            }
                            ui.add_space(5.0);
                            if let Some(new_scale) =
                                InspectScale::new(&self.events.display.pending_model.scale).show(ui)
                            {
                                self.events.display.pending_model.scale = new_scale;
                            }
                            ui.add_space(5.0);
                            match self.events.app_state.current() {
                                AppState::MainMenu
                                | AppState::SiteDrawingEditor
                                | AppState::SiteVisualizer => {}
                                AppState::SiteEditor => {
                                    if ui.button("Browse fuel").clicked() {
                                        self.events.new_model.asset_gallery_status.show = true;
                                    }
                                    if ui.button("Spawn model").clicked() {
                                        let model = Model {
                                            source: self
                                                .events
                                                .display
                                                .pending_model
                                                .source
                                                .clone(),
                                            scale: self.events.display.pending_model.scale,
                                            ..default()
                                        };
                                        self.events.request.change_mode.send(ChangeMode::To(
                                            SelectAnchor3D::create_new_point()
                                                .for_model(model)
                                                .into(),
                                        ));
                                    }
                                }
                                AppState::WorkcellEditor => {
                                    if ui.button("Browse fuel").clicked() {
                                        self.events.new_model.asset_gallery_status.show = true;
                                    }
                                    if ui.button("Spawn visual").clicked() {
                                        let workcell_model = WorkcellModel {
                                            geometry: Geometry::Mesh {
                                                source: self
                                                    .events
                                                    .display
                                                    .pending_model
                                                    .source
                                                    .clone(),
                                                scale: Some(
                                                    *self.events.display.pending_model.scale,
                                                ),
                                            },
                                            ..default()
                                        };
                                        self.events.request.change_mode.send(ChangeMode::To(
                                            SelectAnchor3D::create_new_point()
                                                .for_visual(workcell_model)
                                                .into(),
                                        ));
                                    }
                                    if ui.button("Spawn collision").clicked() {
                                        let workcell_model = WorkcellModel {
                                            geometry: Geometry::Mesh {
                                                source: self
                                                    .events
                                                    .display
                                                    .pending_model
                                                    .source
                                                    .clone(),
                                                scale: Some(
                                                    *self.events.display.pending_model.scale,
                                                ),
                                            },
                                            ..default()
                                        };
                                        self.events.request.change_mode.send(ChangeMode::To(
                                            SelectAnchor3D::create_new_point()
                                                .for_collision(workcell_model)
                                                .into(),
                                        ));
                                    }
                                    ui.add_space(10.0);
                                }
                            }
                        });
                }
            }
        });
    }
}
