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
    Affiliation, CurrentScenario, GetModifier, Modifier, RemoveModifier, ScenarioModifiers,
    UpdateModifier, UpdateModifierEvent,
};
use bevy::{
    ecs::{
        component::{ComponentInfo, Mutable},
        system::SystemState,
        world::OnDespawn,
    },
    prelude::*,
};
use std::{collections::HashSet, fmt::Debug};

pub trait Property: Component<Mutability = Mutable> + Debug + Default + Clone {
    /// Provides the fallback value for each property if no modifier value can be found.
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self;

    /// Inserts a new modifier for an element in the specified scenario. This is triggered
    /// when property T is newly added to an element.
    fn insert(_for_element: Entity, _in_scenario: Entity, _value: Self, _world: &mut World);

    /// Inserts new modifiers elements in a newly added root scenario.
    fn insert_on_new_scenario(_in_scenario: Entity, _world: &mut World);

    /// Helper function that returns the element entities that have existing modifiers
    /// for this property
    fn elements_with_modifiers(
        in_scenario: Entity,
        children: &Query<&Children>,
        modifiers: &Query<(&Modifier<Self>, &Affiliation<Entity>)>,
    ) -> HashSet<Entity> {
        let mut have_element = HashSet::new();
        if let Ok(scenario_children) = children.get(in_scenario) {
            for child in scenario_children {
                if let Ok((_, a)) = modifiers.get(*child) {
                    if let Some(a) = a.0 {
                        have_element.insert(a);
                    }
                }
            }
        }

        have_element
    }
}

pub trait StandardProperty: Component<Mutability = Mutable> + Debug + Default + Clone {}

impl<T: StandardProperty> Property for T {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self {
        T::default()
    }

    fn insert(_for_element: Entity, _in_scenario: Entity, _value: T, _world: &mut World) {
        // Do nothing
    }

    fn insert_on_new_scenario(_in_scenario: Entity, _world: &mut World) {
        // Do nothing
    }
}

/// This event is triggered when changes have been made to an element in the
/// current scenario, and requires the element property value to be updated.
#[derive(Debug, Event, Clone, Copy)]
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

#[derive(Component, Clone, Debug, Deref, DerefMut)]
pub struct LastSetValue<T: Property>(pub T);

impl<T: Property> LastSetValue<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

/// A scenario Element may have its Property T values changed across various scenarios
pub trait Element: Component<Mutability = Mutable> + Debug + Clone + 'static + Send + Sync {}

/// The PropertyPlugin helps to manage Property T values for Elements across
/// various scenarios.
pub struct PropertyPlugin<T: Property, E: Element> {
    _ignore: std::marker::PhantomData<(T, E)>,
}

