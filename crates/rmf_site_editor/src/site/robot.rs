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
        AddModifier, Affiliation, ChangeCurrentScenario, IssueKey, LastSetValue, LevelElevation,
        Modifier, NameInSite, Property, Robot, RobotLevel, ScenarioModifiers,
    },
    Issue, ValidateWorkspace,
};
use bevy::ecs::{hierarchy::ChildOf, system::SystemState};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Implement scenario property for RobotLevel to modify a robot model's parent level
/// across different scenario
impl Property for RobotLevel<Entity> {
    fn get_fallback(
        for_element: Entity,
        _in_scenario: Entity,
        world: &mut World,
    ) -> RobotLevel<Entity> {
        let mut state: SystemState<(
            Query<&LastSetValue<RobotLevel<Entity>>>,
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

        RobotLevel(lowest_level)
    }

    fn insert(for_element: Entity, in_scenario: Entity, value: Self, world: &mut World) {
        let mut state: SystemState<(Query<&ChildOf>, Query<(), With<LevelElevation>>)> =
            SystemState::new(world);
        let (child_of, levels) = state.get(world);

        // When a new RobotLevel component is inserted (as a required component of Robot),
        // we want to make sure that the data reflect the robot's current parent level.
        let level_entity = child_of
            .get(for_element)
            .map(|co| co.parent())
            .ok()
            .filter(|e| levels.get(*e).is_ok());
        world
            .commands()
            .entity(for_element)
            .insert(RobotLevel(level_entity));

        // Create modifier for this robot
        let mut modifier_state: SystemState<(
            Query<(&mut Modifier<RobotLevel<Entity>>, &Affiliation<Entity>)>,
            Query<(Entity, &ScenarioModifiers<Entity>, Ref<Affiliation<Entity>>)>,
        )> = SystemState::new(world);
        let (mut level_modifiers, scenarios) = modifier_state.get_mut(world);

        let Ok((_, scenario_modifiers, _)) = scenarios.get(in_scenario) else {
            return;
        };
        let mut new_level_modifiers = Vec::<(Modifier<RobotLevel<Entity>>, Entity)>::new();

        if let Some((mut level_modifier, _)) = scenario_modifiers
            .get(&for_element)
            .and_then(|e| level_modifiers.get_mut(*e).ok())
        {
            // If a robot level modifier already exists for this scenario, update it
            **level_modifier = value.clone();
        } else {
            // If a robot level modifier does not exist in this scenario, insert one
            new_level_modifiers.push((
                Modifier::<RobotLevel<Entity>>::new(value.clone()),
                in_scenario,
            ));
        }

        // Spawn all new modifier entities
        let new_modifier_entities = new_level_modifiers
            .iter()
            .map(|(modifier, scenario)| (world.spawn(modifier.clone()).id(), *scenario))
            .collect::<Vec<(Entity, Entity)>>();

        let mut events_state: SystemState<EventWriter<AddModifier>> = SystemState::new(world);
        let mut add_modifier = events_state.get_mut(world);
        for (modifier_entity, scenario_entity) in new_modifier_entities.iter() {
            add_modifier.write(AddModifier::new(
                for_element,
                *modifier_entity,
                *scenario_entity,
            ));
        }
    }

    fn insert_on_new_scenario(in_scenario: Entity, world: &mut World) {
        let mut modifier_state: SystemState<(
            Query<&ChildOf>,
            Query<&Children>,
            Query<(), With<LevelElevation>>,
            Query<(&mut Modifier<RobotLevel<Entity>>, &Affiliation<Entity>)>,
            Query<Entity, With<Robot>>,
        )> = SystemState::new(world);
        let (child_of, children, levels, level_modifiers, robot_instances) =
            modifier_state.get_mut(world);

        // Spawn visibility modifier entities when new root scenarios are created
        let mut have_robot = HashSet::new();
        if let Ok(scenario_children) = children.get(in_scenario) {
            for child in scenario_children {
                if let Ok((_, a)) = level_modifiers.get(*child) {
                    if let Some(a) = a.0 {
                        have_robot.insert(a);
                    }
                }
            }
        }

        let mut target_robots = HashMap::<Entity, Option<Entity>>::new();
        for robot_entity in robot_instances.iter() {
            if !have_robot.contains(&robot_entity) {
                // When new root scenarios are created, insert the correct RobotLevel based on the
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
                    .spawn(Modifier::<RobotLevel<Entity>>::new(RobotLevel(*level)))
                    .id(),
            ));
        }

        let mut events_state: SystemState<(
            EventWriter<AddModifier>,
            EventWriter<ChangeCurrentScenario>,
        )> = SystemState::new(world);
        let (mut add_modifier, mut change_current_scenario) = events_state.get_mut(world);
        for (instance_entity, modifier_entity) in new_modifiers.iter() {
            add_modifier.write(AddModifier::new(
                *instance_entity,
                *modifier_entity,
                in_scenario,
            ));
        }
        change_current_scenario.write(ChangeCurrentScenario(in_scenario));
    }
}

/// This system monitors changes to the RobotLevel for each robot and updates its parent
/// level accordingly
pub fn update_robot_level(
    mut commands: Commands,
    robot_levels: Query<(Entity, Ref<RobotLevel<Entity>>), With<Robot>>,
    level_elevation: Query<(), With<LevelElevation>>,
) {
    for (robot_entity, robot_level) in robot_levels.iter() {
        if robot_level.is_changed() && !robot_level.is_added() {
            if let Some(level_entity) = robot_level.0.filter(|e| level_elevation.get(*e).is_ok()) {
                commands.entity(robot_entity).insert(ChildOf(level_entity));
            }
        }
    }
}

/// Unique UUID to identify issue of invalid RobotLevels
pub const INVALID_ROBOT_LEVEL_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x7e6937c359ff4ec88a23c7cef2683e7fu128);

pub fn check_for_invalid_robot_levels(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    child_of: Query<&ChildOf>,
    level_modifiers: Query<(Entity, &Modifier<RobotLevel<Entity>>, &Affiliation<Entity>)>,
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
                    // RobotLevel is not pointing to a valid Level entity
                    format!(
                        "Robot {:?} has an invalid RobotLevel inserted in scenario {:?}: {:?}",
                        robot_name,
                        scenarios
                            .get(scenario_entity)
                            .map(|n| n.0.clone())
                            .unwrap_or(format!("{:?}", scenario_entity)),
                        level_entity
                    )
                } else {
                    format!(
                        "Robot {:?} has no RobotLevel inserted in scenario {:?}",
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
