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
    site::{
        Affiliation, FiducialMarker, FiducialUsage, NameInSite, Group,
        FiducialGroup, Change
    },
    widgets::{AppEvents, Icons},
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::egui::{ComboBox, Ui, ImageButton};

#[derive(Resource, Default)]
pub struct SearchForFiducial(pub String);

enum SearchResult {
    Current,
    NoMatch,
    Match(Entity),
    Conflict(&'static str),
}

impl SearchResult {
    fn consider(&mut self, entity: Entity) {
        match self {
            Self::NoMatch => { *self = SearchResult::Match(entity); }
            Self::Match(_) => { *self = SearchResult::Conflict("Multiple groups have this name"); }
            Self::Conflict(_) | Self::Current => { }
        }
    }

    fn conflict(&mut self, text: &'static str) {
        match self {
            // If we already found a match then don't change the behavior
            Self::Match(_) | Self::Current | Self::Conflict(_) => { }
            // If there is not a match, prevent the user from creating a duplicate
            // fiducial name
            _ => { *self = Self::Conflict(text) }
        }
    }
}

#[derive(SystemParam)]
pub struct InspectFiducialParams<'w, 's> {
    fiducials: Query<'w, 's, (&'static Affiliation<Entity>, &'static Parent), With<FiducialMarker>>,
    group_names: Query<'w, 's, &'static NameInSite, (With<Group>, With<FiducialMarker>)>,
    usage: Query<'w, 's, &'static FiducialUsage>,
    icons: Res<'w, Icons>,
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
        let Ok(tracker) = self.params.usage.get(parent.get()) else { return };

        ui.separator();
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
        let selected_text = get_group_name(*affiliation)
            .unwrap_or_else(|| "<broken reference>".to_owned());

        ui.horizontal(|ui| {
            let search = &mut self.events.change_more.search_for_fiducial.0;
            let mut result = SearchResult::NoMatch;

            if *search == selected_text {
                result = SearchResult::Current;
            }

            for (e, name) in tracker.unused().iter() {
                if *search == *name {
                    result.consider(*e);
                }
            }

            for (e, name) in tracker.used().iter() {
                if *search == *name && !affiliation.0.is_some_and(|a| a == *e) {
                    result.conflict("Group name is already taken");
                }
            }

            if *search == "" {
                // An empty string should not be used
                result.conflict("An empty string is not a good group name");
            }

            match result {
                SearchResult::Current => {
                    ui.button("Current")
                        .on_hover_text("This is the current group of the fiducial");
                }
                SearchResult::NoMatch => {
                    if ui.button("Add")
                        .on_hover_text("Create a new group for this fiducial")
                        .clicked()
                    {
                        let new_group = self.events.commands
                            .spawn(FiducialGroup::new(NameInSite(search.clone())))
                            .set_parent(tracker.site())
                            .id();
                        self.events.change_more.affiliation.send(
                            Change::new(Affiliation(Some(new_group)), self.entity)
                        );
                    }
                }
                SearchResult::Match(group) => {
                    if ui.button("Select").clicked() {
                        self.events.change_more.affiliation.send(
                            Change::new(Affiliation(Some(group)), self.entity)
                        );
                    }
                }
                SearchResult::Conflict(text) => {
                    ui.button("Invalid")
                        .on_hover_text(text);
                }
            }

            ui.text_edit_singleline(search)
                .on_hover_text("Search or add a group name for this fiducial");
        });

        let mut new_affiliation = affiliation.clone();
        ui.horizontal(|ui| {
            if ui.add(ImageButton::new(self.params.icons.trash.egui(), [18., 18.]))
                .on_hover_text("Remove this fiducial from its group")
                .clicked()
            {
                self.events.change_more.affiliation.send(
                    Change::new(Affiliation(None), self.entity)
                );
            }
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

                    for (group, name) in tracker.unused() {
                        if name.contains(&self.events.change_more.search_for_fiducial.0) {
                            let select_affiliation = Affiliation(Some(*group));
                            ui.selectable_value(
                                &mut new_affiliation,
                                select_affiliation,
                                name,
                            );
                        }
                    }
                });
        });

        if new_affiliation != *affiliation {
            self.events.change_more.affiliation.send(
                Change::new(new_affiliation, self.entity)
            );
        }
        ui.separator();
    }
}
