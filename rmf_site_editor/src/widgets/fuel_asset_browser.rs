/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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
    interaction::ModelPreviewCamera,
    site::{
        AssetSource, Category, FuelClient, ModelDescriptionBundle, ModelLoader, ModelProperty,
        NameInSite, SetFuelApiKey, UpdateFuelCache,
    },
    widgets::prelude::*,
    CurrentWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, Button, ComboBox, ImageSource, RichText, ScrollArea, Ui, Window};
use gz_fuel::FuelModel;

/// Add a [`FuelAssetBrowser`] widget to your application.
pub struct FuelAssetBrowserPlugin;

impl Plugin for FuelAssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetGalleryStatus>();
        let panel = PanelWidget::new(fuel_asset_browser_panel, &mut app.world);
        let widget = Widget::new::<FuelAssetBrowser>(&mut app.world);
        app.world.spawn((panel, widget));
    }
}

/// Filters applied to models in the fuel list
#[derive(Default)]
pub struct ShowAssetFilters {
    pub owner: Option<String>,
    pub recall_owner: Option<String>,
    pub tag: Option<String>,
    pub recall_tag: Option<String>,
    pub private: Option<bool>,
    pub recall_private: Option<bool>,
}

/// Used to indicate whether to show or hide the [`FuelAssetBrowser`].
#[derive(Resource, Default)]
pub struct AssetGalleryStatus {
    pub show: bool,
    pub selected: Option<FuelModel>,
    pub cached_owners: Option<Vec<String>>,
    pub cached_tags: Option<Vec<String>>,
    pub filters: ShowAssetFilters,
    pub proposed_api_key: String,
    pub fetching_cache: bool,
    pub show_api_window: bool,
}

/// A widget for browsing models that can be downloaded from fuel.
///
/// This is part of the [`StandardUiPlugin`][1]. If you are not using the
/// `StandardUiPlugin` then it is recommended that you use the
/// [`FuelAssetBrowserPlugin`] to add this to the editor.
///
/// [1]: crate::widgets::StandardUiPlugin
#[derive(SystemParam)]
pub struct FuelAssetBrowser<'w, 's> {
    fuel_client: ResMut<'w, FuelClient>,
    // TODO(luca) refactor to see whether we need
    asset_gallery_status: ResMut<'w, AssetGalleryStatus>,
    model_preview_camera: Res<'w, ModelPreviewCamera>,
    update_cache: EventWriter<'w, UpdateFuelCache>,
    set_api_key: EventWriter<'w, SetFuelApiKey>,
    commands: Commands<'w, 's>,
    current_workspace: Res<'w, CurrentWorkspace>,
    model_loader: ModelLoader<'w, 's>,
}

fn fuel_asset_browser_panel(In(input): In<PanelWidgetInput>, world: &mut World) {
    if world.resource::<AssetGalleryStatus>().show {
        egui::SidePanel::left("asset_gallery")
            .resizable(true)
            .min_width(320.0)
            .show(&input.context, |ui| {
                if let Err(err) = world.try_show(input.id, ui) {
                    error!("Unable to display asset gallery: {err:?}");
                }
            });
    }
}

impl<'w, 's> WidgetSystem for FuelAssetBrowser<'w, 's> {
    fn show(_: (), ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        params.show_widget(ui);
    }
}

