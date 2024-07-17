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

use crate::{site::CurrentScenario, CurrentWorkspace};
use bevy::prelude::*;
use rmf_site_format::{Group, ModelMarker, NameInSite, Pose, Scenario, SiteParent};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

pub fn update_current_scenario(
    mut commands: Commands,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    current_workspace: Res<CurrentWorkspace>,
    scenarios: Query<&Scenario<Entity>>,
    mut model_instances: Query<
        (Entity, &mut Pose, &SiteParent<Entity>, &mut Visibility),
        (With<ModelMarker>, Without<Group>),
    >,
) {
    for ChangeCurrentScenario(scenario_entity) in change_current_scenario.read() {
        // Used to build a scenario from root
        let mut scenario_stack = Vec::<&Scenario<Entity>>::new();
        let mut scenario = scenarios
            .get(*scenario_entity)
            .expect("Failed to get scenario entity");
        loop {
            scenario_stack.push(scenario);
            if let Some(scenario_parent) = scenario.parent_scenario.0 {
                scenario = scenarios
                    .get(scenario_parent)
                    .expect("Scenario parent doesn't exist");
            } else {
                break;
            }
        }

        // Iterate stack to identify instances and poses in this model
        let mut active_model_instances = HashMap::<Entity, Pose>::new();
        for scenario in scenario_stack.iter().rev() {
            for (e, pose) in scenario.added_model_instances.iter() {
                active_model_instances.insert(*e, pose.clone());
            }
            for (e, pose) in scenario.moved_model_instances.iter() {
                active_model_instances.insert(*e, pose.clone());
            }
            for e in scenario.removed_model_instances.iter() {
                active_model_instances.remove(e);
            }
        }

        let current_site_entity = match current_workspace.root {
            Some(current_site) => current_site,
            None => return,
        };

        // If active, assign parent to level, otherwise assign parent to site
        for (entity, mut pose, parent, mut visibility) in model_instances.iter_mut() {
            if let Some(new_pose) = active_model_instances.get(&entity) {
                commands.entity(entity).set_parent(parent.0.unwrap());
                *pose = new_pose.clone();
                *visibility = Visibility::Inherited;
            } else {
                commands.entity(entity).set_parent(current_site_entity);
                *visibility = Visibility::Hidden;
            }
        }

        *current_scenario = CurrentScenario(Some(*scenario_entity));
    }
}

pub fn update_scenario_properties(
    current_scenario: Res<CurrentScenario>,
    mut scenarios: Query<&mut Scenario<Entity>>,
    changed_models: Query<(Entity, &NameInSite, Ref<Pose>), (With<ModelMarker>, Without<Group>)>,
) {
    if let Some(mut current_scenario) = current_scenario
        .0
        .and_then(|entity| scenarios.get_mut(entity).ok())
    {
        for (entity, _, pose) in changed_models.iter() {
            if pose.is_changed() {
                let existing_added_model = current_scenario
                    .added_model_instances
                    .iter_mut()
                    .find(|(e, _)| *e == entity);
                if let Some(existing_added_model) = existing_added_model {
                    existing_added_model.1 = pose.clone();
                    return;
                } else if pose.is_added() {
                    current_scenario
                        .added_model_instances
                        .push((entity, pose.clone()));
                    return;
                }

                let existing_moved_model = current_scenario
                    .moved_model_instances
                    .iter_mut()
                    .find(|(e, _)| *e == entity);
                if let Some(existing_moved_model) = existing_moved_model {
                    existing_moved_model.1 = pose.clone();
                    return;
                } else {
                    current_scenario
                        .moved_model_instances
                        .push((entity, pose.clone()));
                    return;
                }
            }
        }
    }
}
