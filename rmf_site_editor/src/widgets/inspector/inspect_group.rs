/*
 * Copyright (C) 2023 Open Source Robotics Foundation
 *
 * Licensed under the Apahe License, Version 2.0 (the "License");
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
    site::{Affiliation, Change, DefaultFile, Group, Members, SiteID, Texture},
    widgets::{
        inspector::{InspectTexture, SelectionWidget},
        AppEvents,
    },
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, RichText, Ui};

#[derive(SystemParam)]
pub struct InspectGroupParams<'w, 's> {
    pub is_group: Query<'w, 's, (), With<Group>>,
    pub affiliation: Query<'w, 's, &'static Affiliation<Entity>>,
    pub textures: Query<'w, 's, &'static Texture>,
    pub members: Query<'w, 's, &'static Members>,
    pub site_id: Query<'w, 's, &'static SiteID>,
    pub icons: Res<'w, Icons>,
}

pub struct InspectGroup<'a, 'w1, 'w2, 's1, 's2> {
    group: Entity,
    selection: Entity,
    default_file: Option<&'a DefaultFile>,
    params: &'a InspectGroupParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectGroup<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        group: Entity,
        selection: Entity,
        default_file: Option<&'a DefaultFile>,
        params: &'a InspectGroupParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            group,
            selection,
            default_file,
            params,
            events,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        if let Ok(texture) = self.params.textures.get(self.group) {
            ui.label(RichText::new("Texture Properties").size(18.0));
            if let Some(new_texture) = InspectTexture::new(texture, self.default_file).show(ui) {
                self.events
                    .change
                    .texture
                    .send(Change::new(new_texture, self.group));
            }
            ui.add_space(10.0);
        }
        if let Ok(members) = self.params.members.get(self.group) {
            CollapsingHeader::new("Members").show(ui, |ui| {
                for member in members.iter() {
                    let site_id = self.params.site_id.get(self.group).ok().cloned();
                    SelectionWidget::new(*member, site_id, &self.params.icons, self.events)
                        .as_selected(self.selection == *member)
                        .show(ui);
                }
            });
        }
    }
}
