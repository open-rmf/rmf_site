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
    site::Change,
    AppEvents, AppState,
};
use bevy::prelude::*;
use bevy_egui::egui::{CollapsingHeader, Ui};

use rmf_site_format::{
    AssetSource, DrawingBundle, DrawingMarker, Geometry, Model, Pending, PixelsPerMeter, Pose,
    RecallAssetSource, Scale, WorkcellModel,
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
                AppState::MainMenu => {
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
            if let Ok((e, source, scale)) = self.events.pending_asset_sources.get_single() {
                ui.add_space(10.0);
                CollapsingHeader::new("New model")
                    .default_open(false)
                    .show(ui, |ui| {
                        if let Some(new_asset_source) =
                            InspectAssetSource::new(source, &RecallAssetSource::default()).show(ui)
                        {
                            self.events
                                .change
                                .asset_source
                                .send(Change::new(new_asset_source, e));
                        }
                        ui.add_space(5.0);
                        if let Some(new_scale) = InspectScale::new(scale).show(ui) {
                            self.events
                                .workcell_change
                                .scale
                                .send(Change::new(new_scale, e));
                        }
                        ui.add_space(5.0);
                        match self.events.app_state.current() {
                            AppState::MainMenu => {
                                unreachable!();
                            }
                            AppState::SiteEditor => {
                                if ui.button("Spawn model").clicked() {
                                    let model = Model {
                                        source: source.clone(),
                                        ..default()
                                    };
                                    self.events.request.change_mode.send(ChangeMode::To(
                                        SelectAnchor3D::create_new_point().for_model(model).into(),
                                    ));
                                }
                            }
                            AppState::SiteDrawingEditor => {
                                if ui.button("Add Drawing").clicked() {
                                    let drawing = DrawingBundle {
                                        name: Default::default(),
                                        source: source.clone(),
                                        pose: Default::default(),
                                        is_primary: Default::default(),
                                        pixels_per_meter: Default::default(),
                                        marker: DrawingMarker,
                                    };
                                    self.events.commands.spawn(drawing);
                                }
                            }
                            AppState::WorkcellEditor => {
                                if ui.button("Spawn visual").clicked() {
                                    let workcell_model = WorkcellModel {
                                        geometry: Geometry::Mesh {
                                            filename: source.into(),
                                            scale: Some(**scale),
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
                                            filename: source.into(),
                                            scale: Some(**scale),
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
            } else if self.events.pending_asset_sources.is_empty() {
                // Spawn one
                let source = AssetSource::Search("OpenRobotics/AdjTable".to_string());
                self.events
                    .commands
                    .spawn(source.clone())
                    .insert(Scale::default())
                    .insert(Pending);
            }
        });
    }
}
