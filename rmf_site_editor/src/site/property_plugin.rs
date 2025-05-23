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
    Affiliation, ChangeCurrentScenario, CurrentScenario, InstanceMarker, ScenarioMarker,
    ScenarioModifiers,
};
use bevy::{
    ecs::{component::Mutable, hierarchy::ChildOf, system::SystemParam},
    prelude::*,
};
use std::fmt::Debug;

pub trait Property: Component<Mutability = Mutable> + Debug + Default + Clone {
    fn get_fallback(for_element: Entity, in_scenario: Entity) -> Self;
}

// TODO(@xiyuoh) create StandardProperty

pub trait Modifier<T: Property>: Component<Mutability = Mutable> + Debug + Default + Clone {
    /// This system retrieves the property values for this element's modifier, if any
    fn get(&self) -> Option<T>;

    /// This system climbs up the scenario tree to retrieve the inherited property
    /// value for this element, if any. This system can be overwritten in implementation.
    fn retrieve_inherited(
        &self,
        for_element: Entity,
        in_scenario: Entity,
        get_modifier: &GetModifier<Self>,
    ) -> Option<T> {
        let mut parent_value: Option<T> = None;
        let mut target_scenario = in_scenario;
        while parent_value.is_none() {
            let Some(parent_entity) = get_modifier
                .scenarios
                .get(target_scenario)
                .ok()
                .and_then(|(_, p)| p.0)
            else {
                break;
            };

            if let Some(modifier) = get_modifier.get(parent_entity, for_element) {
                parent_value = modifier.get();
            }
            target_scenario = parent_entity;
        }
        parent_value
    }
}

/// This event is triggered when changes have been made to an element in the
/// current scenario, and requires the element property value to be updated.
#[derive(Debug, Event)]
pub struct UpdateProperty {
    pub for_element: Entity,
    pub in_scenario: Entity,
}

impl UpdateProperty {
    pub fn new(for_element: Entity, in_scenario: Entity) -> Self {
        Self {
            for_element,
            in_scenario,
        }
    }
}

pub struct PropertyPlugin<T: Property, S: Modifier<T>> {
    _ignore: std::marker::PhantomData<(T, S)>,
}

impl<T: Property, S: Modifier<T>> Default for PropertyPlugin<T, S> {
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<T: Property, S: Modifier<T>> Plugin for PropertyPlugin<T, S> {
    fn build(&self, app: &mut App) {
        app.add_event::<UpdateProperty>().add_observer(
            |trigger: Trigger<UpdateProperty>,
             mut values: Query<&mut T, With<InstanceMarker>>,
             mut commands: Commands,
             mut add_modifier: EventWriter<AddModifier>,
             current_scenario: Res<CurrentScenario>,
             get_modifier: GetModifier<S>| {
                let event = trigger.event();
                // Only update current scenario properties
                if !current_scenario.0.is_some_and(|e| e == event.in_scenario) {
                    return;
                }

                if let Ok(mut value) = values.get_mut(event.for_element) {
                    if let Some(modifier) = get_modifier.get(event.in_scenario, event.for_element) {
                        *value = modifier
                            .get()
                            .or_else(|| {
                                modifier.retrieve_inherited(
                                    event.for_element,
                                    event.in_scenario,
                                    &get_modifier,
                                )
                            })
                            .unwrap_or(T::get_fallback(event.for_element, event.in_scenario));
                    } else {
                        // No instance modifier exists for this model instance/scenario pairing
                        // TODO(@xiyuoh) catch this with a diagnostic
                        // Make sure that an instance modifier exists in the current scenario tree
                        let root_modifier_entity = commands.spawn(S::default()).id();
                        add_modifier.write(AddModifier::new_to_root(
                            event.for_element,
                            root_modifier_entity,
                            event.in_scenario,
                        ));
                    }
                }
            },
        );
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
        With<ScenarioMarker>,
    >,
    pub modifiers: Query<'w, 's, &'static T>,
}

impl<'w, 's, T: Component<Mutability = Mutable> + Clone + Default> GetModifier<'w, 's, T> {
    pub fn get(&self, scenario: Entity, entity: Entity) -> Option<&T> {
        let mut modifier: Option<&T> = None;
        let mut scenario_entity = scenario;
        while modifier.is_none() {
            let Ok((scenario_modifiers, scenario_parent)) = self.scenarios.get(scenario_entity)
            else {
                break;
            };
            if let Some(target_modifier) = scenario_modifiers
                .get(&entity)
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

/// Handles additions and removals of scenario modifiers
pub fn handle_scenario_modifiers(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut add_modifier: EventReader<AddModifier>,
    mut remove_modifier: EventReader<RemoveModifier>,
    mut scenarios: Query<
        (&mut ScenarioModifiers<Entity>, &Affiliation<Entity>),
        With<ScenarioMarker>,
    >,
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
                continue;
            }
        } else {
            add.in_scenario
        };

        let Ok((mut scenario_modifiers, _)) = scenarios.get_mut(scenario_entity) else {
            continue;
        };
        // If a modifier entity already exists, despawn incoming modifier entity
        if let Some(current_modifier) = scenario_modifiers.get(&add.for_element) {
            // TODO(@xiyuoh) note this means we're getting rid of Recall data also
            commands.entity(*current_modifier).despawn();
        }

        commands
            .entity(add.modifier)
            .insert(Affiliation(Some(add.for_element)))
            .insert(ChildOf(scenario_entity));
        scenario_modifiers.insert(add.for_element, add.modifier);

        commands.trigger(UpdateProperty::new(add.for_element, add.in_scenario));
    }
}
