/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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
    Icons, WorkspaceMarker,
    inspector::SearchResult,
    site::{Category, Change},
    widgets::{Inspect, InspectionPlugin, WidgetSystem, prelude::*},
};
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemParam},
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, ComboBox, ImageButton, Ui};
use rmf_site_format::{
    Affiliation, Group, LaneMarker, LocationTags, MutexGroup, MutexMarker, NameInSite,
};

#[derive(Resource, Default)]
pub struct SearchForMutex(pub String);

#[derive(Default)]
pub struct InspectMutexPlugin {}

impl Plugin for InspectMutexPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SearchForMutex>()
            .add_plugins(InspectionPlugin::<InspectMutexAffiliation>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectMutexAffiliation<'w, 's> {
    with_mutex: Query<
        'w,
        's,
        (&'static Category, &'static Affiliation<Entity>),
        Or<(With<LaneMarker>, With<LocationTags>)>,
    >,
    mutex_groups: Query<'w, 's, &'static NameInSite, (With<Group>, With<MutexMarker>)>,
    child_of: Query<'w, 's, &'static ChildOf>,
    sites: Query<'w, 's, &'static Children, With<WorkspaceMarker>>,
    icons: Res<'w, Icons>,
    search_for_mutex: ResMut<'w, SearchForMutex>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectMutexAffiliation<'w, 's> {
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

impl<'w, 's> InspectMutexAffiliation<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((category, affiliation)) = self.with_mutex.get(id) else {
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

        CollapsingHeader::new("Mutex").show(ui, |ui| {
            let search = &mut self.search_for_mutex.0;

            let mut any_partial_matches = false;
            let mut result = SearchResult::NoMatch;
            for child in children {
                let Ok(name) = self.mutex_groups.get(*child) else {
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

            ui.horizontal(|ui| {
                if any_partial_matches {
                    if ui
                        .add(ImageButton::new(self.icons.search.egui()))
                        .on_hover_text("Search results for this text can be found below")
                        .clicked()
                    {
                        info!("Use the drop-down box to choose a mutex");
                    }
                } else {
                    ui.add(ImageButton::new(self.icons.empty.egui()))
                        .on_hover_text("No search results can be found for this text");
                }

                match result {
                    SearchResult::Empty => {
                        if ui
                            .add(ImageButton::new(self.icons.hidden.egui()))
                            .on_hover_text("An empty string is not a good mutex name")
                            .clicked()
                        {
                            warn!("You should not use an empty string as a mutex name");
                        }
                    }
                    SearchResult::Current => {
                        if ui
                            .add(ImageButton::new(self.icons.selected.egui()))
                            .on_hover_text("This is the name of the currently selected mutex")
                            .clicked()
                        {
                            info!("This mutex is already selected");
                        }
                    }
                    SearchResult::NoMatch => {
                        if ui
                            .add(ImageButton::new(self.icons.add.egui()))
                            .on_hover_text("Create a new mutex")
                            .clicked()
                        {
                            let new_mutex_group = self
                                .commands
                                .spawn(MutexGroup::new(NameInSite(search.clone())))
                                .insert(ChildOf(site))
                                .id();
                            self.commands
                                .trigger(Change::new(Affiliation(Some(new_mutex_group)), id));
                        }
                    }
                    SearchResult::Match(group) => {
                        if ui
                            .add(ImageButton::new(self.icons.confirm.egui()))
                            .on_hover_text("Select this mutex")
                            .clicked()
                        {
                            self.commands
                                .trigger(Change::new(Affiliation(Some(group)), id));
                        }
                    }
                    SearchResult::Conflict(text) => {
                        if ui
                            .add(ImageButton::new(self.icons.reject.egui()))
                            .on_hover_text(text)
                            .clicked()
                        {
                            warn!("Cannot set {search} as the mutex: {text}");
                        }
                    }
                }

                ui.text_edit_singleline(search)
                    .on_hover_text("Search for or create a new mutex");
            });

            let current_mutex_name = if let Some(a) = affiliation.0 {
                self.mutex_groups.get(a).ok().map(|n| n.0.as_str())
            } else {
                None
            }
            .unwrap_or("<none>");

            let mut new_affiliation = affiliation.clone();
            ui.horizontal(|ui| {
                if ui
                    .add(ImageButton::new(self.icons.exit.egui()))
                    .on_hover_text(format!("Remove this mutex from the {}", category.label()))
                    .clicked()
                {
                    new_affiliation = Affiliation(None);
                }

                let mut clear_filter = false;
                ComboBox::from_id_salt("mutex_affiliation")
                    .selected_text(current_mutex_name)
                    .show_ui(ui, |ui| {
                        for child in children {
                            if affiliation.0.is_some_and(|a| a == *child) {
                                continue;
                            }

                            if let Ok(n) = self.mutex_groups.get(*child) {
                                if n.0.contains(&self.search_for_mutex.0) {
                                    let select_affiliation = Affiliation(Some(*child));
                                    ui.selectable_value(
                                        &mut new_affiliation,
                                        select_affiliation,
                                        &n.0,
                                    );
                                }
                            }
                        }

                        if !self.search_for_mutex.0.is_empty() {
                            ui.selectable_value(&mut clear_filter, true, "more...");
                        }
                    });

                if clear_filter {
                    self.search_for_mutex.0.clear();
                }
            });

            if new_affiliation != *affiliation {
                self.commands.trigger(Change::new(new_affiliation, id));
            }
            ui.add_space(10.0);
        });
    }
}
