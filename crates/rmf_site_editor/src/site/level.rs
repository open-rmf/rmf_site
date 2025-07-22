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

use crate::site::*;
use crate::{CurrentWorkspace, Issue, ValidateWorkspace};
use bevy::ecs::{hierarchy::ChildOf, system::SystemState};
use bevy::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub fn update_level_visibility(
    mut levels: Query<(Entity, &mut Visibility), With<LevelElevation>>,
    current_level: Res<CurrentLevel>,
) {
    if current_level.is_changed() {
        for (e, mut visibility) in &mut levels {
            *visibility = if Some(e) == **current_level {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// This system monitors changes to the OnLevel for a model and updates its parent
/// level accordingly
pub fn handle_on_level_change(
    trigger: Trigger<OnInsert, LastSetValue<OnLevel<Entity>>>,
    mut commands: Commands,
    on_levels: Query<(Entity, &OnLevel<Entity>)>,
    level_elevation: Query<(), With<LevelElevation>>,
) {
    if let Ok((entity, on_level)) = on_levels.get(trigger.target()) {
        if let Some(level_entity) = on_level.0.filter(|e| level_elevation.get(*e).is_ok()) {
            commands.entity(entity).insert(ChildOf(level_entity));
        }
    }
}

/// Implement scenario property for OnLevel to modify a model's parent level
/// across different scenario
impl Property for OnLevel<Entity> {
    fn get_fallback(
        for_element: Entity,
        _in_scenario: Entity,
        world: &mut World,
    ) -> OnLevel<Entity> {
        let mut state: SystemState<(
            Query<&LastSetValue<OnLevel<Entity>>>,
            Query<(Entity, &LevelElevation)>,
        )> = SystemState::new(world);
        let (last_set_level, levels) = state.get(world);

        // Return the last set elevation, otherwise return the lowest level in this site
        if let Ok(level) = last_set_level.get(for_element).map(|value| value.0.clone()) {
            return level;
        }

        let mut lowest_level: Option<Entity> = None;
        let mut lowest_level_elevation: f32 = std::f32::INFINITY;
        for (level_entity, level_elevation) in levels.iter() {
            if level_elevation.0 < lowest_level_elevation {
                lowest_level_elevation = level_elevation.0;
                lowest_level = Some(level_entity);
            }
        }

        OnLevel(lowest_level)
    }

    fn insert(
        for_element: Entity,
        _in_scenario: Entity,
        _value: OnLevel<Entity>, // Value is unused for OnLevel
        world: &mut World,
    ) {
        let mut state: SystemState<(Query<&ChildOf>, Query<(), With<LevelElevation>>)> =
            SystemState::new(world);
        let (child_of, levels) = state.get(world);

        // When a new OnLevel component is inserted, we want to make sure that
        // the data reflect the model's current parent level.
        let level_entity = child_of
            .get(for_element)
            .map(|co| co.parent())
            .ok()
            .filter(|e| levels.get(*e).is_ok());
        world
            .commands()
            .entity(for_element)
            .insert(OnLevel(level_entity));
    }

    fn insert_on_new_scenario<E: Element>(in_scenario: Entity, world: &mut World) {
        let mut modifier_state: SystemState<(
            Query<&ChildOf>,
            Query<&Children>,
            Query<(), With<LevelElevation>>,
            Query<(&Modifier<OnLevel<Entity>>, &Affiliation<Entity>)>,
            Query<Entity, With<OnLevel<Entity>>>,
        )> = SystemState::new(world);
        let (child_of, children, levels, level_modifiers, level_models) =
            modifier_state.get_mut(world);

        let have_level = Self::elements_with_modifiers(in_scenario, &children, &level_modifiers);

        let mut target_models = HashMap::<Entity, Option<Entity>>::new();
        for model_entity in level_models.iter() {
            if !have_level.contains(&model_entity) {
                // When new root scenarios are created, insert the correct OnLevel based on the
                // parent level entity of each model
                let level_entity = child_of
                    .get(model_entity)
                    .map(|co| co.parent())
                    .ok()
                    .filter(|e| levels.get(*e).is_ok());
                target_models.insert(model_entity, level_entity);
            }
        }

        for (model, level) in target_models.iter() {
            world.trigger(UpdateModifier::modify(in_scenario, *model, OnLevel(*level)));
        }
    }
}

pub fn assign_orphan_levels_to_site(
    mut commands: Commands,
    new_levels: Query<Entity, (Without<ChildOf>, Added<LevelElevation>)>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    if let Some(site) = current_workspace.to_site(&open_sites) {
        for level in &new_levels {
            commands.entity(site).add_child(level);
        }
    } else {
        warn!(
            "Unable to assign level to any site because there is no \
            current site"
        );
    }
}

pub fn assign_orphan_elements_to_level<T: Component>(
    mut commands: Commands,
    orphan_elements: Query<Entity, (With<T>, Without<ChildOf>)>,
    current_level: Res<CurrentLevel>,
) {
    let current_level = match current_level.0 {
        Some(c) => c,
        None => return,
    };

    for orphan in &orphan_elements {
        commands.entity(current_level).add_child(orphan);
    }
}

/// Unique UUID to identify issue of invalid OnLevels
pub const INVALID_LEVEL_ASSIGNMENT_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x7e6937c359ff4ec88a23c7cef2683e7fu128);

pub fn check_for_invalid_level_assignments(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    child_of: Query<&ChildOf>,
    level_modifiers: Query<(Entity, &Modifier<OnLevel<Entity>>, &Affiliation<Entity>)>,
    levels: Query<Entity, With<LevelElevation>>,
    level_models: Query<&NameInSite, With<OnLevel<Entity>>>,
    scenarios: Query<&NameInSite, With<ScenarioModifiers<Entity>>>,
) {
    for root in validate_events.read() {
        for (modifier_entity, level_modifier, affiliation) in level_modifiers.iter() {
            let Ok(scenario_entity) = child_of.get(modifier_entity).map(|co| co.parent()) else {
                continue;
            };
            if let Some((model_name, model_entity)) = affiliation
                .0
                .and_then(|e| level_models.get(e).ok().zip(Some(e)))
            {
                let brief = if let Some(level_entity) = level_modifier.0 {
                    if levels.get(level_entity).is_ok() {
                        continue;
                    }
                    // OnLevel is not pointing to a valid Level entity
                    format!(
                        "Model {:?} has an invalid OnLevel inserted in scenario {:?}: {:?}",
                        model_name,
                        scenarios
                            .get(scenario_entity)
                            .map(|n| n.0.clone())
                            .unwrap_or(format!("{:?}", scenario_entity)),
                        level_entity
                    )
                } else {
                    format!(
                        "Model {:?} has no OnLevel inserted in scenario {:?}",
                        model_name,
                        scenarios
                            .get(scenario_entity)
                            .map(|n| n.0.clone())
                            .unwrap_or(format!("{:?}", scenario_entity)),
                    )
                };

                let issue = Issue {
                    key: IssueKey {
                        entities: [model_entity].into(),
                        kind: INVALID_LEVEL_ASSIGNMENT_ISSUE_UUID,
                    },
                    brief,
                    hint: "Check that the On Level for this model is assigned to a valid level."
                        .to_string(),
                };
                let issue_id = commands.spawn(issue).id();
                commands.entity(**root).add_child(issue_id);
            }
        }
    }
}
