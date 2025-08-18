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
    Affiliation, ChangeCurrentScenario, CurrentScenario, GetModifier, Modifier, Pending,
    ScenarioModifiers, Trashcan, UpdateModifier, UpdateModifierEvent,
};
use bevy::{
    ecs::{component::Mutable, system::SystemState, world::OnDespawn},
    prelude::*,
};
use std::{collections::HashSet, fmt::Debug};

pub trait Property: Component<Mutability = Mutable> + Debug + Default + Clone + PartialEq {
    /// Provides the fallback value for each property if no modifier value can be found.
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self;

    /// Hook for custom behavior when a new element with Property T is introduced
    fn on_new_element(_for_element: Entity, _in_scenario: Entity, _value: Self, _world: &mut World);

    /// Hook for custom behavior when a new root scenario is created
    fn on_new_scenario<E: Element>(
        _in_scenario: Entity,
        _affiliation: Affiliation<Entity>,
        _world: &mut World,
    );

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

pub trait StandardProperty:
    Component<Mutability = Mutable> + Debug + Default + Clone + PartialEq
{
}

impl<T: StandardProperty> Property for T {
    fn get_fallback(for_element: Entity, _in_scenario: Entity, world: &mut World) -> Self {
        let mut state: SystemState<Query<&LastSetValue<Self>>> = SystemState::new(world);
        let last_set_value = state.get(world);

        last_set_value
            .get(for_element)
            .map(|value| (**value).clone())
            .unwrap_or(Self::default())
    }

    fn on_new_element(_for_element: Entity, _in_scenario: Entity, _value: T, _world: &mut World) {
        // Do nothing
    }

    fn on_new_scenario<E: Element>(
        _in_scenario: Entity,
        _affiliation: Affiliation<Entity>,
        _world: &mut World,
    ) {
        // Do nothing
    }
}

/// This event is triggered when the target element-scenario pair needs to be
/// refreshed to use property values from the appropriate modifier.
#[derive(Debug, Event, Clone, Copy)]
pub struct UseModifier {
    pub for_element: Entity,
    pub in_scenario: Entity,
}

impl UseModifier {
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
        app.add_event::<UseModifier>()
            .add_event::<UpdateModifierEvent<T>>()
            .add_observer(on_update_modifier_event::<T, E>)
            .add_observer(on_use_modifier::<T, E>)
            .add_observer(on_add_property::<T, E>)
            .add_observer(on_add_scenario::<T, E>)
            .add_observer(on_remove_element::<E>)
            .add_systems(Update, update_changed_property::<T, E>);
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
            // Make sure the LastSetValue is updated
            commands
                .entity(event.element)
                .insert(LastSetValue::<T>::new(new_value.clone()));
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

    if event.trigger_use_modifier {
        commands.trigger(UseModifier::new(event.element, event.scenario));
    }
}

/// Use modifier property values for the target element-scenario pair
fn on_use_modifier<T: Property, E: Element>(
    trigger: Trigger<UseModifier>,
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
/// call T::on_new_element for any custom behavior implemented for the Property,
/// e.g. insert additional modifiers in other scenarios.
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
    T::on_new_element(trigger.target(), scenario_entity, value.clone(), world);
}

/// When a new scenario has been created, this observer checks that it is a root
/// scenario and calls T::on_new_scenario for any custom behavior implemented
/// for the Property, e.g. insert additional modifiers in the new scenario.
fn on_add_scenario<T: Property + 'static + Send + Sync, E: Element>(
    trigger: Trigger<OnAdd, ScenarioModifiers<Entity>>,
    world: &mut World,
    state: &mut SystemState<Query<&Affiliation<Entity>>>,
    events_state: &mut SystemState<EventWriter<ChangeCurrentScenario>>,
) {
    let scenarios = state.get_mut(world);
    let Ok(affiliation) = scenarios.get(trigger.target()) else {
        return;
    };

    T::on_new_scenario::<E>(trigger.target(), *affiliation, world);

    let mut change_current_scenario = events_state.get_mut(world);
    change_current_scenario.write(ChangeCurrentScenario(trigger.target()));
}

/// Handles cleanup of scenario modifiers when elements are despawned
pub fn on_remove_element<E: Element>(
    trigger: Trigger<OnDespawn, E>,
    trashcan: Res<Trashcan>,
    mut commands: Commands,
    mut scenarios: Query<&mut ScenarioModifiers<Entity>>,
) {
    for mut scenario_modifiers in scenarios.iter_mut() {
        if let Some(modifier_entity) = scenario_modifiers.remove(&trigger.target()) {
            commands.entity(modifier_entity).insert(ChildOf(trashcan.0));
        }
    }
}

/// Track manual changes to property values in the current scenario and update
/// the relevant modifiers accordingly
fn update_changed_property<T: Property, E: Element>(
    mut commands: Commands,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    changed_values: Query<
        (Entity, Ref<T>, Option<Ref<LastSetValue<T>>>),
        (With<E>, Without<Pending>),
    >,
    current_scenario: Res<CurrentScenario>,
) {
    // Do nothing if scenario has changed
    for ChangeCurrentScenario(_) in change_current_scenario.read() {
        return;
    }
    let Some(scenario) = current_scenario.0 else {
        return;
    };

    for (entity, new_value, last_set_value) in changed_values.iter() {
        if new_value.is_changed() {
            if let Some(last_set_value) = last_set_value {
                if last_set_value.is_changed() {
                    // The new value might have been set by UpdateModifierEvent.
                    // If the last_set_value was changed on this cycle and it
                    // matches new_value then we take this to be the case.
                    // TODO(@mxgrey): Think of a more robust way to track this
                    if **last_set_value == *new_value {
                        continue;
                    }
                }
            }
            // The user has set a new value for this property, so we should
            // update its modifier in the current scenario.
            commands.trigger(UpdateModifier::modify_without_trigger(
                scenario,
                entity,
                new_value.clone(),
            ));
        }
    }
}
