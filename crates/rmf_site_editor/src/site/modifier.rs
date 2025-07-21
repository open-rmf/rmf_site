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
        Affiliation, ChangeCurrentScenario, CurrentScenario, Inclusion, IssueKey, NameInSite,
        Property, ScenarioModifiers, StandardProperty, Trashcan, UpdateProperty,
    },
    Issue, ValidateWorkspace,
};
use bevy::{
    ecs::{component::Mutable, hierarchy::ChildOf, system::SystemParam},
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

impl StandardProperty for Inclusion {}

#[derive(Clone, Debug, Event)]
pub struct AddModifier {
    for_element: Entity,
    modifier: Entity,
    in_scenario: Entity,
    to_root: bool,
}

impl AddModifier {
    pub fn new(for_element: Entity, modifier: Entity, in_scenario: Entity) -> Self {
        Self {
            for_element,
            modifier,
            in_scenario,
            to_root: false,
        }
    }

    pub fn new_to_root(for_element: Entity, modifier: Entity, in_scenario: Entity) -> Self {
        Self {
            for_element,
            modifier,
            in_scenario,
            to_root: true,
        }
    }
}

#[derive(Clone, Debug, Event)]
pub struct RemoveModifier {
    for_element: Entity,
    in_scenario: Entity,
}

impl RemoveModifier {
    pub fn new(for_element: Entity, in_scenario: Entity) -> Self {
        Self {
            for_element,
            in_scenario,
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
        UpdateModifierEvent::new(scenario, element, Self::Modify(value))
    }

    pub fn reset(scenario: Entity, element: Entity) -> UpdateModifierEvent<T> {
        UpdateModifierEvent::new(scenario, element, Self::Reset)
    }
}

#[derive(Clone, Debug, Event, Copy)]
pub struct UpdateModifierEvent<T: Property> {
    pub scenario: Entity,
    pub element: Entity,
    pub update_mode: UpdateModifier<T>,
    /// Whether to trigger an UpdateProperty event when updating the modifier
    pub trigger_update_property: bool,
}

impl<T: Property> UpdateModifierEvent<T> {
    pub fn new(scenario: Entity, element: Entity, update_mode: UpdateModifier<T>) -> Self {
        Self {
            scenario,
            element,
            update_mode,
            trigger_update_property: true,
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
            trigger_update_property: false,
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

/// Handles additions of scenario modifiers
pub fn add_scenario_modifiers(
    trigger: Trigger<AddModifier>,
    mut commands: Commands,
    mut scenarios: Query<(&mut ScenarioModifiers<Entity>, &Affiliation<Entity>)>,
) {
    let event = trigger.event();
    let scenario_entity = if event.to_root {
        let mut target_scenario = event.in_scenario;
        let mut root_scenario: Option<Entity> = None;
        while root_scenario.is_none() {
            let Ok((_, parent_scenario)) = scenarios.get(target_scenario) else {
                break;
            };
            if let Some(parent_entity) = parent_scenario.0 {
                target_scenario = parent_entity;
            } else {
                root_scenario = Some(target_scenario);
                break;
            }
        }
        if let Some(root_scenario_entity) = root_scenario {
            root_scenario_entity
        } else {
            error!("No root scenario found for the current scenario tree!");
            return;
        }
    } else {
        event.in_scenario
    };

    let Ok((mut scenario_modifiers, _)) = scenarios.get_mut(scenario_entity) else {
        return;
    };
    // If a modifier entity already exists, we ignore and despawn incoming modifier
    // entity.
    if scenario_modifiers.contains_key(&event.for_element) {
        commands.entity(event.modifier).despawn();
    } else {
        commands
            .entity(event.modifier)
            .insert(Affiliation(Some(event.for_element)))
            .insert(ChildOf(scenario_entity));
        scenario_modifiers.insert(event.for_element, event.modifier);
    }

    commands.trigger(UpdateProperty::new(event.for_element, event.in_scenario));
}

/// Handles removals of scenario modifiers
pub fn remove_scenario_modifiers(
    trigger: Trigger<RemoveModifier>,
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut scenarios: Query<(&mut ScenarioModifiers<Entity>, &Affiliation<Entity>)>,
    current_scenario: Res<CurrentScenario>,
    trashcan: Res<Trashcan>,
) {
    let event = trigger.event();
    let Ok((mut scenario_modifiers, _)) = scenarios.get_mut(event.in_scenario) else {
        return;
    };
    if let Some(modifier) = scenario_modifiers.remove(&event.for_element) {
        commands.entity(modifier).insert(ChildOf(trashcan.0));
    }

    if current_scenario.0.is_some_and(|e| e == event.in_scenario) {
        change_current_scenario.write(ChangeCurrentScenario(event.in_scenario));
    };
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
