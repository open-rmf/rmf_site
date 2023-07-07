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
use crate::site::{AssetSource, FuelClient, Model, UpdateFuelCache};
use crate::AppEvents;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, ComboBox, RichText, ScrollArea, Ui};
use gz_fuel::{FuelClient as GzFuelClient, FuelModel};

/// Filters applied to models in the fuel list
pub struct ShowAssetFilters {
    pub owner: Option<String>,
    pub recall_owner: Option<String>,
    pub tag: Option<String>,
    pub recall_tag: Option<String>,
}

/// Used to signals whether to show or hide the left side panel with the asset gallery
#[derive(Resource)]
pub struct AssetGalleryStatus {
    pub show: bool,
    pub selected: Option<FuelModel>,
    pub cached_owners: Option<Vec<String>>,
    pub cached_tags: Option<Vec<String>>,
    pub filters: ShowAssetFilters,
    pub fetching_cache: bool,
}

impl Default for ShowAssetFilters {
    fn default() -> Self {
        Self {
            owner: Some("OpenRobotics".into()),
            recall_owner: None,
            tag: None,
            recall_tag: None,
        }
    }
}

impl Default for AssetGalleryStatus {
    fn default() -> Self {
        Self {
            show: true,
            selected: None,
            cached_owners: None,
            cached_tags: None,
            filters: Default::default(),
            fetching_cache: false,
        }
    }
}

#[derive(SystemParam)]
pub struct NewModelParams<'w, 's> {
    pub fuel_client: ResMut<'w, FuelClient>,
    // TODO(luca) refactor to see whether we need
    pub asset_gallery_status: ResMut<'w, AssetGalleryStatus>,
    pub model_preview_camera: Res<'w, ModelPreviewCamera>,
    pub update_cache: EventWriter<'w, 's, UpdateFuelCache>,
}

pub struct NewModel<'a, 'w, 's> {
    events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> NewModel<'a, 'w, 's> {
    pub fn new(events: &'a mut AppEvents<'w, 's>) -> Self {
        Self { events }
    }

    pub fn show(self, ui: &mut Ui) {
        let fuel_client = &mut self.events.new_model.fuel_client;
        let gallery_status = &mut self.events.new_model.asset_gallery_status;
        ui.label(RichText::new("Asset Gallery").size(18.0));
        ui.add_space(10.0);
        match &fuel_client.models {
            Some(models) => {
                // Note, unwraps here are safe because the client will return None only if models
                // are not populated which will not happen in this match branch
                let owner_filter = gallery_status.filters.owner.clone();
                let mut owner_filter_enabled = owner_filter.is_some();
                ui.label(RichText::new("Filters").size(14.0));
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut owner_filter_enabled, "Owners");
                    gallery_status.filters.owner = match owner_filter_enabled {
                        true => {
                            let owners = gallery_status
                                .cached_owners
                                .clone()
                                .or_else(|| fuel_client.get_owners())
                                .unwrap();
                            let mut selected = match &owner_filter {
                                Some(s) => s.clone(),
                                None => gallery_status
                                    .filters
                                    .recall_owner
                                    .clone()
                                    .unwrap_or(owners[0].clone()),
                            };
                            ComboBox::from_id_source("Asset Owner Filter")
                                .selected_text(selected.clone())
                                .show_ui(ui, |ui| {
                                    for owner in owners.into_iter() {
                                        ui.selectable_value(&mut selected, owner.clone(), owner);
                                    }
                                    ui.end_row();
                                });
                            gallery_status.filters.recall_owner = Some(selected.clone());
                            Some(selected)
                        }
                        false => None,
                    };
                });
                ui.add_space(5.0);
                // TODO(luca) should we cache the models by owner result to avoid calling at every
                // frame?
                let mut models = match &owner_filter {
                    Some(owner) => fuel_client.as_ref().models_by_owner(None, &owner).unwrap(),
                    None => models.clone(),
                };

                let tag_filter = gallery_status.filters.tag.clone();
                let mut tag_filter_enabled = tag_filter.is_some();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut tag_filter_enabled, "Tags");
                    gallery_status.filters.tag = match tag_filter_enabled {
                        true => {
                            let tags = gallery_status
                                .cached_tags
                                .clone()
                                .or_else(|| fuel_client.get_tags())
                                .unwrap();
                            let mut selected = match &tag_filter {
                                Some(s) => s.clone(),
                                None => gallery_status
                                    .filters
                                    .recall_tag
                                    .clone()
                                    .unwrap_or(tags[0].clone()),
                            };
                            ComboBox::from_id_source("Asset Tag Filter")
                                .selected_text(selected.clone())
                                .show_ui(ui, |ui| {
                                    for tag in tags.into_iter() {
                                        ui.selectable_value(&mut selected, tag.clone(), tag);
                                    }
                                    ui.end_row();
                                });
                            gallery_status.filters.recall_tag = Some(selected.clone());
                            Some(selected)
                        }
                        false => None,
                    };
                });

                if let Some(tag) = &tag_filter {
                    models = fuel_client
                        .as_ref()
                        .models_by_tag(Some(&models), &tag)
                        .unwrap();
                }
                ui.add_space(10.0);

                ui.label(RichText::new("Models").size(14.0));
                ui.add_space(5.0);
                // Show models
                let mut new_selected = None;
                ScrollArea::vertical()
                    .max_height(350.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for model in models {
                            let sel = gallery_status
                                .selected
                                .as_ref()
                                .is_some_and(|s| *s == model);
                            if ui.selectable_label(sel, &model.name).clicked() {
                                new_selected = Some(model);
                            }
                        }
                    });
                ui.add_space(10.0);

                ui.image(
                    self.events.new_model.model_preview_camera.egui_handle,
                    bevy_egui::egui::Vec2::new(320.0, 240.0),
                );
                ui.add_space(10.0);

                if gallery_status.selected != new_selected {
                    if let Some(selected) = new_selected {
                        // Set the model preview source to what is selected
                        let model_entity = self.events.new_model.model_preview_camera.model_entity;
                        let model = Model {
                            source: AssetSource::Remote(
                                selected.owner.clone() + "/" + &selected.name + "/model.sdf",
                            ),
                            ..default()
                        };
                        self.events.commands.entity(model_entity).insert(model);
                        gallery_status.selected = Some(selected);
                    }
                }

                if let Some(selected) = &gallery_status.selected {
                    if ui.button("Spawn model").clicked() {
                        let model = Model {
                            source: AssetSource::Remote(
                                selected.owner.clone() + "/" + &selected.name + "/model.sdf",
                            ),
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
            }
        }
        ui.add_space(10.0);
        if gallery_status.fetching_cache == true {
            ui.label("Updating model cache...");
        } else {
            if ui.add(Button::new("Update model cache")).clicked() {
                self.events.new_model.update_cache.send(UpdateFuelCache);
            }
        }
        if ui.add(Button::new("Close")).clicked() {
            self.events.new_model.asset_gallery_status.show = false;
        }
    }
}