impl<T: Property, E: Element> Default for PropertyPlugin<T, E> {
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<T: Property, E: Element> Plugin for PropertyPlugin<T, E> {
    fn build(&self, app: &mut App) {
        app.add_event::<UpdateProperty>()
            .add_event::<UpdateModifierEvent<T>>()
            .add_observer(on_update_modifier_event::<T, E>)
            .add_observer(on_update_property::<T, E>)
            .add_observer(on_add_property::<T, E>)
            .add_observer(on_add_root_scenario::<T>)
            .add_observer(on_remove_element::<E>)
            .add_observer(on_remove_modifier::<T>);
    }
}

/// Handles any updates to property modifiers and process them accordingly before
/// triggering updates to property values
fn on_update_modifier_event<T: Property, E: Element>(
    trigger: Trigger<UpdateModifierEvent<T>>,
    mut commands: Commands,
    mut property_modifiers: Query<&mut Modifier<T>, With<Affiliation<Entity>>>,
    mut scenarios: Query<(&mut ScenarioModifiers<Entity>, &Affiliation<Entity>)>,
) {
    let event = trigger.event();
    let Ok((mut scenario_modifiers, parent_scenario)) = scenarios.get_mut(event.scenario) else {
        return;
    };

    let modifier_entity = scenario_modifiers.get(&event.element);
    let property_modifier = modifier_entity.and_then(|e| property_modifiers.get_mut(*e).ok());

    match &event.update_mode {
        UpdateModifier::<T>::Modify(new_value) => {
            if let Some(mut property_modifier) = property_modifier {
                **property_modifier = new_value.clone();
            } else if let Some(modifier_entity) = modifier_entity {
                commands
                    .entity(*modifier_entity)
                    .insert(Modifier::<T>::new(new_value.clone()));
            } else {
                // Add new modifier and insert into ScenarioModifiers
                let modifier_entity = commands
                    .spawn(Modifier::<T>::new(new_value.clone()))
                    .insert(Affiliation(Some(event.element)))
                    .insert(ChildOf(event.scenario))
                    .id();
                scenario_modifiers.insert(event.element, modifier_entity);
            }
        }
        UpdateModifier::<T>::Reset => {
            // Only process resets if this is not a root scenario
            if parent_scenario.0.is_some() {
                if let Some(modifier_entity) = modifier_entity {
                    commands.entity(*modifier_entity).remove::<Modifier<T>>();
                }
            }
        }
    }

    if event.trigger_update_property {
        commands.trigger(UpdateProperty::new(event.element, event.scenario));
    }
}

/// Updates the current scenario's property values based on changes to the property modifiers
fn on_update_property<T: Property, E: Element>(
    trigger: Trigger<UpdateProperty>,
    world: &mut World,
    values: &mut QueryState<&mut T, With<E>>,
    scenario_state: &mut SystemState<(Commands, Res<CurrentScenario>, GetModifier<Modifier<T>>)>,
) {
    let event = trigger.event();
    let fallback_value = T::get_fallback(event.for_element, event.in_scenario, world);
    let (mut commands, current_scenario, get_modifier) = scenario_state.get(world);
    // Only update current scenario properties
    if !current_scenario.0.is_some_and(|e| e == event.in_scenario) {
        return;
    }
    // Only update elements registered for this plugin
    if !values.get(world, event.for_element).is_ok() {
        return;
    }

    let new_value: T = if let Some(modifier) =
        get_modifier.get(event.in_scenario, event.for_element)
    {
        (**modifier).clone()
    } else {
        // No modifier exists in this tree for this scenario/element pairing
        // Make sure that a modifier for this property exists in the current scenario tree
        if let Some(modifier_entity) = get_modifier
            .scenarios
            .get(event.in_scenario)
            .ok()
            .and_then(|(scenario_modifiers, _)| scenario_modifiers.get(&event.for_element))
        {
            commands
                .entity(*modifier_entity)
                .insert(Modifier::<T>::new(fallback_value));
            scenario_state.apply(world);
        } else {
            // Modifier entity does not exist, add one to the root scenario
            let mut target_scenario = event.in_scenario;
            let mut root_scenario: Option<Entity> = None;
            while root_scenario.is_none() {
                let Ok((_, parent_scenario)) = get_modifier.scenarios.get(target_scenario) else {
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
                world.trigger(UpdateModifier::modify(
                    root_scenario_entity,
                    event.for_element,
                    fallback_value,
                ))
            }
        }
        return;
    };

    let changed = values
        .get_mut(world, event.for_element)
        .is_ok_and(|mut value| {
            *value = new_value.clone();
            true
        });
    if changed {
        world
            .commands()
            .entity(event.for_element)
            .insert(LastSetValue::<T>::new(new_value));
    }
}

/// When an entity has been newly inserted with Property T, this observer will
/// call T::insert so that the appropriate modifiers can be created for this
/// Property via the callback.
fn on_add_property<T: Property, E: Element>(
    trigger: Trigger<OnAdd, T>,
    world: &mut World,
    state: &mut SystemState<(Query<&T, With<E>>, Res<CurrentScenario>)>,
) {
    let (values, current_scenario) = state.get_mut(world);
    let Ok(value) = values.get(trigger.target()) else {
        return;
    };
    let Some(scenario_entity) = current_scenario.0 else {
        return;
    };
    T::insert(trigger.target(), scenario_entity, value.clone(), world);
}

/// When a new scenario has been created, this observer checks that it is a root
/// scenario and calls T::insert_on_new_scenario so that the appropriate modifiers
/// can be created for this Property via the callback.
fn on_add_root_scenario<T: Property + 'static + Send + Sync>(
    trigger: Trigger<OnAdd, ScenarioModifiers<Entity>>,
    world: &mut World,
    state: &mut SystemState<Query<&Affiliation<Entity>>>,
) {
    let scenarios = state.get_mut(world);
    if !scenarios.get(trigger.target()).is_ok_and(|p| p.0.is_none()) {
        return;
    }

    T::insert_on_new_scenario(trigger.target(), world);
}

/// Handles cleanup of scenario modifiers when elements are despawned
pub fn on_remove_element<E: Element>(
    trigger: Trigger<OnDespawn, E>,
    scenarios: Query<Entity, With<ScenarioModifiers<Entity>>>,
    mut commands: Commands,
) {
    for scenario_entity in scenarios.iter() {
        commands.trigger(RemoveModifier::new(trigger.target(), scenario_entity));
    }
}

/// Whenever a Modifier<T> component has been removed from a modifier entity, this
/// observer checks that the entity is not empty (has no other Modifiers). If
/// empty, the modifier entity will be sent for removal and despawn.
fn on_remove_modifier<T: Property>(
    trigger: Trigger<OnRemove, Modifier<T>>,
    world: &mut World,
    state: &mut SystemState<(
        Commands,
        Res<CurrentScenario>,
        Query<(Entity, &ScenarioModifiers<Entity>)>,
        Query<&Affiliation<Entity>>,
    )>,
) {
    let modifier_entity = trigger.target();
    let Ok(components_info) = world
        .inspect_entity(modifier_entity)
        .map(|c| c.cloned().collect::<Vec<ComponentInfo>>())
    else {
        return;
    };

    // Count number of Modifier components
    let mut modifier_components: usize = 0;
    for info in components_info.iter() {
        let component_name = info.name();
        if component_name.to_string().contains("Modifier") {
            modifier_components += 1;
        }
    }

    // OnRemove is run before the component is actually removed, so there would
    // be at least one Modifier component attached to the entity during this check
    if modifier_components > 1 {
        // Target entity has existing modifier components, ignore
        return;
    }

    let (mut commands, current_scenario, scenarios, affiliation) = state.get_mut(world);
    // Check that this modifier entity has an affiliated element
    let Some(element) = affiliation.get(modifier_entity).ok().and_then(|a| a.0) else {
        return;
    };
    let scenario_entity: Option<Entity> = if let Some(scenario_entity) = current_scenario.0 {
        // Check that this element-modifier pair exists in the current scenario,
        // else search for the target scenario
        if scenarios
            .get(scenario_entity)
            .is_ok_and(|(_, scenario_modifiers)| {
                scenario_modifiers
                    .get(&element)
                    .is_some_and(|e| *e == modifier_entity)
            })
        {
            Some(scenario_entity)
        } else {
            None
        }
    } else {
        // The current scenario wasn't the target scenario, loop over scenario
        // modifiers to find
        let mut scenario: Option<Entity> = None;
        for (scenario_entity, scenario_modifiers) in scenarios.iter() {
            if scenario_modifiers
                .get(&element)
                .is_some_and(|e| *e == modifier_entity)
            {
                scenario = Some(scenario_entity);
                break;
            }
        }
        scenario
    };

    if let Some(scenario_entity) = scenario_entity {
        commands.trigger(RemoveModifier::new(element, scenario_entity));
        state.apply(world);
    }
}
