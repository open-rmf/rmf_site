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
    site::{AssetSource, Category, DefaultFile, DrawingBundle, Recall, RecallAssetSource, Scale},
    widgets::{prelude::*, AssetGalleryStatus},
    AppState, CurrentWorkspace,
};
use bevy::{audio::Source, ecs::system::SystemParam, prelude::*};
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
    change_mode: EventWriter<'w, ChangeMode>,
    current_workspace: Res<'w, CurrentWorkspace>,
    model_descriptions: Query<
        'w,
        's,
        (Entity, &'static NameInSite, Option<&'static SiteID>),
        (With<ModelMarker>, With<Group>),
    >,
    creation_data: ResMut<'w, CreationData>,
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
            CreationData::ModelInstance(_) => self.show_create_model_instance(ui),
        }
    }

    pub fn show_create_site_objects(&mut self, ui: &mut Ui) {
        match self.app_state.get() {
            AppState::SiteEditor => {
                Grid::new("create_site_objects")
                    .num_columns(3)
                    .show(ui, |ui| {
                        if ui.button("↔ Lane").clicked() {
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor::create_new_edge_sequence().for_lane().into(),
                            ));
                        }

                        if ui.button("📌 Location").clicked() {
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor::create_new_point().for_location().into(),
                            ));
                        }

                        if ui.button("■ Wall").clicked() {
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor::create_new_edge_sequence().for_wall().into(),
                            ));
                        }

                        ui.end_row();

                        if ui.button("🚪 Door").clicked() {
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor::create_one_new_edge().for_door().into(),
                            ));
                        }

                        if ui.button("⬍ Lift").clicked() {
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor::create_one_new_edge().for_lift().into(),
                            ));
                        }

                        if ui.button("✏ Floor").clicked() {
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor::create_new_path().for_floor().into(),
                            ));
                        }

                        ui.end_row();

                        if ui.button("☉ Fiducial").clicked() {
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor::create_new_point().for_site_fiducial().into(),
                            ));
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
                                            self.change_mode.send(ChangeMode::To(
                                                SelectAnchor3D::create_new_point()
                                                    .for_model_instance(model_instance)
                                                    .into(),
                                            ));
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
                            if ui.button("Spawn visual").clicked() {
                                let workcell_model = WorkcellModel {
                                    geometry: Geometry::Mesh {
                                        source: pending_model.source.clone(),
                                        scale: Some(*pending_model.scale),
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
                                        source: pending_model.source.clone(),
                                        scale: Some(*pending_model.scale),
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
            }
        }
    }

    pub fn show_create_model_instance(&mut self, ui: &mut Ui) {
        match self.app_state.get() {
            AppState::MainMenu | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {}
            AppState::SiteEditor | AppState::WorkcellEditor => {
                let pending_model_instance = match *self.creation_data {
                    CreationData::ModelInstance(ref mut pending_model) => pending_model,
                    _ => return,
                };

                let mut style = (*ui.ctx().style()).clone();
                style.wrap = Some(false);
                ui.horizontal(|ui| {
                    ui.ctx().set_style(style);
                    ui.label("Description");
                    let selected_text = pending_model_instance
                        .description_entity
                        .and_then(|description_entity| {
                            self.model_descriptions
                                .get(description_entity)
                                .ok()
                                .map(|(_, name, _)| name.0.clone())
                        })
                        .unwrap_or_else(|| "Select Description".to_string());
                    ComboBox::from_id_source("select_instance_to_spawn")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for (entity, name, site_id) in self.model_descriptions.iter() {
                                if ui
                                    .selectable_label(
                                        pending_model_instance.description_entity == Some(entity),
                                        format!(
                                            "#{} {}",
                                            site_id
                                                .map(|s| s.to_string())
                                                .unwrap_or("*".to_string()),
                                            name.0
                                        ),
                                    )
                                    .clicked()
                                {
                                    pending_model_instance.description_entity = Some(entity);
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(pending_model_instance.description_entity.is_some(), |ui| {
                        if ui.button("➕ Spawn").clicked() {
                            let model_instance: ModelInstance<Entity> = ModelInstance {
                                name: NameInSite(pending_model_instance.instance_name.clone()),
                                description: Affiliation(Some(
                                    pending_model_instance.description_entity.unwrap(),
                                )),
                                ..Default::default()
                            };
                            self.change_mode.send(ChangeMode::To(
                                SelectAnchor3D::create_new_point()
                                    .for_model_instance(model_instance)
                                    .into(),
                            ));
                        }
                    });
                    ui.text_edit_singleline(&mut pending_model_instance.instance_name);
                });
            }
        }
    }
}

#[derive(Resource, Clone, Default)]
enum CreationData {
    #[default]
    SiteObject,
    Drawing(PendingDrawing),
    ModelDescription(PendingModelDescription),
    ModelInstance(PendingModelInstance),
}

impl CreationData {
    fn to_string(&self) -> &str {
        match self {
            Self::SiteObject => "Site Object",
            Self::Drawing(_) => "Drawing",
            Self::ModelDescription(_) => "Model Description",
            Self::ModelInstance(_) => "Model Instance",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "Site Object" => Self::SiteObject,
            "Drawing" => Self::Drawing(PendingDrawing::default()),
            "Model Description" => Self::ModelDescription(PendingModelDescription::default()),
            "Model Instance" => Self::ModelInstance(PendingModelInstance::default()),
            _ => Self::SiteObject,
        }
    }

    fn string_values() -> Vec<&'static str> {
        vec![
            "Site Object",
            "Drawing",
            "Model Description",
            "Model Instance",
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
