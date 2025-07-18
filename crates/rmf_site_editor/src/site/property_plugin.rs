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
    AddModifier, Affiliation, CurrentScenario, GetModifier, Modifier, ScenarioMarker,
    ScenarioModifiers,
};
use bevy::{
    ecs::{component::Mutable, query::QueryFilter, system::SystemState},
    prelude::*,
};
use smallvec::SmallVec;
use std::fmt::Debug;

pub trait Property: Component<Mutability = Mutable> + Debug + Default + Clone {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self;

    /// Inserts a new modifier for an element in the specified scenario. This is triggered
    /// when property T is newly added to an element.
    fn insert(_for_element: Entity, _in_scenario: Entity, _value: Self, _world: &mut World);

    /// Inserts new modifiers elements in a newly added root scenario.
    fn insert_on_new_scenario(_in_scenario: Entity, _world: &mut World);
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

#[derive(Component, Clone, Debug)]
pub struct LastSetValue<T: Property>(pub T);

impl<T: Property> LastSetValue<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

pub struct PropertyPlugin<T: Property, F: QueryFilter + 'static + Send + Sync> {
    _ignore: std::marker::PhantomData<(T, F)>,
}

impl<T: Property, F: QueryFilter + 'static + Send + Sync> Default for PropertyPlugin<T, F> {
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<T: Property, F: QueryFilter + 'static + Send + Sync> Plugin for PropertyPlugin<T, F> {
    fn build(&self, app: &mut App) {
        app.add_event::<UpdateProperty>()
            .add_systems(PostUpdate, update_property_value::<T, F>)
            .add_observer(on_add_property::<T, F>)
            .add_observer(on_add_root_scenario::<T, F>);
    }
}

fn update_property_value<T: Property, F: QueryFilter + 'static + Send + Sync>(
    world: &mut World,
    values: &mut QueryState<&mut T, F>,
    read_events_state: &mut SystemState<EventReader<UpdateProperty>>,
    add_modifier_state: &mut SystemState<EventWriter<AddModifier>>,
    scenario_state: &mut SystemState<(Commands, Res<CurrentScenario>, GetModifier<Modifier<T>>)>,
) {
    let mut update_property_events = read_events_state.get_mut(world);
    if update_property_events.is_empty() {
        return;
    }
    let mut update_property = SmallVec::<[UpdateProperty; 8]>::new();
    for event in update_property_events.read() {
        update_property.push(*event);
    }

    for event in update_property.iter() {
        let fallback_value = T::get_fallback(event.for_element, event.in_scenario, world);
        let (mut commands, current_scenario, get_modifier) = scenario_state.get(world);
        // Only update current scenario properties
        if !current_scenario.0.is_some_and(|e| e == event.in_scenario) {
            continue;
        }
        // Only update elements registered for this plugin
        if !values.get(world, event.for_element).is_ok() {
            continue;
        }

        let new_value: T =
            if let Some(modifier) = get_modifier.get(event.in_scenario, event.for_element) {
                (**modifier).clone()
            } else {
                // No modifier exists in this tree for this scenario/element pairing
                // Make sure that a modifier for this property exists in the current scenario tree
                if let Some(modifier_entity) =
                    get_modifier.scenarios.get(event.in_scenario).ok().and_then(
                        |(scenario_modifiers, _)| scenario_modifiers.get(&event.for_element),
                    )
                {
                    commands
                        .entity(**modifier_entity)
                        .insert(Modifier::<T>::new(fallback_value));
                    scenario_state.apply(world);
                } else {
                    // Modifier entity does not exist, add one
                    let root_modifier_entity =
                        commands.spawn(Modifier::<T>::new(fallback_value)).id();
                    scenario_state.apply(world);
                    let mut add_modifier = add_modifier_state.get_mut(world);
                    add_modifier.write(AddModifier::new_to_root(
                        event.for_element,
                        root_modifier_entity,
                        event.in_scenario,
                    ));
                }
                continue;
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
}

fn on_add_property<T: Property, F: QueryFilter + 'static + Send + Sync>(
    trigger: Trigger<OnAdd, T>,
    world: &mut World,
    state: &mut SystemState<(Query<&T, F>, Res<CurrentScenario>)>,
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

fn on_add_root_scenario<T: Property, F: QueryFilter + 'static + Send + Sync>(
    trigger: Trigger<OnAdd, ScenarioModifiers>,
    world: &mut World,
    state: &mut SystemState<Query<&Affiliation, With<ScenarioMarker>>>,
) {
    let scenarios = state.get_mut(world);
    if !scenarios.get(trigger.target()).is_ok_and(|p| p.0.is_none()) {
        return;
    }

    T::insert_on_new_scenario(trigger.target(), world);
}
