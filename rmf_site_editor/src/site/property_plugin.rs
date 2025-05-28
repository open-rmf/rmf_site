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
};
use bevy::{
    ecs::{component::Mutable, query::QueryFilter, system::SystemState},
    prelude::*,
};
use std::fmt::Debug;

pub trait Property: Component<Mutability = Mutable> + Debug + Default + Clone {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self;
}

pub trait StandardProperty: Component<Mutability = Mutable> + Debug + Default + Clone {}

impl<T: StandardProperty> Property for T {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self {
        T::default()
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

#[derive(Component, Clone, Debug)]
pub struct LastSetValue<T: Property>(pub T);

impl<T: Property> LastSetValue<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

pub struct PropertyPlugin<T: Property, M: Modifier<T>, F: QueryFilter + 'static + Send + Sync> {
    _ignore: std::marker::PhantomData<(T, M, F)>,
}

impl<T: Property, M: Modifier<T>, F: QueryFilter + 'static + Send + Sync> Default
    for PropertyPlugin<T, M, F>
{
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<T: Property, M: Modifier<T>, F: QueryFilter + 'static + Send + Sync> Plugin
    for PropertyPlugin<T, M, F>
{
    fn build(&self, app: &mut App) {
        app.add_event::<UpdateProperty>()
            .add_observer(update_property_value::<T, M, F>)
            .add_observer(on_add_property::<T, M, F>)
            .add_observer(on_add_root_scenario::<T, M, F>);
    }
}

fn update_property_value<T: Property, M: Modifier<T>, F: QueryFilter + 'static + Send + Sync>(
    trigger: Trigger<UpdateProperty>,
    world: &mut World,
    state: &mut SystemState<(
        Query<&mut T, F>,
        EventWriter<AddModifier>,
        Res<CurrentScenario>,
        GetModifier<M>,
    )>,
) {
    let (_, _, current_scenario, get_modifier) = state.get_mut(world);
    let event = trigger.event();
    // Only update current scenario properties
    if !current_scenario.0.is_some_and(|e| e == event.in_scenario) {
        return;
    }

    let new_value = if let Some(modifier) = get_modifier.get(event.in_scenario, event.for_element) {
        modifier
            .get()
            .or_else(|| {
                modifier.retrieve_inherited(event.for_element, event.in_scenario, &get_modifier)
            })
            .unwrap_or(T::get_fallback(event.for_element, event.in_scenario, world))
    } else {
        // No modifier exists for this scenario/element pairing
        // Make sure that a modifier exists in the current scenario tree
        let root_modifier_entity = world.commands().spawn(M::default()).id();
        let (_, mut add_modifier, _, _) = state.get_mut(world);
        add_modifier.write(AddModifier::new_to_root(
            event.for_element,
            root_modifier_entity,
            event.in_scenario,
        ));
        return;
    };

    // TODO(@xiyuoh) Pose and visibility changes are currently not being updated all at one go
    // Consider using EventReader instead of observer for value updates
    let (mut values, _, _, _) = state.get_mut(world);
    let changed = values.get_mut(event.for_element).is_ok_and(|mut value| {
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

fn on_add_property<T: Property, M: Modifier<T>, F: QueryFilter + 'static + Send + Sync>(
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
    M::insert(trigger.target(), scenario_entity, value.clone(), world);
}

fn on_add_root_scenario<T: Property, M: Modifier<T>, F: QueryFilter + 'static + Send + Sync>(
    trigger: Trigger<OnAdd, ScenarioMarker>,
    world: &mut World,
    state: &mut SystemState<Query<&Affiliation<Entity>, With<ScenarioMarker>>>,
) {
    let scenarios = state.get_mut(world);
    if !scenarios.get(trigger.target()).is_ok_and(|p| p.0.is_none()) {
        return;
    }

    M::insert_on_new_scenario(trigger.target(), world);
}
