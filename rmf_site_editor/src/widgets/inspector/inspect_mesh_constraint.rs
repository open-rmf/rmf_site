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

use crate::widgets::{Inspect, SelectorWidget, prelude::*};
use bevy::prelude::*;
use rmf_site_format::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::site::Category;

#[derive(SystemParam)]
pub struct InspectModelDependents<'w, 's> {
    dependents: Query<'w, 's, &'static ConstraintDependents, With<ModelMarker>>,
    categories: Query<'w, 's, &'static Category>,
    selector: SelectorWidget<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelDependents<'w, 's> {
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

impl<'w, 's> InspectModelDependents<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok(dependents) = self.dependents.get(id) else {
            return;
        };

        ui.vertical(|ui| {
            if dependents.0.is_empty() {
                ui.label("No dependents");
            } else {
                ui.heading("Constraint Dependents");
                let mut category_map: BTreeMap<Category, BTreeSet<Entity>> = BTreeMap::new();
                for e in &dependents.0 {
                    if let Ok(category) = self.categories.get(*e) {
                        category_map
                            .entry(*category)
                            .or_default()
                            .insert(*e);
                    } else {
                        ui.label(format!("ERROR: Broken reference to entity {e:?}"));
                    }
                }

                for (category, entities) in &category_map {
                    ui.label(category.label());

                    for e in entities {
                        ui.horizontal(|ui| {
                            self.selector.show_widget(*e, ui);
                        });
                    }
                }
            }
        });

        ui.add_space(10.0);
    }
}
