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
    site::{
        AddModifier, Affiliation, ChangeCurrentScenario, Element, IssueKey, LastSetValue,
        LevelElevation, Modifier, NameInSite, OnLevel, Property, Robot, ScenarioModifiers,
    },
    Issue, ValidateWorkspace,
};
use bevy::ecs::{hierarchy::ChildOf, system::SystemState};
use bevy::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

impl Element for Robot {}

/// Implement scenario property for OnLevel to modify a robot model's parent level
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
        if let Ok(robot_level) = last_set_level.get(for_element).map(|value| value.0.clone()) {
            return robot_level;
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
        in_scenario: Entity,
        _value: OnLevel<Entity>, // Value is unused for OnLevel
        world: &mut World,
    ) {
        let mut state: SystemState<(Query<&ChildOf>, Query<(), With<LevelElevation>>)> =
            SystemState::new(world);
        let (child_of, levels) = state.get(world);

        // When a new OnLevel component is inserted (as a required component of Robot),
        // we want to make sure that the data reflect the robot's current parent level.
        let level_entity = child_of
            .get(for_element)
            .map(|co| co.parent())
            .ok()
            .filter(|e| levels.get(*e).is_ok());
        world
            .commands()
            .entity(for_element)
            .insert(OnLevel(level_entity));

        let level_modifier =
            Self::create_modifier(for_element, in_scenario, OnLevel(level_entity), world);
        if let Some(level_modifier) = level_modifier {
            let modifier_entity = world.spawn(level_modifier).id();
            world.trigger(AddModifier::new(for_element, modifier_entity, in_scenario));
        }
    }

    fn insert_on_new_scenario(in_scenario: Entity, world: &mut World) {
        let mut modifier_state: SystemState<(
            Query<&ChildOf>,
            Query<&Children>,
            Query<(), With<LevelElevation>>,
            Query<(&Modifier<OnLevel<Entity>>, &Affiliation<Entity>)>,
            Query<Entity, With<Robot>>,
        )> = SystemState::new(world);
        let (child_of, children, levels, level_modifiers, robot_instances) =
            modifier_state.get_mut(world);

        let have_robot = Self::elements_with_modifiers(in_scenario, &children, &level_modifiers);

        let mut target_robots = HashMap::<Entity, Option<Entity>>::new();
        for robot_entity in robot_instances.iter() {
            if !have_robot.contains(&robot_entity) {
                // When new root scenarios are created, insert the correct OnLevel based on the
                // parent level entity of each robot model
                let level_entity = child_of
                    .get(robot_entity)
                    .map(|co| co.parent())
                    .ok()
                    .filter(|e| levels.get(*e).is_ok());
                target_robots.insert(robot_entity, level_entity);
            }
        }

        let mut new_modifiers = Vec::<(Entity, Entity)>::new();
        for (robot, level) in target_robots.iter() {
            new_modifiers.push((
                *robot,
                world
                    .commands()
                    .spawn(Modifier::<OnLevel<Entity>>::new(OnLevel(*level)))
                    .id(),
            ));
        }

        for (instance_entity, modifier_entity) in new_modifiers.iter() {
            world.trigger(AddModifier::new(
                *instance_entity,
                *modifier_entity,
                in_scenario,
            ));
        }
        let mut events_state: SystemState<EventWriter<ChangeCurrentScenario>> =
            SystemState::new(world);
        let mut change_current_scenario = events_state.get_mut(world);
        change_current_scenario.write(ChangeCurrentScenario(in_scenario));
    }
}

/// This system monitors changes to the OnLevel for each robot and updates its parent
/// level accordingly
pub fn update_robot_level(
    trigger: Trigger<OnReplace, LastSetValue<OnLevel<Entity>>>,
    mut commands: Commands,
    robot_levels: Query<(Entity, &OnLevel<Entity>), With<Robot>>,
    level_elevation: Query<(), With<LevelElevation>>,
) {
    if let Ok((robot_entity, robot_level)) = robot_levels.get(trigger.target()) {
        if let Some(level_entity) = robot_level.0.filter(|e| level_elevation.get(*e).is_ok()) {
            commands.entity(robot_entity).insert(ChildOf(level_entity));
        }
    }
}

/// Unique UUID to identify issue of invalid OnLevels
pub const INVALID_ROBOT_LEVEL_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x7e6937c359ff4ec88a23c7cef2683e7fu128);

pub fn check_for_invalid_robot_levels(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    child_of: Query<&ChildOf>,
    level_modifiers: Query<(Entity, &Modifier<OnLevel<Entity>>, &Affiliation<Entity>)>,
    levels: Query<Entity, With<LevelElevation>>,
    robots: Query<&NameInSite, With<Robot>>,
    scenarios: Query<&NameInSite, With<ScenarioModifiers<Entity>>>,
) {
    for root in validate_events.read() {
        for (modifier_entity, level_modifier, affiliation) in level_modifiers.iter() {
            let Ok(scenario_entity) = child_of.get(modifier_entity).map(|co| co.parent()) else {
                continue;
            };
            if let Some((robot_name, robot_entity)) =
                affiliation.0.and_then(|e| robots.get(e).ok().zip(Some(e)))
            {
                let brief = if let Some(level_entity) = level_modifier.0 {
                    if levels.get(level_entity).is_ok() {
                        continue;
                    }
                    // OnLevel is not pointing to a valid Level entity
                    format!(
                        "Robot {:?} has an invalid OnLevel inserted in scenario {:?}: {:?}",
                        robot_name,
                        scenarios
                            .get(scenario_entity)
                            .map(|n| n.0.clone())
                            .unwrap_or(format!("{:?}", scenario_entity)),
                        level_entity
                    )
                } else {
                    format!(
                        "Robot {:?} has no OnLevel inserted in scenario {:?}",
                        robot_name,
                        scenarios
                            .get(scenario_entity)
                            .map(|n| n.0.clone())
                            .unwrap_or(format!("{:?}", scenario_entity)),
                    )
                };

                let issue = Issue {
                    key: IssueKey {
                        entities: [robot_entity].into(),
                        kind: INVALID_ROBOT_LEVEL_ISSUE_UUID,
                    },
                    brief,
                    hint: "Check that the Robot Level for this robot is assigned to a valid level."
                        .to_string(),
                };
                let issue_id = commands.spawn(issue).id();
                commands.entity(**root).add_child(issue_id);
            }
        }
    }
}
