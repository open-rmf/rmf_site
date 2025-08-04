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
    Issue, ValidateWorkspace,
    site::{Affiliation, IssueKey, NameInSite, Property, ScenarioModifiers},
};
use bevy::{
    ecs::{component::Mutable, system::SystemParam},
    prelude::*,
};
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Component, Debug, Default, Clone, Deref, DerefMut)]
pub struct Modifier<T: Property>(T);

impl<T: Property> Modifier<T> {
    pub fn new(value: T) -> Self {
        Self(value)
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
