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
    site::{Affiliation, Change, DefaultFile, Group, Members, ModelMarker, NameInSite, Texture},
    widgets::{inspector::InspectTexture, prelude::*, Inspect, SelectorWidget},
    CurrentWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, RichText, Ui};

#[derive(SystemParam)]
pub struct InspectGroup<'w, 's> {
    is_group: Query<'w, 's, (), With<Group>>,
    affiliation: Query<'w, 's, &'static Affiliation<Entity>, Without<ModelMarker>>,
    names: Query<'w, 's, &'static NameInSite>,
    textures: Query<'w, 's, &'static Texture>,
    members: Query<'w, 's, &'static Members>,
    default_file: Query<'w, 's, &'static DefaultFile>,
    current_workspace: Res<'w, CurrentWorkspace>,
    change_texture: EventWriter<'w, Change<Texture>>,
    selector: SelectorWidget<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectGroup<'w, 's> {
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

impl<'w, 's> InspectGroup<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        if self.is_group.contains(id) {
            self.show_group_properties(id, ui);
        }

        if let Ok(Affiliation(Some(group))) = self.affiliation.get(id) {
            ui.separator();
            let name = self.names.get(*group).map(|n| n.0.as_str()).unwrap_or("");
            ui.label(RichText::new(format!("Group Properties of [{}]", name)).size(18.0));
            ui.add_space(5.0);
            self.show_group_properties(*group, ui);
        }
    }

    pub fn show_group_properties(&mut self, id: Entity, ui: &mut Ui) {
        let default_file = self
            .current_workspace
            .root
            .map(|e| self.default_file.get(e).ok())
            .flatten();

        if let Ok(texture) = self.textures.get(id) {
            ui.label(RichText::new("Texture Properties").size(18.0));
            if let Some(new_texture) = InspectTexture::new(texture, default_file).show(ui) {
                self.change_texture.send(Change::new(new_texture, id));
            }
            ui.add_space(10.0);
        }
        if let Ok(members) = self.members.get(id) {
            CollapsingHeader::new("Members").show(ui, |ui| {
                for member in members.iter() {
                    self.selector.show_widget(*member, ui);
                }
            });
        }
    }
}
