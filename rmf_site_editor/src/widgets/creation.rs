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
    interaction::{AnchorSelection, ObjectPlacement, PlaceableObject, Selection},
    site::{
        AssetSource, CurrentLevel, DefaultFile, DrawingBundle, Recall, RecallAssetSource, Scale,
    },
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
    current_workspace: Res<'w, CurrentWorkspace>,
    current_level: Res<'w, CurrentLevel>,
    pending_drawings: ResMut<'w, PendingDrawing>,
    pending_model: ResMut<'w, PendingModel>,
    asset_gallery: Option<ResMut<'w, AssetGalleryStatus>>,
    commands: Commands<'w, 's>,
    anchor_selection: AnchorSelection<'w, 's>,
    object_placement: ObjectPlacement<'w, 's>,
    selection: Res<'w, Selection>,
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
                        self.anchor_selection.create_lanes();
                    }

                    if ui.button("Location").clicked() {
                        self.anchor_selection.create_location();
                    }

                    if ui.button("Wall").clicked() {
                        self.anchor_selection.create_walls();
                    }

                    if ui.button("Door").clicked() {
                        self.anchor_selection.create_door();
                    }

                    if ui.button("Lift").clicked() {
                        self.anchor_selection.create_lift();
                    }

                    if ui.button("Floor").clicked() {
                        self.anchor_selection.create_floor();
                    }
                    if ui.button("Fiducial").clicked() {
                        self.anchor_selection.create_site_fiducial();
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
                        self.anchor_selection.create_drawing_fiducial();
                    }
                    if ui.button("Measurement").clicked() {
                        self.anchor_selection.create_measurements();
                    }
                }
                AppState::WorkcellEditor => {
                    if ui.button("Frame").clicked() {
                        self.place_object(PlaceableObject::Anchor);
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
                                            self.spawn_model_2d(model);
                                        }
                                    }
                                    AppState::WorkcellEditor => {
                                        if ui.button("Browse fuel").clicked() {
                                            asset_gallery.show = true;
                                        }
                                        if ui.button("Spawn visual").clicked() {
                                            let model = Model {
                                                source: self.pending_model.source.clone(),
                                                scale: Scale(*self.pending_model.scale),
                                                ..default()
                                            };
                                            self.place_object(PlaceableObject::VisualMesh(model));
                                        }
                                        if ui.button("Spawn collision").clicked() {
                                            let model = Model {
                                                source: self.pending_model.source.clone(),
                                                scale: Scale(*self.pending_model.scale),
                                                ..default()
                                            };
                                            self.place_object(PlaceableObject::CollisionMesh(
                                                model,
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

    pub fn place_object(&mut self, object: PlaceableObject) {
        if let Some(workspace) = self.current_workspace.root {
            self.object_placement
                .place_object_3d(object, self.selection.0, workspace);
        } else {
            warn!("Unable to create [{object:?}] outside of a workspace");
        }
    }

    pub fn spawn_model_2d(&mut self, object: Model) {
        if let Some(level) = self.current_level.0 {
            self.object_placement.place_object_2d(object, level);
        } else {
            warn!("Unable to create [{object:?}] outside of a level");
        }
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
