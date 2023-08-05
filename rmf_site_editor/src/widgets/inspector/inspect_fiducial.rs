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
    site::{Affiliation, FiducialMarker, UnusedFiducials, NameInSite, Group, Change},
    widgets::AppEvents,
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::egui::{ComboBox, Ui};

#[derive(SystemParam)]
pub struct InspectFiducialParams<'w, 's> {
    fiducials: Query<'w, 's, (&'static Affiliation<Entity>, &'static Parent), With<FiducialMarker>>,
    group_names: Query<'w, 's, &'static NameInSite, (With<Group>, With<FiducialMarker>)>,
    unused: Query<'w, 's, &'static UnusedFiducials>,
}

pub struct InspectFiducialWidget<'a, 'w1, 'w2, 's1, 's2> {
    entity: Entity,
    params: &'a InspectFiducialParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectFiducialWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        entity: Entity,
        params: &'a InspectFiducialParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self { entity, params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        let Ok((affiliation, parent)) = self.params.fiducials.get(self.entity) else { return };
        let Ok(tracker) = self.params.unused.get(parent.get()) else { return };

        ui.horizontal(|ui| {
            ui.label("Affiliation");
            let get_group_name = |affiliation: Affiliation<Entity>| {
                if let Some(group) = affiliation.0 {
                    if let Ok(name) = self.params.group_names.get(group) {
                        Some(name.0.clone())
                    } else {
                        None
                    }
                } else {
                    Some("<none>".to_owned())
                }
            };

            let mut new_affiliation = affiliation.clone();
            let selected_text = get_group_name(*affiliation)
                .unwrap_or_else(|| "<broken reference>".to_owned());
            ComboBox::from_id_source("fiducial_affiliation")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    if let Some(group_name) = get_group_name(new_affiliation) {
                        ui.selectable_value(
                            &mut new_affiliation,
                            *affiliation,
                            group_name,
                        );
                    }

                    if new_affiliation.0.is_some() {
                        ui.selectable_value(
                            &mut new_affiliation,
                            Affiliation(None),
                            "<none>",
                        );
                    }

                    for group in tracker.unused() {
                        let select_affiliation = Affiliation(Some(*group));
                        let Some(group_name) = get_group_name(select_affiliation) else { continue };
                        ui.selectable_value(
                            &mut new_affiliation,
                            select_affiliation,
                            group_name,
                        );
                    }
                });

            if new_affiliation != *affiliation {
                self.events.change_more.affiliation.send(
                    Change::new(new_affiliation, self.entity)
                );
            }
        });
    }
}
