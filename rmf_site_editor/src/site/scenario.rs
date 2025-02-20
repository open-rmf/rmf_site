/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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
    interaction::{Select, Selection},
    site::{
        Affiliation, CurrentScenario, Delete, DeletionBox, DeletionFilters, Dependents,
        InstanceMarker, Pending, Pose, Scenario, ScenarioBundle, ScenarioMarker,
    },
    CurrentWorkspace,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

/// Handles changes to the current scenario
pub fn update_current_scenario(
    mut commands: Commands,
    mut selection: ResMut<Selection>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    current_workspace: Res<CurrentWorkspace>,
    scenarios: Query<&Scenario<Entity>>,
    mut instances: Query<(Entity, &mut Pose, &mut Visibility), With<InstanceMarker>>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
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

        // Iterate stack to identify instances in this model
        let mut active_instances = HashMap::<Entity, Pose>::new();
        for scenario in scenario_stack.iter().rev() {
            for (e, pose) in scenario.added_instances.iter() {
                active_instances.insert(*e, pose.clone());
            }
            for (e, pose) in scenario.moved_instances.iter() {
                active_instances.insert(*e, pose.clone());
            }
            for e in scenario.removed_instances.iter() {
                active_instances.remove(e);
            }
        }

        let current_site_entity = match current_workspace.root {
            Some(current_site) => current_site,
            None => return,
        };

        // If active, assign parent to level, otherwise assign parent to site
        for (entity, mut pose, mut visibility) in instances.iter_mut() {
            if let Some(new_pose) = active_instances.get(&entity) {
                *pose = new_pose.clone();
                *visibility = Visibility::Inherited;
            } else {
                commands.entity(entity).set_parent(current_site_entity);
                *visibility = Visibility::Hidden;
            }
        }

        // Deselect if not in current scenario
        if let Some(selected_entity) = selection.0.clone() {
            if let Ok((instance_entity, ..)) = instances.get(selected_entity) {
                if active_instances.get(&instance_entity).is_none() {
                    selection.0 = None;
                }
            }
        }

        *current_scenario = CurrentScenario(Some(*scenario_entity));
    }
}