impl<'w, 's> FuelAssetBrowser<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let fuel_client = &mut self.fuel_client;
        let gallery_status = &mut self.asset_gallery_status;
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

                let private_filter = gallery_status.filters.private.clone();
                let mut private_filter_enabled = private_filter.is_some();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut private_filter_enabled, "Private");
                    gallery_status.filters.private = match private_filter_enabled {
                        true => {
                            let mut selected = match &private_filter {
                                Some(s) => s.clone(),
                                None => gallery_status.filters.recall_private.unwrap_or(false),
                            };
                            ComboBox::from_id_source("Asset Private Filter")
                                .selected_text(selected.to_string())
                                .show_ui(ui, |ui| {
                                    for private in [true, false].into_iter() {
                                        ui.selectable_value(
                                            &mut selected,
                                            private,
                                            private.to_string(),
                                        );
                                    }
                                    ui.end_row();
                                });
                            gallery_status.filters.recall_private = Some(selected);
                            Some(selected)
                        }
                        false => None,
                    };
                });

                ui.add_space(10.0);

                // TODO(luca) should we cache the models by filters result to avoid calling at every
                // frame?
                let models = models
                    .iter()
                    .filter(|m| {
                        owner_filter.is_none()
                            | owner_filter.as_ref().is_some_and(|owner| m.owner == *owner)
                    })
                    .filter(|m| {
                        private_filter.is_none()
                            | private_filter
                                .as_ref()
                                .is_some_and(|private| m.private == *private)
                    })
                    .filter(|m| {
                        tag_filter.is_none()
                            | tag_filter.as_ref().is_some_and(|tag| m.tags.contains(&tag))
                    });

                ui.label(RichText::new("Models").size(14.0));
                ui.add_space(5.0);
                // Show models
                let mut new_selected = None;
                ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for model in models {
                            let sel = gallery_status.selected.as_ref().is_some_and(|s| s == model);
                            if ui.selectable_label(sel, &model.name).clicked() {
                                new_selected = Some(model);
                            }
                        }
                    });
                ui.add_space(10.0);

                ui.image(ImageSource::Texture(
                    (self.model_preview_camera.egui_handle, [320.0, 240.0].into()).into(),
                ));
                ui.add_space(10.0);

                if gallery_status.selected.as_ref() != new_selected {
                    if let Some(selected) = new_selected {
                        // Set the model preview source to what is selected
                        let model_entity = self.model_preview_camera.model_entity;
                        let source = AssetSource::Remote(
                            selected.owner.clone() + "/" + &selected.name + "/model.sdf",
                        );
                        self.model_loader.update_asset_source(model_entity, source);
                        gallery_status.selected = Some(selected.clone());
                    }
                }

                if let Some(selected) = &gallery_status.selected {
                    if ui.button("Load as Description").clicked() {
                        if let Some(site_entity) = self.current_workspace.root {
                            let model_description: ModelDescriptionBundle =
                                ModelDescriptionBundle {
                                    name: NameInSite(selected.owner.clone() + "/" + &selected.name),
                                    source: ModelProperty(AssetSource::Remote(
                                        selected.owner.clone()
                                            + "/"
                                            + &selected.name
                                            + "/model.sdf",
                                    )),
                                    ..Default::default()
                                };
                            self.commands
                                .spawn(model_description)
                                .insert(Category::ModelDescription)
                                .set_parent(site_entity);
                        }
                    }
                }
            }
            None => {
                ui.label("No models found");
            }
        }
        ui.add_space(10.0);
        if gallery_status.show_api_window {
            Window::new("API Key").show(ui.ctx(), |ui| {
                ui.label("Key");
                ui.text_edit_singleline(&mut gallery_status.proposed_api_key);
                if ui.add(Button::new("Save")).clicked() {
                    // Take it to avoid leaking the information in the dialog
                    self.set_api_key
                        .send(SetFuelApiKey(gallery_status.proposed_api_key.clone()));
                    fuel_client.token = Some(std::mem::take(&mut gallery_status.proposed_api_key));
                    gallery_status.show_api_window = false;
                } else if ui.add(Button::new("Close")).clicked() {
                    gallery_status.proposed_api_key = Default::default();
                    gallery_status.show_api_window = false;
                }
            });
        }
        if ui.add(Button::new("Set API key")).clicked() {
            gallery_status.show_api_window = true;
        }
        if gallery_status.fetching_cache {
            ui.label("Updating model cache...");
        } else {
            if ui.add(Button::new("Update model cache")).clicked() {
                self.update_cache.send(UpdateFuelCache);
            }
        }
        if ui.add(Button::new("Close")).clicked() {
            gallery_status.show = false;
        }
    }
}
