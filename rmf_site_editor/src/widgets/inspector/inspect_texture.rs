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
    site::DefaultFile,
    AppEvents,
    inspector::{InspectAssetSource, InspectValue},
    widgets::egui::RichText,
    WorkspaceMarker,
};
use bevy::prelude::*;
use bevy_egui::egui::{Grid, Ui};
use rmf_site_format::{
    RecallAssetSource, Texture, NameInSite, Group, Affiliation, FloorMarker,
    WallMarker,
};

pub struct InspectTextureAffiliationParams<'w, 's> {
    with_texture: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        Or<(With<FloorMarker>, With<WallMarker>)>,
    >,
    texture_groups: Query<'w, 's, (&'static NameInSite, &'static Texture), With<Group>>,
    parents: Query<'w, 's, &'static Parent>,
    sites: Query<'w, 's, &'static Children, With<WorkspaceMarker>>,
}

pub struct InspectTextureAffiliation<'a, 'w1, 'w2, 's1, 's2> {
    entity: Entity,
    params: &'a InspectTextureAffiliationParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectTextureAffiliation<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        entity: Entity,
        params: &'a InspectTextureAffiliationParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self { entity, params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        let Ok(affiliation) = self.params.with_texture.get(self.entity) else { return };
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

        ui.separator();
        ui.label("Texture");


    }
}

pub struct InspectTexture<'a> {
    texture: &'a Texture,
    default_file: Option<&'a DefaultFile>,
}

impl<'a> InspectTexture<'a> {
    pub fn new(
        texture: &'a Texture,
        default_file: Option<&'a DefaultFile>,
    ) -> Self {
        Self { texture, default_file }
    }

    pub fn show(self, ui: &mut Ui) -> Option<Texture> {
        let mut new_texture = self.texture.clone();

        ui.label(RichText::new("Texture Properties").size(18.0));
        // TODO(luca) recall
        if let Some(new_source) =
            InspectAssetSource::new(
                &new_texture.source,
                &RecallAssetSource::default(),
                self.default_file,
            ).show(ui)
        {
            new_texture.source = new_source;
        }
        ui.add_space(10.0);
        Grid::new("texture_properties").show(ui, |ui| {
            if let Some(width) = new_texture.width {
                if let Some(new_width) = InspectValue::<f32>::new(String::from("Width"), width)
                    .clamp_range(0.001..=std::f32::MAX)
                    .speed(0.1)
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
                    .speed(0.1)
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

        if new_texture.width != self.texture.width
            || new_texture.height != self.texture.height
            || new_texture.alpha != self.texture.alpha
            || new_texture.source != self.texture.source
        {
            Some(new_texture)
        } else {
            None
        }
    }
}
