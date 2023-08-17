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

use crate::{
    inspector::{InspectAssetSource, InspectValue, SearchResult},
    site::{Category, Change, DefaultFile},
    widgets::egui::RichText,
    AppEvents, Icons, WorkspaceMarker,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, Grid, ImageButton, Ui};
use rmf_site_format::{
    Affiliation, FloorMarker, Group, NameInSite, RecallAssetSource, Texture, TextureGroup,
    WallMarker,
};

#[derive(Resource, Default)]
pub struct SearchForTexture(pub String);

#[derive(SystemParam)]
pub struct InspectTextureAffiliationParams<'w, 's> {
    with_texture: Query<
        'w,
        's,
        (&'static Category, &'static Affiliation<Entity>),
        Or<(With<FloorMarker>, With<WallMarker>)>,
    >,
    texture_groups: Query<'w, 's, (&'static NameInSite, &'static Texture), With<Group>>,
    parents: Query<'w, 's, &'static Parent>,
    sites: Query<'w, 's, &'static Children, With<WorkspaceMarker>>,
    icons: Res<'w, Icons>,
}

pub struct InspectTextureAffiliation<'a, 'w1, 'w2, 's1, 's2> {
    entity: Entity,
    default_file: Option<&'a DefaultFile>,
    params: &'a InspectTextureAffiliationParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectTextureAffiliation<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        entity: Entity,
        default_file: Option<&'a DefaultFile>,
        params: &'a InspectTextureAffiliationParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            entity,
            default_file,
            params,
            events,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let Ok((category, affiliation)) = self.params.with_texture.get(self.entity) else { return };
        let mut site = self.entity;
        let children = loop {
            if let Ok(children) = self.params.sites.get(site) {
                break children;
            }

            if let Ok(parent) = self.params.parents.get(site) {
                site = parent.get();
            } else {
                return;
            }
        };
        let site = site;

        let search = &mut self.events.change_more.search_for_texture.0;

        let mut any_partial_matches = false;
        let mut result = SearchResult::NoMatch;
        for child in children {
            let Ok((name, _)) = self.params.texture_groups.get(*child) else { continue };
            if name.0.contains(&*search) {
                any_partial_matches = true;
            }

            if name.0 == *search {
                result.consider(*child);
            }
        }
        let any_partial_matches = any_partial_matches;

        if search.is_empty() {
            result = SearchResult::Empty;
        }

        if let (SearchResult::Match(e), Some(current)) = (&result, &affiliation.0) {
            if *e == *current {
                result = SearchResult::Current;
            }
        }

        ui.separator();
        ui.label("Texture");
        ui.horizontal(|ui| {
            if any_partial_matches {
                if ui
                    .add(ImageButton::new(
                        self.params.icons.search.egui(),
                        [18., 18.],
                    ))
                    .on_hover_text("Search results for this text can be found below")
                    .clicked()
                {
                    info!("Use the drop-down box to choose a texture");
                }
            } else {
                ui.add(ImageButton::new(self.params.icons.empty.egui(), [18., 18.]))
                    .on_hover_text("No search results can be found for this text");
            }

            match result {
                SearchResult::Empty => {
                    if ui
                        .add(ImageButton::new(
                            self.params.icons.hidden.egui(),
                            [18., 18.],
                        ))
                        .on_hover_text("An empty string is not a good texture name")
                        .clicked()
                    {
                        warn!("You should not use an empty string as a texture name");
                    }
                }
                SearchResult::Current => {
                    if ui
                        .add(ImageButton::new(
                            self.params.icons.selected.egui(),
                            [18., 18.],
                        ))
                        .on_hover_text("This is the name of the currently selected texture")
                        .clicked()
                    {
                        info!("This texture is already selected");
                    }
                }
                SearchResult::NoMatch => {
                    if ui
                        .add(ImageButton::new(self.params.icons.add.egui(), [18., 18.]))
                        .on_hover_text(if affiliation.0.is_some() {
                            "Create a new copy of the current texture"
                        } else {
                            "Create a new texture"
                        })
                        .clicked()
                    {
                        let new_texture = if let Some((_, t)) = affiliation
                            .0
                            .map(|a| self.params.texture_groups.get(a).ok())
                            .flatten()
                        {
                            t.clone()
                        } else {
                            Texture::default()
                        };

                        let new_texture_group = self
                            .events
                            .commands
                            .spawn(TextureGroup {
                                name: NameInSite(search.clone()),
                                texture: new_texture,
                                group: default(),
                            })
                            .set_parent(site)
                            .id();
                        self.events.change_more.affiliation.send(Change::new(
                            Affiliation(Some(new_texture_group)),
                            self.entity,
                        ));
                    }
                }
                SearchResult::Match(group) => {
                    if ui
                        .add(ImageButton::new(
                            self.params.icons.confirm.egui(),
                            [18., 18.],
                        ))
                        .on_hover_text("Select this texture")
                        .clicked()
                    {
                        self.events
                            .change_more
                            .affiliation
                            .send(Change::new(Affiliation(Some(group)), self.entity));
                    }
                }
                SearchResult::Conflict(text) => {
                    if ui
                        .add(ImageButton::new(
                            self.params.icons.reject.egui(),
                            [18., 18.],
                        ))
                        .on_hover_text(text)
                        .clicked()
                    {
                        warn!("Cannot set {search} as the texture: {text}");
                    }
                }
            }

            ui.text_edit_singleline(search)
                .on_hover_text("Search for or create a new texture");
        });

        let (current_texture_name, current_texture) = if let Some(a) = affiliation.0 {
            self.params
                .texture_groups
                .get(a)
                .ok()
                .map(|(n, t)| (n.0.as_str(), Some((a, t))))
        } else {
            None
        }
        .unwrap_or(("<none>", None));

        let mut new_affiliation = affiliation.clone();
        ui.horizontal(|ui| {
            if ui
                .add(ImageButton::new(self.params.icons.exit.egui(), [18., 18.]))
                .on_hover_text(format!("Remove this texture from the {}", category.label()))
                .clicked()
            {
                new_affiliation = Affiliation(None);
            }

            ComboBox::from_id_source("texture_affiliation")
                .selected_text(current_texture_name)
                .show_ui(ui, |ui| {
                    for child in children {
                        if affiliation.0.is_some_and(|a| a == *child) {
                            continue;
                        }

                        if let Ok((n, _)) = self.params.texture_groups.get(*child) {
                            if n.0.contains(&*search) {
                                let select_affiliation = Affiliation(Some(*child));
                                ui.selectable_value(&mut new_affiliation, select_affiliation, &n.0);
                            }
                        }
                    }
                });
        });

        if new_affiliation != *affiliation {
            self.events
                .change_more
                .affiliation
                .send(Change::new(new_affiliation, self.entity));
        }

        if let Some((group, texture)) = current_texture {
            ui.add_space(5.0);
            ui.label(RichText::new(format!("Properties of [{current_texture_name}]")).size(18.0));
            if let Some(new_texture) = InspectTexture::new(texture, self.default_file).show(ui) {
                self.events
                    .change_more
                    .texture
                    .send(Change::new(new_texture, group))
            }
        }
        ui.add_space(10.0);
    }
}

