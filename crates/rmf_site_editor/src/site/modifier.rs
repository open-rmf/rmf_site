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
        Affiliation, Element, Inclusion, IssueKey, NameInSite, Pending, Property, ScenarioModifiers,
    },
    Issue, ValidateWorkspace,
};
use bevy::{
    ecs::{
        component::Mutable,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use std::{collections::HashSet, fmt::Debug};
use uuid::Uuid;

#[derive(Component, Debug, Default, Clone, Deref, DerefMut)]
pub struct Modifier<T: Property>(T);

impl<T: Property> Modifier<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl Property for Inclusion {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Inclusion {
        Inclusion::default()
    }

    fn insert(for_element: Entity, in_scenario: Entity, _value: Inclusion, world: &mut World) {
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

    fn insert_on_new_scenario<E: Element>(in_scenario: Entity, world: &mut World) {
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

#[derive(Clone, Debug, Copy)]
pub enum UpdateModifier<T: Property> {
    Modify(T),
    Reset,
}

impl<T: Property> UpdateModifier<T> {
    pub fn modify(scenario: Entity, element: Entity, value: T) -> UpdateModifierEvent<T> {
        UpdateModifierEvent::<T>::new(scenario, element, Self::Modify(value))
    }

    pub fn modify_without_trigger(
        scenario: Entity,
        element: Entity,
        value: T,
    ) -> UpdateModifierEvent<T> {
        UpdateModifierEvent::<T>::new_without_trigger(scenario, element, Self::Modify(value))
    }

    pub fn reset(scenario: Entity, element: Entity) -> UpdateModifierEvent<T> {
        UpdateModifierEvent::<T>::new(scenario, element, Self::Reset)
    }
}

#[derive(Clone, Debug, Event, Copy)]
pub struct UpdateModifierEvent<T: Property> {
    pub scenario: Entity,
    pub element: Entity,
    pub update_mode: UpdateModifier<T>,
    /// Whether to trigger a UseModifier event when updating the modifier
    pub trigger_use_modifier: bool,
}

impl<T: Property> UpdateModifierEvent<T> {
    pub fn new(scenario: Entity, element: Entity, update_mode: UpdateModifier<T>) -> Self {
        Self {
            scenario,
            element,
            update_mode,
            trigger_use_modifier: true,
        }
    }

    pub fn new_without_trigger(
        scenario: Entity,
        element: Entity,
        update_mode: UpdateModifier<T>,
    ) -> Self {
        Self {
            scenario,
            element,
            update_mode,
            trigger_use_modifier: false,
        }
    }
}

#[derive(SystemParam)]
pub struct GetModifier<'w, 's, T: Component<Mutability = Mutable> + Clone + Default> {
    pub scenarios: Query<
        'w,
        's,
        (
            &'static ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
    >,
    pub modifiers: Query<'w, 's, &'static T>,
}

impl<'w, 's, T: Component<Mutability = Mutable> + Clone + Default> GetModifier<'w, 's, T> {
    /// Retrieves the element's modifier in a scenario or the nearest inherited modifier.
    /// If None is returned, there is no modifier for the scenario-element pair in this scenario tree.
    pub fn get(&self, scenario: Entity, element: Entity) -> Option<&T> {
        let mut modifier: Option<&T> = None;
        let mut scenario_entity = scenario;
        while modifier.is_none() {
            let Ok((scenario_modifiers, scenario_parent)) = self.scenarios.get(scenario_entity)
            else {
                break;
            };
            if let Some(target_modifier) = scenario_modifiers
                .get(&element)
                .and_then(|e| self.modifiers.get(*e).ok())
            {
                modifier = Some(target_modifier);
                break;
            }

            if let Some(parent_entity) = scenario_parent.0 {
                scenario_entity = parent_entity;
            } else {
                // Modifier does not exist in the current scenario tree
                break;
            }
        }
        modifier
    }
}

/// Unique UUID to identify issue of missing root scenario modifiers
pub const MISSING_ROOT_MODIFIER_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x98df792d3de44d26b126a9335f9e743au128);

pub fn check_for_missing_root_modifiers<M: Component<Mutability = Mutable>>(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    scenarios: Query<(
        &ScenarioModifiers<Entity>,
        &NameInSite,
        &Affiliation<Entity>,
    )>,
    elements: Query<(Entity, Option<&NameInSite>), With<M>>,
) {
    for root in validate_events.read() {
        for (scenario_modifiers, scenario_name, parent_scenario) in scenarios.iter() {
            if parent_scenario.0.is_some() {
                continue;
            }
            for (element, element_name) in elements.iter() {
                if !scenario_modifiers.contains_key(&element) {
                    let name = element_name
                        .map(|name| name.0.clone())
                        .unwrap_or(element.index().to_string());
                    let issue = Issue {
                        key: IssueKey {
                            entities: [element].into(),
                            kind: MISSING_ROOT_MODIFIER_ISSUE_UUID,
                        },
                        brief: format!(
                            "Modifier for element {:?} is missing in root scenario {:?}",
                            name, scenario_name.0
                        ),
                        hint: "Toggle the scenario properties for this element in specified scenario tree. \
                               The editor will append a modifier with fallback values for this element."
                            .to_string(),
                    };
                    let issue_id = commands.spawn(issue).id();
                    commands.entity(**root).add_child(issue_id);
                }
            }
        }
    }
}
