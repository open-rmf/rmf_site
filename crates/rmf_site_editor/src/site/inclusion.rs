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

use crate::site::{
    Affiliation, Element, GetModifier, Inclusion, InstanceMarker, LastSetValue, Modifier, Pending,
    Property, ScenarioModifiers, UpdateModifier,
};
use bevy::{ecs::system::SystemState, prelude::*};
use rmf_site_picking::{Select, Selection};
use std::collections::HashSet;

impl Property for Inclusion {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Inclusion {
        Inclusion::default()
    }

    fn on_new_element(
        for_element: Entity,
        in_scenario: Entity,
        _value: Inclusion,
        world: &mut World,
    ) {
        let mut scenario_state: SystemState<
            Query<(Entity, &ScenarioModifiers<Entity>, &Affiliation<Entity>)>,
        > = SystemState::new(world);
        let scenarios = scenario_state.get_mut(world);

        // Insert inclusion modifier into all root scenarios outside of the current tree as hidden
        let mut current_root_entity: Entity = in_scenario;
        while let Ok((_, _, parent_scenario)) = scenarios.get(current_root_entity) {
            if let Some(parent_scenario_entity) = parent_scenario.0 {
                current_root_entity = parent_scenario_entity;
            } else {
                break;
            }
        }
        let mut root_scenarios = HashSet::<Entity>::new();
        for (scenario_entity, _, parent_scenario) in scenarios.iter() {
            if parent_scenario.0.is_some() || scenario_entity == current_root_entity {
                continue;
            }
            root_scenarios.insert(scenario_entity);
        }
        for root in root_scenarios.iter() {
            world.trigger(UpdateModifier::modify(
                *root,
                for_element,
                Inclusion::Hidden,
            ));
        }
    }

    fn on_new_scenario<E: Element>(
        in_scenario: Entity,
        affiliation: Affiliation<Entity>,
        world: &mut World,
    ) {
        // Only insert Hidden inclusion modifiers for root scenarios
        if affiliation.0.is_some() {
            return;
        }
        let mut state: SystemState<(
            Query<&Children>,
            Query<(&Modifier<Inclusion>, &Affiliation<Entity>)>,
            Query<Entity, (With<E>, Without<Pending>)>,
        )> = SystemState::new(world);
        let (children, task_modifiers, task_entities) = state.get_mut(world);

        let have_task = Self::elements_with_modifiers(in_scenario, &children, &task_modifiers);

        let mut target_tasks = HashSet::new();
        for task_entity in task_entities.iter() {
            if !have_task.contains(&task_entity) {
                target_tasks.insert(task_entity);
            }
        }

        for target in target_tasks.iter() {
            // Mark all task modifiers as Hidden
            world.trigger(UpdateModifier::modify(
                in_scenario,
                *target,
                Inclusion::Hidden,
            ));
        }
    }
}

/// This system monitors changes to the Inclusion property for instances and
/// updates the model visibility accordingly
pub fn handle_inclusion_change_for_model_visibility(
    trigger: Trigger<OnInsert, LastSetValue<Inclusion>>,
    mut instances: Query<(&Inclusion, &mut Visibility), With<InstanceMarker>>,
) {
    if let Ok((inclusion, mut visibility)) = instances.get_mut(trigger.target()) {
        match *inclusion {
            Inclusion::Included => {
                *visibility = Visibility::Inherited;
            }
            Inclusion::Hidden => {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

pub fn check_selected_is_included(
    mut select: EventWriter<Select>,
    selection: Res<Selection>,
    inclusion: Query<&Inclusion>,
) {
    if selection.get_single().is_some_and(|e| {
        inclusion.get(e).is_ok_and(|v| match v {
            Inclusion::Hidden => true,
            _ => false,
        })
    }) {
        select.write(Select::new(None));
    }
}

/// Count the number of scenarios an element is included in with the Inclusion modifier
pub fn count_scenarios_with_inclusion(
    scenarios: &Query<(Entity, &ScenarioModifiers<Entity>, &Affiliation<Entity>)>,
    element: Entity,
    get_modifier: &GetModifier<Modifier<Inclusion>>,
) -> i32 {
    scenarios.iter().fold(0, |x, (e, _, _)| {
        match get_modifier
            .get(e, element)
            .map(|m| **m)
            .unwrap_or(Inclusion::Hidden)
        {
            Inclusion::Hidden => x,
            _ => x + 1,
        }
    })
}
