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
        AssetSource, Category, CurrentLevel, DefaultFile, DrawingBundle, Recall,
        RecallAssetSource, Scale
    },
    widgets::{prelude::*, AssetGalleryStatus},
    AppState, CurrentWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, ComboBox, Grid, Ui};

use rmf_site_format::{
    Affiliation, DrawingProperties, Geometry, Group, IsStatic, ModelDescriptionBundle,
    ModelInstance, ModelMarker, ModelProperty, NameInSite, SiteID, WorkcellModel,
};

/// This widget provides a widget with buttons for creating new site elements.
#[derive(Default)]
pub struct CreationPlugin {}

impl Plugin for CreationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreationData>()
            .add_plugins(PropertiesTilePlugin::<Creation>::new());
    }
}

#[derive(SystemParam)]
struct Creation<'w, 's> {
    default_file: Query<'w, 's, &'static DefaultFile>,
    app_state: Res<'w, State<AppState>>,
    current_workspace: Res<'w, CurrentWorkspace>,
    creation_data: ResMut<'w, CreationData>,
    current_level: Res<'w, CurrentLevel>,
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
        ui.horizontal(|ui| {
            ui.label("New");
            ComboBox::from_id_source("creation_mode")
                .selected_text(self.creation_data.to_string())
                .show_ui(ui, |ui| {
                    for mode_name in CreationData::string_values() {
                        if ui
                            .selectable_value(
                                &mut self.creation_data.to_string(),
                                mode_name,
                                mode_name,
                            )
                            .clicked()
                        {
                            *self.creation_data = CreationData::from_string(mode_name);
                        }
                    }
                });
        });
        ui.separator();

