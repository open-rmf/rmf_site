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
    inspector::{InspectAssetSourceComponent, InspectValue, SearchResult},
    site::{Category, Change, DefaultFile},
    widgets::{prelude::*, Inspect, InspectionPlugin},
    Icons, WorkspaceMarker,
};
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemParam},
    prelude::*,
};
use bevy_egui::egui::{ComboBox, Grid, ImageButton, Ui};
use rmf_site_format::{
    Affiliation, FloorMarker, Group, NameInSite, RecallAssetSource, Texture, TextureGroup,
    WallMarker,
};

#[derive(Default)]
pub struct InspectTexturePlugin {}

impl Plugin for InspectTexturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SearchForTexture>()
            .add_plugins(InspectionPlugin::<InspectTextureAffiliation>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectTextureAffiliation<'w, 's> {
    with_texture: Query<
        'w,
        's,
        (&'static Category, &'static Affiliation<Entity>),
        Or<(With<FloorMarker>, With<WallMarker>)>,
    >,
    texture_groups: Query<'w, 's, (&'static NameInSite, &'static Texture), With<Group>>,
    child_of: Query<'w, 's, &'static ChildOf>,
    sites: Query<'w, 's, &'static Children, With<WorkspaceMarker>>,
    icons: Res<'w, Icons>,
    search_for_texture: ResMut<'w, SearchForTexture>,
    commands: Commands<'w, 's>,
    change_affiliation: EventWriter<'w, Change<Affiliation<Entity>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectTextureAffiliation<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(selection, ui);
    }
}

impl<'w, 's> InspectTextureAffiliation<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((category, affiliation)) = self.with_texture.get(id) else {
            return;
        };
        let mut site = id;
        let children = loop {
            if let Ok(children) = self.sites.get(site) {
                break children;
            }

            if let Ok(child_of) = self.child_of.get(site) {
                site = child_of.parent();
            } else {
                return;
            }
        };
        let site = site;

        let search = &mut self.search_for_texture.0;

        let mut any_partial_matches = false;
        let mut result = SearchResult::NoMatch;
        for child in children {
            let Ok((name, _)) = self.texture_groups.get(*child) else {
                continue;
            };
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
                    .add(ImageButton::new(self.icons.search.egui()))
                    .on_hover_text("Search results for this text can be found below")
                    .clicked()
                {
                    info!("Use the drop-down box to choose a texture");
                }
            } else {
                ui.add(ImageButton::new(self.icons.empty.egui()))
                    .on_hover_text("No search results can be found for this text");
            }

            match result {
                SearchResult::Empty => {
                    if ui
                        .add(ImageButton::new(self.icons.hidden.egui()))
                        .on_hover_text("An empty string is not a good texture name")
                        .clicked()
                    {
                        warn!("You should not use an empty string as a texture name");
                    }
                }
                SearchResult::Current => {
                    if ui
                        .add(ImageButton::new(self.icons.selected.egui()))
                        .on_hover_text("This is the name of the currently selected texture")
                        .clicked()
                    {
                        info!("This texture is already selected");
                    }
                }
                SearchResult::NoMatch => {
                    if ui
                        .add(ImageButton::new(self.icons.add.egui()))
                        .on_hover_text(if affiliation.0.is_some() {
                            "Create a new copy of the current texture"
                        } else {
                            "Create a new texture"
                        })
                        .clicked()
                    {
                        let new_texture = if let Some((_, t)) = affiliation
                            .0
                            .map(|a| self.texture_groups.get(a).ok())
                            .flatten()
                        {
                            t.clone()
                        } else {
                            Texture::default()
                        };

                        let new_texture_group = self
                            .commands
                            .spawn(TextureGroup {
                                name: NameInSite(search.clone()),
                                texture: new_texture,
                                group: default(),
                            })
                            .insert(ChildOf(site))
                            .id();
                        self.change_affiliation
                            .write(Change::new(Affiliation(Some(new_texture_group)), id));
                    }
                }
                SearchResult::Match(group) => {
                    if ui
                        .add(ImageButton::new(self.icons.confirm.egui()))
                        .on_hover_text("Select this texture")
                        .clicked()
                    {
                        self.change_affiliation
                            .write(Change::new(Affiliation(Some(group)), id));
                    }
                }
                SearchResult::Conflict(text) => {
                    if ui
                        .add(ImageButton::new(self.icons.reject.egui()))
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

        let (current_texture_name, _current_texture) = if let Some(a) = affiliation.0 {
            self.texture_groups
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
                .add(ImageButton::new(self.icons.exit.egui()))
                .on_hover_text(format!("Remove this texture from the {}", category.label()))
                .clicked()
            {
                new_affiliation = Affiliation(None);
            }

            let mut clear_filter = false;
            ComboBox::from_id_salt("texture_affiliation")
                .selected_text(current_texture_name)
                .show_ui(ui, |ui| {
                    for child in children {
                        if affiliation.0.is_some_and(|a| a == *child) {
                            continue;
                        }

                        if let Ok((n, _)) = self.texture_groups.get(*child) {
                            if n.0.contains(&self.search_for_texture.0) {
                                let select_affiliation = Affiliation(Some(*child));
                                ui.selectable_value(&mut new_affiliation, select_affiliation, &n.0);
                            }
                        }
                    }

                    if !self.search_for_texture.0.is_empty() {
                        ui.selectable_value(&mut clear_filter, true, "more...");
                    }
                });

            if clear_filter {
                self.search_for_texture.0.clear();
            }
        });

        if new_affiliation != *affiliation {
            self.change_affiliation
                .write(Change::new(new_affiliation, id));
        }
        ui.add_space(10.0);
    }
}

#[derive(Resource, Default)]
pub struct SearchForTexture(pub String);

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
        if let Some(new_source) = InspectAssetSourceComponent::new(
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
                if let Some(new_width) = InspectValue::<f32>::new("Width", width)
                    .clamp_range(0.001..=std::f32::MAX)
                    .speed(0.01)
                    .tooltip("Texture width in meters")
                    .show(ui)
                {
                    new_texture.width = Some(new_width);
                }
                ui.end_row();
            }
            if let Some(height) = new_texture.height {
                if let Some(new_height) = InspectValue::<f32>::new("Height", height)
                    .clamp_range(0.001..=std::f32::MAX)
                    .speed(0.01)
                    .tooltip("Texture height in meters")
                    .show(ui)
                {
                    new_texture.height = Some(new_height);
                }
                ui.end_row();
            }
            if let Some(alpha) = new_texture.alpha {
                if let Some(new_alpha) = InspectValue::<f32>::new("Alpha", alpha)
                    .clamp_range(0.0..=1.0)
                    .speed(0.1)
                    .tooltip("Transparency (0 = transparent, 1 = opaque)")
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