pub struct InspectTexture<'a> {
    texture: &'a Texture,
    default_file: Option<&'a DefaultFile>,
}

impl<'a> InspectTexture<'a> {
    pub fn new(texture: &'a Texture, default_file: Option<&'a DefaultFile>) -> Self {
        Self {
            texture,
            default_file,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Option<Texture> {
        let mut new_texture = self.texture.clone();

        // TODO(luca) recall
        if let Some(new_source) = InspectAssetSource::new(
            &new_texture.source,
            &RecallAssetSource::default(),
            self.default_file,
        )
        .show(ui)
        {
            new_texture.source = new_source;
        }
        ui.add_space(10.0);
        Grid::new("texture_properties").show(ui, |ui| {
            if let Some(width) = new_texture.width {
                if let Some(new_width) = InspectValue::<f32>::new(String::from("Width"), width)
                    .clamp_range(0.001..=std::f32::MAX)
                    .speed(0.01)
                    .tooltip("Texture width in meters".to_string())
                    .show(ui)
                {
                    new_texture.width = Some(new_width);
                }
                ui.end_row();
            }
            if let Some(height) = new_texture.height {
                if let Some(new_height) = InspectValue::<f32>::new(String::from("Height"), height)
                    .clamp_range(0.001..=std::f32::MAX)
                    .speed(0.01)
                    .tooltip("Texture height in meters".to_string())
                    .show(ui)
                {
                    new_texture.height = Some(new_height);
                }
                ui.end_row();
            }
            if let Some(alpha) = new_texture.alpha {
                if let Some(new_alpha) = InspectValue::<f32>::new(String::from("Alpha"), alpha)
                    .clamp_range(0.0..=1.0)
                    .speed(0.1)
                    .tooltip("Transparency (0 = transparent, 1 = opaque)".to_string())
                    .show(ui)
                {
                    new_texture.alpha = Some(new_alpha);
                }
                ui.end_row();
            }
        });

        if new_texture != *self.texture {
            Some(new_texture)
        } else {
            None
        }
    }
}
