/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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
        AssociatedGraphs, Change, ConsiderAssociatedGraph, NameInSite, NavGraphMarker,
        RecallAssociatedGraphs,
    },
    widgets::{AppEvents, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, ImageButton, Ui};
use smallvec::SmallVec;
use std::collections::BTreeMap;

#[derive(SystemParam)]
pub struct InspectAssociatedGraphsParams<'w, 's> {
    associated: Query<
        'w,
        's,
        (
            &'static AssociatedGraphs<Entity>,
            &'static RecallAssociatedGraphs<Entity>,
        ),
    >,
    graphs: Query<'w, 's, (Entity, &'static NameInSite), With<NavGraphMarker>>,
    icons: Res<'w, Icons>,
}

pub struct InspectAssociatedGraphsWidget<'a, 'w1, 's1, 'w2, 's2> {
    pub entity: Entity,
    pub params: &'a InspectAssociatedGraphsParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> InspectAssociatedGraphsWidget<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(
        entity: Entity,
        params: &'a InspectAssociatedGraphsParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            entity,
            params,
            events,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let (associated, recall) = match self.params.associated.get(self.entity) {
            Ok(q) => q,
            _ => return,
        };

        let mut new_associated = associated.clone();
        ui.horizontal(|ui| {
            ui.label("Associated Graphs");
            ComboBox::from_id_source("Associated Graphs")
                .selected_text(new_associated.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        AssociatedGraphs::All,
                        recall.assume_only(&new_associated),
                        recall.assume_all_except(&new_associated),
                    ] {
                        ui.selectable_value(&mut new_associated, variant.clone(), variant.label());
                    }
                });
        });

        match &mut new_associated {
            AssociatedGraphs::All => {}
            AssociatedGraphs::Only(set) | AssociatedGraphs::AllExcept(set) => {
                let mut removed_graphs: SmallVec<[Entity; 2]> = SmallVec::new();
                for g in set.iter() {
                    let (_, name) = match self.params.graphs.get(*g) {
                        Ok(q) => q,
                        _ => continue,
                    };
                    ui.horizontal(|ui| {
                        if ui
                            .add(ImageButton::new(self.params.icons.trash.egui(), [18., 18.]))
                            .clicked()
                        {
                            removed_graphs.push(*g);
                        }
                        ui.label(&name.0);
                    });
                }

                let unused_graphs: BTreeMap<Entity, &NameInSite> = BTreeMap::from_iter(
                    self.params.graphs.iter().filter(|(e, _)| !set.contains(e)),
                );

                if let Some((first, _)) = unused_graphs.iter().next() {
                    ui.horizontal(|ui| {
                        let add_graph = ui.button("Add").clicked();
                        let mut choice = recall.consider.unwrap_or(*first);
                        let choice_text = unused_graphs
                            .get(&choice)
                            .map(|n| n.0.clone())
                            .unwrap_or_else(|| "<ERROR>".to_string());
                        ComboBox::from_id_source("Add Associated Graph")
                            .selected_text(choice_text)
                            .show_ui(ui, |ui| {
                                for (e, name) in unused_graphs.iter() {
                                    ui.selectable_value(&mut choice, *e, &name.0);
                                }
                            });

                        if add_graph {
                            set.insert(choice);
                            self.events
                                .request
                                .consider_graph
                                .send(ConsiderAssociatedGraph::new(None, self.entity));
                        } else {
                            if Some(choice) != recall.consider {
                                self.events
                                    .request
                                    .consider_graph
                                    .send(ConsiderAssociatedGraph::new(Some(choice), self.entity));
                            }
                        }
                    });
                }

                for g in removed_graphs {
                    set.remove(&g);
                }
            }
        }

        if new_associated != *associated {
            self.events
                .change
                .associated_graphs
                .send(Change::new(new_associated, self.entity));
        }

        ui.add_space(10.0);
    }
}
