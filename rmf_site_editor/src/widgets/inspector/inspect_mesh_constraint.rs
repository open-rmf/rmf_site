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

use bevy_egui::egui::Ui;
use rmf_site_format::*;
use std::collections::{BTreeMap, HashSet};

use crate::{
    site::{Category, SiteID},
    widgets::{inspector::SelectionWidget, AppEvents, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};

#[derive(SystemParam)]
pub struct InspectModelDependentsParams<'w, 's> {
    pub dependents: Query<'w, 's, &'static ConstraintDependents, With<ModelMarker>>,
    pub info: Query<'w, 's, (&'static Category, Option<&'static SiteID>)>,
    pub icons: Res<'w, Icons>,
}

pub struct InspectModelDependentsWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub model: Entity,
    pub params: &'a InspectModelDependentsParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectModelDependentsWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        model: Entity,
        params: &'a InspectModelDependentsParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            model,
            params,
            events,
        }
    }

    fn show_dependents(
        dependents: &HashSet<Entity>,
        params: &InspectModelDependentsParams<'w1, 's1>,
        events: &mut AppEvents<'w2, 's2>,
        ui: &mut Ui,
    ) {
        ui.heading("Constraint Dependents");
        let mut category_map: BTreeMap<Category, BTreeMap<Entity, Option<u32>>> = BTreeMap::new();
        for e in dependents {
            if let Ok((category, site_id)) = params.info.get(*e) {
                category_map
                    .entry(*category)
                    .or_default()
                    .insert(*e, site_id.map(|s| s.0));
            } else {
                ui.label(format!("ERROR: Broken reference to entity {e:?}"));
            }
        }

        for (category, entities) in &category_map {
            ui.label(category.label());

            for (e, site_id) in entities {
                ui.horizontal(|ui| {
                    SelectionWidget::new(*e, site_id.map(SiteID), params.icons.as_ref(), events)
                        .show(ui);
                });
            }
        }
    }

    pub fn show(mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            if let Ok(dependents) = self.params.dependents.get(self.model) {
                if dependents.0.is_empty() {
                    ui.label("No dependents");
                } else {
                    Self::show_dependents(&dependents.0, &self.params, &mut self.events, ui);
                }
            } else {
                ui.label("ERROR: Unable to find dependents info for this model");
            }
        });
    }
}
