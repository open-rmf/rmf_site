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
        Property, ScenarioMarker, ScenarioModifiers, SiteID, StandardProperty, UpdateProperty,
    },
    Issue, ValidateWorkspace,
};
use bevy::{
    ecs::{
        component::Mutable, hierarchy::ChildOf, query::QueryFilter, system::SystemParam,
        world::OnDespawn,
    },
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

#[derive(Clone, Debug, Event, Copy)]
pub struct UpdateModifier<T> {
    pub scenario: Entity,
    pub element: Entity,
    pub update: T,
}

impl<T> UpdateModifier<T> {
    pub fn new(scenario: Entity, element: Entity, update: T) -> Self {
        Self {
            scenario,
            element,
            update,
        }
    }
}

#[derive(SystemParam)]
pub struct GetModifier<'w, 's, T: Component<Mutability = Mutable> + Clone + Default> {
    pub scenarios:
        Query<'w, 's, (&'static ScenarioModifiers, &'static Affiliation), With<ScenarioMarker>>,
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
                scenario_entity = *parent_entity;
            } else {
                // Modifier does not exist in the current scenario tree
                break;
            }
        }
        modifier
    }
}

/// Handles additions and removals of scenario modifiers
pub fn handle_scenario_modifiers(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut add_modifier: EventReader<AddModifier>,
    mut remove_modifier: EventReader<RemoveModifier>,
    mut scenarios: Query<(&mut ScenarioModifiers, &Affiliation), With<ScenarioMarker>>,
    mut update_property: EventWriter<UpdateProperty>,
    current_scenario: Res<CurrentScenario>,
) {
    for remove in remove_modifier.read() {
        let Ok((mut scenario_modifiers, _)) = scenarios.get_mut(remove.in_scenario) else {
            continue;
        };
        if let Some(modifier) = scenario_modifiers.remove(&remove.for_element) {
            commands.entity(modifier).despawn();
        }

        if current_scenario.0.is_some_and(|e| e == remove.in_scenario) {
            change_current_scenario.write(ChangeCurrentScenario(remove.in_scenario));
        };
    }

    for add in add_modifier.read() {
        let scenario_entity = if add.to_root {
            let mut target_scenario = add.in_scenario;
            let mut root_scenario: Option<Entity> = None;
            while root_scenario.is_none() {
                let Ok((_, parent_scenario)) = scenarios.get(target_scenario) else {
                    break;
                };
                if let Some(parent_entity) = parent_scenario.0 {
                    target_scenario = *parent_entity;
                } else {
                    root_scenario = Some(target_scenario);
                    break;
                }
            }
            if let Some(root_scenario_entity) = root_scenario {
                root_scenario_entity
            } else {
                error!("No root scenario found for the current scenario tree!");
                continue;
            }
        } else {
            add.in_scenario
        };

        let Ok((mut scenario_modifiers, _)) = scenarios.get_mut(scenario_entity) else {
            continue;
        };
        // If a modifier entity already exists, we ignore and despawn incoming modifier
        // entity.
        if scenario_modifiers.contains_key(&add.for_element) {
            commands.entity(add.modifier).despawn();
        } else {
            commands
                .entity(add.modifier)
                .insert(Affiliation::affiliated(add.for_element))
                .insert(ChildOf(scenario_entity));
            scenario_modifiers.insert(add.for_element, add.modifier);
        }

        update_property.write(UpdateProperty::new(add.for_element, add.in_scenario));
    }
}

/// Handles cleanup of scenario modifiers when elements are despawned
pub fn handle_cleanup_modifiers<M: Component<Mutability = Mutable> + Debug + Default + Clone>(
    trigger: Trigger<OnDespawn, M>,
    scenarios: Query<Entity, With<ScenarioMarker>>,
    mut remove_modifier: EventWriter<RemoveModifier>,
) {
    for scenario_entity in scenarios.iter() {
        remove_modifier.write(RemoveModifier::new(trigger.target(), scenario_entity));
    }
}

/// If a modifier entity in ScenarioModifiers has all Modifier<T> components removed,
/// send this entity to be removed and despawned.
pub fn handle_empty_modifiers<T: Property, F: QueryFilter>(
    mut remove_modifier: EventWriter<RemoveModifier>,
    mut removals: RemovedComponents<Modifier<T>>,
    affiliation: Query<&Affiliation>,
    current_scenario: Res<CurrentScenario>,
    modifiers: Query<(), F>,
    scenarios: Query<(Entity, &ScenarioModifiers), With<ScenarioMarker>>,
) {
    if !removals.is_empty() {
        for modifier_entity in removals.read() {
            // Check that this modifier entity no longer satisfy the specified filter
            if modifiers.get(modifier_entity).is_ok() {
                continue;
            }
            // Check that this modifier entity has an affiliated element
            let Some(element) = affiliation.get(modifier_entity).ok().and_then(|a| a.0) else {
                continue;
            };
            // Check that this element-modifier pair exists in the current scenario, else
            // search for the target scenario
            if let Some(scenario_entity) = current_scenario.0 {
                if scenarios
                    .get(scenario_entity)
                    .is_ok_and(|(_, scenario_modifiers)| {
                        scenario_modifiers
                            .get(&element)
                            .is_some_and(|e| *e == modifier_entity)
                    })
                {
                    remove_modifier.write(RemoveModifier::new(*element, scenario_entity));
                    continue;
                }
            }

            // The current scenario wasn't the target scenario, loop over scenario
            // modifiers to find
            for (scenario_entity, scenario_modifiers) in scenarios.iter() {
                if scenario_modifiers
                    .get(&element)
                    .is_some_and(|e| *e == modifier_entity)
                {
                    remove_modifier.write(RemoveModifier::new(*element, scenario_entity));
                    break;
                }
            }
            // Target scenario entity can't be found, this is an invalid modifier,
            // do nothing
        }
    }
}

/// Unique UUID to identify issue of missing root scenario modifiers
pub const MISSING_ROOT_MODIFIER_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x98df792d3de44d26b126a9335f9e743au128);

pub fn check_for_missing_root_modifiers<M: Component<Mutability = Mutable>>(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    scenarios: Query<(&ScenarioModifiers, &NameInSite, &Affiliation), With<ScenarioMarker>>,
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
                            entities: [SiteID::from(element)].into(),
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
