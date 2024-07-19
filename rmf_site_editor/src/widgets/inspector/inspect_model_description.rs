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
    site::{Category, Change},
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, Ui};
use rmf_site_format::{Affiliation, Group, ModelMarker, NameInSite};

#[derive(SystemParam)]
pub struct InspectModelDescription<'w, 's> {
    model_instances: Query<
        'w,
        's,
        (&'static Category, &'static Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
    model_descriptions:
        Query<'w, 's, (Entity, &'static NameInSite), (With<Group>, With<ModelMarker>)>,
    change_affiliation: EventWriter<'w, Change<Affiliation<Entity>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelDescription<'w, 's> {
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

impl<'w, 's> InspectModelDescription<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((_, current_description_entity)) = self.model_instances.get(id) else {
            return;
        };
        let Ok((current_description_entity, current_description_name)) =
            self.model_descriptions.get(
                current_description_entity
                    .0
                    .expect("Model instances should have valid affiliation"),
            )
        else {
            return;
        };

        let mut new_description_entity = current_description_entity.clone();
        ui.horizontal(|ui| {
            ui.label("Description");
            ComboBox::from_id_source("model_description_affiliation")
                .selected_text(current_description_name.0.as_str())
                .show_ui(ui, |ui| {
                    for (entity, name, ..) in self.model_descriptions.iter() {
                        ui.selectable_value(&mut new_description_entity, entity, name.0.as_str());
                    }
                });
        });
        if new_description_entity != current_description_entity {
            self.change_affiliation
                .send(Change::new(Affiliation(Some(new_description_entity)), id));
        }
    }
}
