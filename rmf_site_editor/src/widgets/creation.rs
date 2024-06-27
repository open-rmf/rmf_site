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
    inspector::{InspectAssetSourceComponent, InspectScaleComponent},
    interaction::{ChangeMode, SelectAnchor, SelectAnchor3D},
    site::{AssetSource, DefaultFile, DrawingBundle, Recall, RecallAssetSource, Scale},
    widgets::{prelude::*, AssetGalleryStatus},
    AppState, CurrentWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, Ui};

use rmf_site_format::{DrawingProperties, Geometry, Model, WorkcellModel};

/// This widget provides a widget with buttons for creating new site elements.
#[derive(Default)]
pub struct CreationPlugin {}

impl Plugin for CreationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingDrawing>()
            .init_resource::<PendingModel>()
            .add_plugins(PropertiesTilePlugin::<Creation>::new());
    }
}

#[derive(SystemParam)]
struct Creation<'w, 's> {
    default_file: Query<'w, 's, &'static DefaultFile>,
    app_state: Res<'w, State<AppState>>,
    change_mode: EventWriter<'w, ChangeMode>,
    current_workspace: Res<'w, CurrentWorkspace>,
    pending_drawings: ResMut<'w, PendingDrawing>,
    pending_model: ResMut<'w, PendingModel>,
    asset_gallery: Option<ResMut<'w, AssetGalleryStatus>>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for Creation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        match params.app_state.get() {
            AppState::SiteEditor | AppState::SiteDrawingEditor | AppState::WorkcellEditor => {}
            AppState::MainMenu | AppState::SiteVisualizer => return,
        }
        CollapsingHeader::new("Create")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> Creation<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            match self.app_state.get() {
                AppState::MainMenu | AppState::SiteVisualizer => {
                    return;
                }
                AppState::SiteEditor => {
                    if ui.button("Lane").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_edge_sequence().for_lane().into(),
                        ));
                    }

                    if ui.button("Location").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_point().for_location().into(),
                        ));
                    }

                    if ui.button("Wall").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_edge_sequence().for_wall().into(),
                        ));
                    }

                    if ui.button("Door").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_one_new_edge().for_door().into(),
                        ));
                    }

                    if ui.button("Lift").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_one_new_edge().for_lift().into(),
                        ));
                    }

                    if ui.button("Floor").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_path().for_floor().into(),
                        ));
                    }
                    if ui.button("Fiducial").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_point().for_site_fiducial().into(),
                        ));
                    }

                    ui.add_space(10.0);
                    CollapsingHeader::new("New drawing")
                        .default_open(false)
                        .show(ui, |ui| {
                            let default_file = self
                                .current_workspace
                                .root
                                .map(|e| self.default_file.get(e).ok())
                                .flatten();
                            if let Some(new_asset_source) = InspectAssetSourceComponent::new(
                                &self.pending_drawings.source,
                                &self.pending_drawings.recall_source,
                                default_file,
                            )
                            .show(ui)
                            {
                                self.pending_drawings
                                    .recall_source
                                    .remember(&new_asset_source);
                                self.pending_drawings.source = new_asset_source;
                            }
                            ui.add_space(5.0);
                            if ui.button("Add Drawing").clicked() {
                                self.commands.spawn(DrawingBundle::new(DrawingProperties {
                                    source: self.pending_drawings.source.clone(),
                                    ..default()
                                }));
                            }
                        });
                }
                AppState::SiteDrawingEditor => {
                    if ui.button("Fiducial").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_new_point()
                                .for_drawing_fiducial()
                                .into(),
                        ));
                    }
                    if ui.button("Measurement").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor::create_one_new_edge().for_measurement().into(),
                        ));
                    }
                }
                AppState::WorkcellEditor => {
                    if ui.button("Frame").clicked() {
                        self.change_mode.send(ChangeMode::To(
                            SelectAnchor3D::create_new_point().for_anchor(None).into(),
                        ));
                    }
                }
            }
            match self.app_state.get() {
                AppState::MainMenu | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {}
                AppState::SiteEditor | AppState::WorkcellEditor => {
                    ui.add_space(10.0);
                    CollapsingHeader::new("New model")
                        .default_open(false)
                        .show(ui, |ui| {
                            let default_file = self
                                .current_workspace
                                .root
                                .map(|e| self.default_file.get(e).ok())
                                .flatten();
                            if let Some(new_asset_source) = InspectAssetSourceComponent::new(
                                &self.pending_model.source,
                                &self.pending_model.recall_source,
                                default_file,
                            )
                            .show(ui)
                            {
                                self.pending_model.recall_source.remember(&new_asset_source);
                                self.pending_model.source = new_asset_source;
                            }
                            ui.add_space(5.0);
                            if let Some(new_scale) =
                                InspectScaleComponent::new(&self.pending_model.scale).show(ui)
                            {
                                self.pending_model.scale = new_scale;
                            }
                            ui.add_space(5.0);
                            if let Some(asset_gallery) = &mut self.asset_gallery {
                                match self.app_state.get() {
                                    AppState::MainMenu
                                    | AppState::SiteDrawingEditor
                                    | AppState::SiteVisualizer => {}
                                    AppState::SiteEditor => {
                                        if ui.button("Browse fuel").clicked() {
                                            asset_gallery.show = true;
                                        }
                                        if ui.button("Spawn model").clicked() {
                                            let model = Model {
                                                source: self.pending_model.source.clone(),
                                                scale: self.pending_model.scale,
                                                ..default()
                                            };
                                            self.change_mode.send(ChangeMode::To(
                                                SelectAnchor3D::create_new_point()
                                                    .for_model(model)
                                                    .into(),
                                            ));
                                        }
                                    }
                                    AppState::WorkcellEditor => {
                                        if ui.button("Browse fuel").clicked() {
                                            asset_gallery.show = true;
                                        }
                                        if ui.button("Spawn visual").clicked() {
                                            let workcell_model = WorkcellModel {
                                                geometry: Geometry::Mesh {
                                                    source: self.pending_model.source.clone(),
                                                    scale: Some(*self.pending_model.scale),
                                                },
                                                ..default()
                                            };
                                            self.change_mode.send(ChangeMode::To(
                                                SelectAnchor3D::create_new_point()
                                                    .for_visual(workcell_model)
                                                    .into(),
                                            ));
                                        }
                                        if ui.button("Spawn collision").clicked() {
                                            let workcell_model = WorkcellModel {
                                                geometry: Geometry::Mesh {
                                                    source: self.pending_model.source.clone(),
                                                    scale: Some(*self.pending_model.scale),
                                                },
                                                ..default()
                                            };
                                            self.change_mode.send(ChangeMode::To(
                                                SelectAnchor3D::create_new_point()
                                                    .for_collision(workcell_model)
                                                    .into(),
                                            ));
                                        }
                                        ui.add_space(10.0);
                                    }
                                }
                            }
                        });
                }
            }
        });
    }
}

#[derive(Resource, Clone, Default)]
struct PendingDrawing {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
}

#[derive(Resource, Clone, Default)]
struct PendingModel {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
    pub scale: Scale,
}
