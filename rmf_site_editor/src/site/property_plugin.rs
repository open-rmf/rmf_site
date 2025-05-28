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
        Affiliation, ChangeCurrentScenario, CurrentScenario, IssueKey, NameInSite, ScenarioMarker,
        ScenarioModifiers,
    },
    Issue, ValidateWorkspace,
};
use bevy::{
    ecs::{
        component::Mutable,
        hierarchy::ChildOf,
        query::QueryFilter,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use std::fmt::Debug;
use uuid::Uuid;

pub trait Property: Component<Mutability = Mutable> + Debug + Default + Clone {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self;
}

pub trait StandardProperty: Component<Mutability = Mutable> + Debug + Default + Clone {}

impl<T: StandardProperty> Property for T {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Self {
        T::default()
    }
}

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

    fn insert_modifier(_for_element: Entity, _in_scenario: Entity, _value: T, _world: &mut World) {}
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

pub struct PropertyPlugin<T: Property, S: Modifier<T>, F: QueryFilter + 'static + Send + Sync> {
    _ignore: std::marker::PhantomData<(T, S, F)>,
}

impl<T: Property, S: Modifier<T>, F: QueryFilter + 'static + Send + Sync> Default
    for PropertyPlugin<T, S, F>
{
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

// impl<T: Property, S: Modifier<T>, M: Component<Mutability = Mutable>> Plugin
impl<T: Property, S: Modifier<T>, F: QueryFilter + 'static + Send + Sync> Plugin
    for PropertyPlugin<T, S, F>
{
    fn build(&self, app: &mut App) {
        app.add_event::<UpdateProperty>()
            .add_observer(update_property_value::<T, S, F>)
            .add_observer(on_add_property::<T, S, F>);
    }
}

fn update_property_value<T: Property, S: Modifier<T>, F: QueryFilter + 'static + Send + Sync>(
    trigger: Trigger<UpdateProperty>,
    world: &mut World,
    state: &mut SystemState<(
        Query<&mut T, F>,
        EventWriter<AddModifier>,
        Res<CurrentScenario>,
        GetModifier<S>,
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
        let root_modifier_entity = world.commands().spawn(S::default()).id();
        let (_, mut add_modifier, _, _) = state.get_mut(world);
        add_modifier.write(AddModifier::new_to_root(
            event.for_element,
            root_modifier_entity,
            event.in_scenario,
        ));
        return;
    };

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

fn on_add_property<T: Property, S: Modifier<T>, F: QueryFilter + 'static + Send + Sync>(
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
    S::insert_modifier(trigger.target(), scenario_entity, value.clone(), world);
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

#[derive(Component, Clone, Debug)]
pub struct LastSetValue<T: Property>(pub T);

impl<T: Property> LastSetValue<T> {
    pub fn new(value: T) -> Self {
        Self(value)
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
        // If a modifier entity already exists, we assume it's meant to replace the current modifier.
        // Despawn incoming modifier entity. Note that this erases any Recall data
        if let Some(current_modifier) = scenario_modifiers.get(&add.for_element) {
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

/// Unique UUID to identify issue of missing root scenario modifiers
pub const MISSING_ROOT_MODIFIER_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x98df792d3de44d26b126a9335f9e743au128);

pub fn check_for_missing_root_modifiers<M: Component<Mutability = Mutable>>(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    scenarios: Query<
        (
            &ScenarioModifiers<Entity>,
            &NameInSite,
            &Affiliation<Entity>,
        ),
        With<ScenarioMarker>,
    >,
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
