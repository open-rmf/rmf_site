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

use crate::interaction::{ChangeMode, ModelPreviewCamera, SelectAnchor3D};
use crate::site::{AssetSource, Model};
use crate::AppEvents;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{
    widgets::Image as EguiImage, Button, Color32, ComboBox, Context, Frame, Pos2, Rect, ScrollArea,
    Ui, Window,
};
use bevy_egui::EguiContext;
use gz_fuel::{FuelClient as GzFuelClient, FuelModel};

#[derive(Resource, Default, Deref, DerefMut)]
pub struct FuelClient(GzFuelClient);

/// Filters applied to models in the fuel list
pub struct ShowAssetFilters {
    pub owner: Option<String>,
}

/// Used to signals whether to show or hide the left side panel with the asset gallery
#[derive(Resource)]
pub struct AssetGalleryStatus {
    pub show: bool,
    pub selected: Option<FuelModel>,
    pub filters: ShowAssetFilters,
}

impl Default for ShowAssetFilters {
    fn default() -> Self {
        Self {
            owner: Some("OpenRobotics".into()),
        }
    }
}

impl Default for AssetGalleryStatus {
    fn default() -> Self {
        Self {
            show: true,
            selected: None,
            filters: Default::default(),
        }
    }
}

#[derive(SystemParam)]
pub struct NewModelParams<'w, 's> {
    pub fuel_client: ResMut<'w, FuelClient>,
    // TODO(luca) refactor to see whether we need
    pub asset_gallery_status: ResMut<'w, AssetGalleryStatus>,
    pub model_preview_camera: Res<'w, ModelPreviewCamera>,
    pub image_assets: Res<'w, Assets<Image>>,
    _ignore: Query<'w, 's, ()>,
}

pub struct NewModel<'a, 'w, 's> {
    events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> NewModel<'a, 'w, 's> {
    pub fn new(events: &'a mut AppEvents<'w, 's>) -> Self {
        Self { events }
    }

    pub fn show(mut self, ui: &mut Ui) {
        let fuel_client = &mut self.events.new_model.fuel_client;
        let owners = fuel_client.get_owners_cached().unwrap();
        match &fuel_client.models {
            Some(models) => {
                // TODO(luca) remove unwrap
                let mut owner_filter = self
                    .events
                    .new_model
                    .asset_gallery_status
                    .filters
                    .owner
                    .clone();
                let mut owner_filter_enabled = owner_filter.is_some();
                ui.checkbox(&mut owner_filter_enabled, "Owners");
                match owner_filter_enabled {
                    true => {
                        let mut selected = match owner_filter {
                            Some(s) => s,
                            None => owners[0].clone(),
                        };
                        ComboBox::from_id_source("Asset Owner Filter")
                            .selected_text(selected.clone())
                            .show_ui(ui, |ui| {
                                for owner in owners.into_iter() {
                                    ui.selectable_value(&mut selected, owner.clone(), owner);
                                }
                                ui.end_row();
                            });
                        owner_filter = Some(selected);
                    }
                    false => {
                        owner_filter = None;
                    }
                }
                let models = match &owner_filter {
                    Some(owner) => fuel_client.as_ref().models_by_owner(&owner).unwrap(),
                    None => models.clone(),
                };
                // Show models
                ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                    for model in models {
                        let sel = self
                            .events
                            .new_model
                            .asset_gallery_status
                            .selected
                            .as_ref()
                            .is_some_and(|s| s.name == model.name);
                        if ui.selectable_label(sel, &model.name).clicked() {
                            self.events.new_model.asset_gallery_status.selected = Some(model);
                        }
                    }
                });
                // Set the model source to what is selected
                if let Some(selected) = &self.events.new_model.asset_gallery_status.selected {
                    let model_entity = self.events.new_model.model_preview_camera.model_entity;
                    let model = Model {
                        source: AssetSource::Remote(
                            selected.owner.clone() + "/" + &selected.name + "/model.sdf",
                        ),
                        ..default()
                    };
                    self.events.commands.entity(model_entity).insert(model);
                }

                ui.image(
                    self.events.new_model.model_preview_camera.egui_handle,
                    bevy_egui::egui::Vec2::new(320.0, 240.0),
                );

                if owner_filter != self.events.new_model.asset_gallery_status.filters.owner {
                    self.events.new_model.asset_gallery_status.filters.owner = owner_filter;
                }
                if let Some(selected) = &self.events.new_model.asset_gallery_status.selected {
                    if ui.button("Spawn model").clicked() {
                        let source =
                            AssetSource::Search(selected.owner.clone() + "/" + &selected.name);
                        let model = Model {
                            source: source.clone(),
                            ..default()
                        };
                        self.events.request.change_mode.send(ChangeMode::To(
                            SelectAnchor3D::create_new_point().for_model(model).into(),
                        ));
                    }
                }
            }
            None => {
                ui.label("No models found");
                if ui.add(Button::new("Fetch models")).clicked() {
                    // TODO(luca) make this async to avoid blocking the whole application
                    fuel_client.update_cache_blocking();
                    /*
                    AsyncComputeTaskPool::get().spawn(
                    async move {fuel_client.update_cache()})
                        .detach();
                    */
                }
            }
        }
        if ui.add(Button::new("Close")).clicked() {
            self.events.new_model.asset_gallery_status.show = false;
        }
    }
}