        match *self.creation_data {
            CreationData::SiteObject => {
                self.show_create_site_objects(ui);
            }
            CreationData::Drawing(_) => {
                self.show_create_drawing(ui);
            }
            CreationData::ModelDescription(_) => {
                self.show_create_model_description(ui);
            }
        }
    }

    pub fn show_create_site_objects(&mut self, ui: &mut Ui) {
        match self.app_state.get() {
            AppState::SiteEditor => {
                Grid::new("create_site_objects")
                    .num_columns(3)
                    .show(ui, |ui| {
                        if ui.button("↔ Lane").clicked() {
                            self.anchor_selection.create_lanes();
                        }

                        if ui.button("📌 Location").clicked() {
                            self.anchor_selection.create_location();
                        }

                        if ui.button("■ Wall").clicked() {
                            self.anchor_selection.create_walls();
                        }

                        ui.end_row();

                        if ui.button("🚪 Door").clicked() {
                            self.anchor_selection.create_door();
                        }

                        if ui.button("⬍ Lift").clicked() {
                            self.anchor_selection.create_lift();
                        }

                        if ui.button("✏ Floor").clicked() {
                            self.anchor_selection.create_floor();
                        }

                        ui.end_row();

                        if ui.button("☉ Fiducial").clicked() {
                            self.anchor_selection.create_site_fiducial();
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
            _ => {
                return;
            }
        }
    }

    pub fn show_create_drawing(&mut self, ui: &mut Ui) {
        match self.app_state.get() {
            AppState::SiteEditor => {
                let pending_drawings = match *self.creation_data {
                    CreationData::Drawing(ref mut pending_drawings) => pending_drawings,
                    _ => return,
                };

                let default_file = self
                    .current_workspace
                    .root
                    .map(|e| self.default_file.get(e).ok())
                    .flatten();
                if let Some(new_asset_source) = InspectAssetSourceComponent::new(
                    &pending_drawings.source,
                    &pending_drawings.recall_source,
                    default_file,
                )
                .show(ui)
                {
                    pending_drawings.recall_source.remember(&new_asset_source);
                    pending_drawings.source = new_asset_source;
                }
                ui.add_space(5.0);
                if ui.button("Add Drawing").clicked() {
                    self.commands.spawn(DrawingBundle::new(DrawingProperties {
                        source: pending_drawings.source.clone(),
                        ..default()
                    }));
                }
            }
            _ => {
                return;
            }
        }
    }

    pub fn show_create_model_description(&mut self, ui: &mut Ui) {
        match self.app_state.get() {
            AppState::MainMenu | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {}
            AppState::SiteEditor | AppState::WorkcellEditor => {
                let pending_model = match *self.creation_data {
                    CreationData::ModelDescription(ref mut pending_model) => pending_model,
                    _ => return,
                };

                ui.horizontal(|ui| {
                    ui.label("Description Name");
                    let mut new_name = pending_model.name.clone();
                    ui.text_edit_singleline(&mut new_name);
                    pending_model.name = new_name;
                });

                ui.add_space(10.0);
                let default_file = self
                    .current_workspace
                    .root
                    .map(|e| self.default_file.get(e).ok())
                    .flatten();
                if let Some(new_asset_source) = InspectAssetSourceComponent::new(
                    &pending_model.source,
                    &pending_model.recall_source,
                    default_file,
                )
                .show(ui)
                {
                    pending_model.recall_source.remember(&new_asset_source);
                    pending_model.source = new_asset_source;
                }

                ui.add_space(5.0);
                if let Some(new_scale) = InspectScaleComponent::new(&pending_model.scale).show(ui) {
                    pending_model.scale = new_scale;
                }

                ui.separator();
                if let Some(asset_gallery) = &mut self.asset_gallery {
                    match self.app_state.get() {
                        AppState::MainMenu
                        | AppState::SiteDrawingEditor
                        | AppState::SiteVisualizer => {}
                        AppState::SiteEditor => {
                            ui.add_space(5.0);

                            ui.horizontal(|ui| {
                                let add_icon = match pending_model.spawn_instance {
                                    true => "✚",
                                    false => "⬆",
                                };
                                if ui.button(add_icon).clicked() {
                                    if let Some(site_entity) = self.current_workspace.root {
                                        let model_description_bundle = ModelDescriptionBundle {
                                            name: NameInSite(pending_model.name.clone()),
                                            source: ModelProperty(pending_model.source.clone()),
                                            is_static: ModelProperty(IsStatic::default()),
                                            scale: ModelProperty(pending_model.scale.clone()),
                                            group: Group,
                                            marker: ModelMarker,
                                        };
                                        let description_entity = self
                                            .commands
                                            .spawn(model_description_bundle)
                                            .insert(Category::ModelDescription)
                                            .set_parent(site_entity)
                                            .id();
                                        if pending_model.spawn_instance {
                                            let model_instance: ModelInstance<Entity> =
                                                ModelInstance {
                                                    name: NameInSite(
                                                        pending_model.instance_name.clone(),
                                                    ),
                                                    description: Affiliation(Some(
                                                        description_entity,
                                                    )),
                                                    ..Default::default()
                                                };
                                            if let Some(level) = self.current_level.0 {
                                                self.object_placement.place_object_2d(model_instance, level);
                                            } else {
                                                warn!("Unable to create [{model_instance:?}] outside of a level");
                                            }
                                        }
                                    }
                                }
                                ComboBox::from_id_source("load_or_load_and_spawn")
                                    .selected_text(if pending_model.spawn_instance {
                                        "Load and Spawn"
                                    } else {
                                        "Load Description"
                                    })
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(
                                                pending_model.spawn_instance,
                                                "Load and Spawn",
                                            )
                                            .clicked()
                                        {
                                            pending_model.spawn_instance = true;
                                        }
                                        if ui
                                            .selectable_label(
                                                !pending_model.spawn_instance,
                                                "Load Description",
                                            )
                                            .clicked()
                                        {
                                            pending_model.spawn_instance = false;
                                        }
                                    });
                                if pending_model.spawn_instance {
                                    ui.text_edit_singleline(&mut pending_model.instance_name);
                                }
                            });
                            ui.add_space(3.0);
                            if ui
                                .selectable_label(asset_gallery.show, "Browse Fuel")
                                .clicked()
                            {
                                asset_gallery.show = !asset_gallery.show;
                            }
                        }
                        AppState::WorkcellEditor => {
                            if ui
                                .selectable_label(asset_gallery.show, "Browse Fuel")
                                .clicked()
                            {
                                asset_gallery.show = !asset_gallery.show;
                            }
                            // TODO(@xiyuoh) Review this block again after PR 239 has been merged.
                            // Use self.place_object instead without double borrowing mut
                            if ui.button("Spawn visual").clicked() {
                                let workcell_model = WorkcellModel {
                                    geometry: Geometry::Mesh {
                                        source: pending_model.source.clone(),
                                        scale: Some(*pending_model.scale),
                                    },
                                    ..default()
                                };
                                let object = PlaceableObject::VisualMesh(
                                    workcell_model,
                                );
                                if let Some(workspace) = self.current_workspace.root {
                                    self.object_placement
                                        .place_object_3d(object, self.selection.0, workspace);
                                } else {
                                    warn!("Unable to create [{object:?}] outside of a workspace");
                                }
                            }
                            if ui.button("Spawn collision").clicked() {
                                let workcell_model = WorkcellModel {
                                    geometry: Geometry::Mesh {
                                        source: pending_model.source.clone(),
                                        scale: Some(*pending_model.scale),
                                    },
                                    ..default()
                                };
                                self.place_object(PlaceableObject::CollisionMesh(
                                    workcell_model,
                                ));
                            }
                            ui.add_space(10.0);
                        }
                    }
                }
            }
        }
    }

    pub fn place_object(&mut self, object: PlaceableObject) {
        if let Some(workspace) = self.current_workspace.root {
            self.object_placement
                .place_object_3d(object, self.selection.0, workspace);
        } else {
            warn!("Unable to create [{object:?}] outside of a workspace");
        }
    }
}

#[derive(Resource, Clone, Default)]
enum CreationData {
    #[default]
    SiteObject,
    Drawing(PendingDrawing),
    ModelDescription(PendingModelDescription),
}

impl CreationData {
    fn to_string(&self) -> &str {
        match self {
            Self::SiteObject => "Site Object",
            Self::Drawing(_) => "Drawing",
            Self::ModelDescription(_) => "Model Description",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "Site Object" => Self::SiteObject,
            "Drawing" => Self::Drawing(PendingDrawing::default()),
            "Model Description" => Self::ModelDescription(PendingModelDescription::default()),
            _ => Self::SiteObject,
        }
    }

    fn string_values() -> Vec<&'static str> {
        vec![
            "Site Object",
            "Drawing",
            "Model Description",
        ]
    }
}

#[derive(Clone, Default)]
struct PendingDrawing {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
}

#[derive(Clone)]
struct PendingModelInstance {
    pub description_entity: Option<Entity>,
    pub instance_name: String,
}

impl Default for PendingModelInstance {
    fn default() -> Self {
        Self {
            description_entity: None,
            instance_name: "<Unnamed Instance>".to_string(),
        }
    }
}

#[derive(Clone)]
struct PendingModelDescription {
    pub name: String,
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
    pub scale: Scale,
    pub spawn_instance: bool,
    pub instance_name: String,
}

impl Default for PendingModelDescription {
    fn default() -> Self {
        Self {
            name: "<Unnamed Description>".to_string(),
            source: AssetSource::default(),
            recall_source: RecallAssetSource::default(),
            scale: Scale::default(),
            spawn_instance: true,
            instance_name: " <Unnamed Instance>".to_string(),
        }
    }
}
