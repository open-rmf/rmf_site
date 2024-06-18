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
    widgets::{prelude::*, Icons, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, ImageButton, Ui};
use smallvec::SmallVec;
use std::collections::BTreeMap;

#[derive(SystemParam)]
pub struct InspectAssociatedGraphs<'w, 's> {
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
    consider_graph: EventWriter<'w, ConsiderAssociatedGraph>,
    change_associated_graphs: EventWriter<'w, Change<AssociatedGraphs<Entity>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectAssociatedGraphs<'w, 's> {
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

impl<'w, 's> InspectAssociatedGraphs<'w, 's> {
    fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let (associated, recall) = match self.associated.get(id) {
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
                    let (_, name) = match self.graphs.get(*g) {
                        Ok(q) => q,
                        _ => continue,
                    };
                    ui.horizontal(|ui| {
                        if ui.add(ImageButton::new(self.icons.trash.egui())).clicked() {
                            removed_graphs.push(*g);
                        }
                        ui.label(&name.0);
                    });
                }

                let unused_graphs: BTreeMap<Entity, &NameInSite> =
                    BTreeMap::from_iter(self.graphs.iter().filter(|(e, _)| !set.contains(e)));

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
                            self.consider_graph
                                .send(ConsiderAssociatedGraph::new(None, id));
                        } else {
                            if Some(choice) != recall.consider {
                                self.consider_graph
                                    .send(ConsiderAssociatedGraph::new(Some(choice), id));
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
            self.change_associated_graphs
                .send(Change::new(new_associated, id));
        }

        ui.add_space(10.0);
    }
}
