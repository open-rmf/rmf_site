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
    site::{Change, DrawingBundle, DrawingMarker, Recall},
    AppEvents, AppState, SuppressRecencyRank,
};
use bevy::prelude::*;
use bevy_egui::egui::{CollapsingHeader, Ui};

use rmf_site_format::{
    AssetSource, Drawing, Geometry, Model, ModelMarker, Pending, RecallAssetSource, Scale,
    WorkcellModel,
};

pub struct CreateWidget<'a, 'w, 's> {
    pub events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> CreateWidget<'a, 'w, 's> {
    pub fn new(events: &'a mut AppEvents<'w, 's>) -> Self {
        Self { events }
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
                    if ui.button("Constraint").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_one_new_edge().for_constraint().into(),
                        ));
                    }

                    ui.add_space(10.0);
                    CollapsingHeader::new("New drawing")
                        .default_open(false)
                        .show(ui, |ui| {
                            if let Some(new_asset_source) = InspectAssetSource::new(
                                &self.events.pending_drawings.source,
                                &self.events.pending_drawings.recall_source,
                            ).show(ui) {
                                self.events.pending_drawings.recall_source.remember(
                                    &new_asset_source
                                );
                                self.events.pending_drawings.source = new_asset_source;
                            }
                            ui.add_space(5.0);
                            if ui.button("Add Drawing").clicked() {
                                let drawing = Drawing {
                                    source: self.events.pending_drawings.source.clone(),
                                    ..default()
                                };
                                self.events.commands.spawn(DrawingBundle::new(&drawing));
                            }
                        });
                }
                AppState::SiteDrawingEditor => {
                    if ui.button("Fiducial").clicked() {
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_point().for_fiducial().into(),
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
                            if let Some(new_asset_source) = InspectAssetSource::new(
                                &self.events.pending_model.source,
                                &self.events.pending_model.recall_source,
                            ).show(ui) {
                                self.events.pending_model.recall_source.remember(&new_asset_source);
                                self.events.pending_model.source = new_asset_source;
                            }
                            ui.add_space(5.0);
                            if let Some(new_scale) = InspectScale::new(
                                &self.events.pending_model.scale,
                            ).show(ui) {
                                self.events.pending_model.scale = new_scale;
                            }
                            ui.add_space(5.0);
                            match self.events.app_state.current() {
                                AppState::MainMenu
                                | AppState::SiteDrawingEditor
                                | AppState::SiteVisualizer => {}
                                AppState::SiteEditor => {
                                    if ui.button("Spawn model").clicked() {
                                        let model = Model {
                                            source: self.events.pending_model.source.clone(),
                                            scale: self.events.pending_model.scale,
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
                                    if ui.button("Spawn visual").clicked() {
                                        let workcell_model = WorkcellModel {
                                            geometry: Geometry::Mesh {
                                                filename: (&self.events.pending_model.source).into(),
                                                scale: Some(*self.events.pending_model.scale),
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
                                                filename: (&self.events.pending_model.source).into(),
                                                scale: Some(*self.events.pending_model.scale),
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