/// Tracks pose changes for instances in the current scenario to update its properties
pub fn update_scenario_properties(
    current_scenario: Res<CurrentScenario>,
    mut scenarios: Query<&mut Scenario<Entity>>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    changed_instances: Query<(Entity, Ref<Pose>), (With<InstanceMarker>, Without<Pending>)>,
) {
    // Do nothing if scenario has changed, as we rely on pose changes by the user and not the system updating instances
    for ChangeCurrentScenario(_) in change_current_scenario.read() {
        return;
    }

    if let Some(mut current_scenario) = current_scenario
        .0
        .and_then(|entity| scenarios.get_mut(entity).ok())
    {
        for (entity, pose) in changed_instances.iter() {
            if pose.is_changed() {
                let existing_removed_instance = current_scenario
                    .removed_instances
                    .iter_mut()
                    .find(|e| **e == entity)
                    .map(|e| e.clone());
                if let Some(existing_removed_instance) = existing_removed_instance {
                    current_scenario
                        .moved_instances
                        .retain(|(e, _)| *e != existing_removed_instance);
                    current_scenario
                        .added_instances
                        .retain(|(e, _)| *e != existing_removed_instance);
                    return;
                }

                let existing_added_instance: Option<&mut (Entity, Pose)> = current_scenario
                    .added_instances
                    .iter_mut()
                    .find(|(e, _)| *e == entity);
                if let Some(existing_added_instance) = existing_added_instance {
                    existing_added_instance.1 = pose.clone();
                    return;
                } else if pose.is_added() {
                    current_scenario
                        .added_instances
                        .push((entity, pose.clone()));
                    return;
                }

                let existing_moved_instance = current_scenario
                    .moved_instances
                    .iter_mut()
                    .find(|(e, _)| *e == entity);
                if let Some(existing_moved_instance) = existing_moved_instance {
                    existing_moved_instance.1 = pose.clone();
                    return;
                } else {
                    current_scenario
                        .moved_instances
                        .push((entity, pose.clone()));
                    return;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct RemoveScenario(pub Entity);

/// When a scenario is removed, all child scenarios are removed as well
pub fn handle_remove_scenarios(
    mut commands: Commands,
    mut remove_scenario_requests: EventReader<RemoveScenario>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut delete: EventWriter<Delete>,
    mut current_scenario: ResMut<CurrentScenario>,
    current_workspace: Res<CurrentWorkspace>,
    mut scenarios: Query<
        (Entity, &Scenario<Entity>, Option<&mut Dependents>),
        With<ScenarioMarker>,
    >,
    children: Query<&Children>,
) {
    for request in remove_scenario_requests.read() {
        // Any child scenarios or instances added within the subtree are considered dependents
        // to be deleted
        let mut subtree_dependents = std::collections::HashSet::<Entity>::new();
        let mut queue = vec![request.0];
        while let Some(scenario_entity) = queue.pop() {
            if let Ok((_, scenario, _)) = scenarios.get(scenario_entity) {
                scenario.added_instances.iter().for_each(|(e, _)| {
                    subtree_dependents.insert(*e);
                });
            }
            if let Ok(children) = children.get(scenario_entity) {
                children.iter().for_each(|e| {
                    subtree_dependents.insert(*e);
                    queue.push(*e);
                });
            }
        }

        // Change to parent scenario, else root, else create an empty scenario and switch to it
        if let Some(parent_scenario_entity) = scenarios
            .get(request.0)
            .map(|(_, s, _)| s.parent_scenario.0)
            .ok()
            .flatten()
        {
            change_current_scenario.send(ChangeCurrentScenario(parent_scenario_entity));
        } else if let Some((root_scenario_entity, _, _)) = scenarios
            .iter()
            .filter(|(e, s, _)| request.0 != *e && s.parent_scenario.0.is_none())
            .next()
        {
            change_current_scenario.send(ChangeCurrentScenario(root_scenario_entity));
        } else {
            let new_scenario_entity = commands
                .spawn(ScenarioBundle::<Entity>::default())
                .set_parent(current_workspace.root.expect("No current site"))
                .id();
            *current_scenario = CurrentScenario(Some(new_scenario_entity));
        }

        // Delete with dependents
        if let Ok((_, _, Some(mut dependents))) = scenarios.get_mut(request.0) {
            dependents.extend(subtree_dependents.iter());
        } else {
            commands
                .entity(request.0)
                .insert(Dependents(subtree_dependents));
        }
        delete.send(Delete::new(request.0).and_dependents());
    }
}

pub fn setup_instance_deletion_filter(mut deletion_filter: ResMut<DeletionFilters>) {
    deletion_filter.insert(DeletionBox(Box::new(IntoSystem::into_system(
        filter_instance_deletion,
    ))));
}

/// Handle requests to remove model instances. If an instance was added in this scenario, or if
/// the scenario is root, the InstanceMarker is removed, allowing it to be permanently deleted.
/// Otherwise, it is only temporarily removed.
fn filter_instance_deletion(
    In(mut input): In<HashSet<Delete>>,
    mut scenarios: Query<&mut Scenario<Entity>>,
    current_scenario: ResMut<CurrentScenario>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    model_instance: Query<&Affiliation<Entity>, With<InstanceMarker>>,
    selection: Res<Selection>,
    mut select: EventWriter<Select>,
) -> HashSet<Delete> {
    let Some(current_scenario_entity) = current_scenario.0 else {
        return input;
    };

    let mut remove_only: HashSet<Delete> = HashSet::new();
    for removal in input.iter() {
        if let Ok(_) = model_instance.get(removal.element) {
            if let Ok(mut current_scenario) = scenarios.get_mut(current_scenario_entity) {
                // Delete if root scenario
                if current_scenario.parent_scenario.0.is_none() {
                    current_scenario
                        .added_instances
                        .retain(|(e, _)| *e != removal.element);
                    continue;
                }
                // Delete if added in this scenario
                if let Some(added_id) = current_scenario
                    .added_instances
                    .iter()
                    .position(|(e, _)| *e == removal.element)
                {
                    current_scenario.added_instances.remove(added_id);
                    continue;
                }
                // Otherwise, remove only
                current_scenario
                    .moved_instances
                    .retain(|(e, _)| *e != removal.element);
                current_scenario.removed_instances.push(removal.element);
                change_current_scenario.send(ChangeCurrentScenario(current_scenario_entity));
                remove_only.insert(*removal);
                if selection.0 == Some(removal.element) {
                    select.send(Select(None));
                }
            }
        }
    }
    input.retain(|delete| !remove_only.contains(delete));
    input
}
