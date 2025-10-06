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
    update_model_instances, Change, Group, ModelMarker, ModelProperty, ModelPropertyData, Recall,
    RecallPlugin, Robot,
};
use bevy::{ecs::component::Mutable, prelude::*};
use rmf_site_format::robot_properties::*;
use serde_json::{Error, Map, Value};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Event)]
pub struct UpdateRobotPropertyKinds {
    pub entity: Entity,
    pub label: String,
    pub value: serde_json::Value,
}

#[derive(Default)]
pub struct RobotPropertiesPlugin {}

impl Plugin for RobotPropertiesPlugin {
    fn build(&self, app: &mut App) {
        // Allows us to toggle Robot as a configurable property
        // from the model description inspector
        let component_id = app.world_mut().register_component::<ModelProperty<Robot>>();
        let mut model_property_data = ModelPropertyData::from_world(app.world_mut());
        model_property_data.optional.insert(
            component_id,
            (
                "Robot".to_string(),
                |mut e_cmd| {
                    e_cmd.insert(ModelProperty::<Robot>::default());
                },
                |mut e_cmd| {
                    e_cmd.remove::<ModelProperty<Robot>>();
                },
            ),
        );
        app.insert_resource(model_property_data);
        app.world_mut().init_resource::<RobotPropertyRegistry>();
        app.add_event::<UpdateRobotPropertyKinds>()
            .add_systems(PostUpdate, update_model_instances::<Robot>);
    }
}

type InsertDefaultValueFn = fn() -> Result<Value, Error>;

pub struct RobotPropertyRegistration {
    pub widget: Option<Entity>,
    pub kinds: HashMap<String, RobotPropertyKindRegistration>,
}

pub struct RobotPropertyKindRegistration {
    pub default: InsertDefaultValueFn,
}

/// This resource keeps track of all the properties that can be configured for a robot.
#[derive(Resource, Deref, DerefMut)]
pub struct RobotPropertyRegistry(pub HashMap<String, RobotPropertyRegistration>);

impl FromWorld for RobotPropertyRegistry {
    fn from_world(_world: &mut World) -> Self {
        Self(HashMap::new())
    }
}

/// Implement this plugin to add a new configurable robot property to the site
pub struct RobotPropertyPlugin<Property, RecallProperty>
where
    Property: RobotProperty,
    RecallProperty: Recall + Component<Mutability = Mutable> + Default,
    RecallProperty::Source: RobotProperty,
{
    _ignore: std::marker::PhantomData<(Property, RecallProperty)>,
}

impl<Property, RecallProperty> Default for RobotPropertyPlugin<Property, RecallProperty>
where
    Property: RobotProperty,
    RecallProperty: Recall + Component<Mutability = Mutable> + Default,
    RecallProperty::Source: RobotProperty,
{
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<Property, RecallProperty> Plugin for RobotPropertyPlugin<Property, RecallProperty>
where
    Property: RobotProperty,
    RecallProperty: Recall + Component<Mutability = Mutable> + Default,
    RecallProperty::Source: RobotProperty,
{
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<RobotPropertyRegistry>()
            .0
            .insert(
                Property::label(),
                RobotPropertyRegistration {
                    widget: None,
                    kinds: HashMap::new(),
                },
            );
        app.add_observer(on_add_robot_property::<Property>)
            .add_observer(on_change_robot_property::<Property>)
            .add_observer(on_remove_robot_property::<Property>)
            .add_plugins(RecallPlugin::<RecallProperty>::default());
    }
}

/// Implement this plugin to add a new configurable robot property kind to the site
pub struct RobotPropertyKindPlugin<Kind, Property, RecallKind>
where
    Kind: RobotPropertyKind,
    Property: RobotProperty,
    RecallKind: RecallPropertyKind<Kind = Kind>,
    RecallKind::Source: RobotPropertyKind,
{
    _ignore: std::marker::PhantomData<(Kind, Property, RecallKind)>,
}

impl<Kind, Property, RecallKind> Default for RobotPropertyKindPlugin<Kind, Property, RecallKind>
where
    Kind: RobotPropertyKind,
    Property: RobotProperty,
    RecallKind: RecallPropertyKind<Kind = Kind>,
    RecallKind::Source: RobotPropertyKind,
{
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<Kind, Property, RecallKind> Plugin for RobotPropertyKindPlugin<Kind, Property, RecallKind>
where
    Kind: RobotPropertyKind,
    Property: RobotProperty,
    RecallKind: RecallPropertyKind<Kind = Kind>,
    RecallKind::Source: RobotPropertyKind,
{
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<RobotPropertyRegistry>()
            .0
            .get_mut(&Property::label())
            .map(|registration| {
                registration.kinds.insert(
                    Kind::label(),
                    RobotPropertyKindRegistration {
                        default: || serde_json::to_value(Kind::default()),
                    },
                );
            });
        app.add_observer(on_update_robot_property_kind::<Kind, Property, RecallKind>)
            .add_plugins(RecallPlugin::<RecallKind>::default());
    }
}

#[derive(Default)]
pub struct EmptyRobotPropertyPlugin<T: RobotProperty> {
    _ignore: std::marker::PhantomData<T>,
}

impl<T: RobotProperty> EmptyRobotPropertyPlugin<T> {
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<T: RobotProperty> Plugin for EmptyRobotPropertyPlugin<T> {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<RobotPropertyRegistry>()
            .0
            .get_mut(&T::label())
            .map(|registration| {
                registration.kinds.insert(
                    EmptyRobotProperty::<T>::label(),
                    RobotPropertyKindRegistration {
                        default: || Ok(Value::Null),
                    },
                );
            });
        app.add_observer(on_empty_robot_property::<T>);
    }
}

// TODO(@xiyuoh) Combine on_insert, on_change and on_remove observers for
// RobotProperty when multi-event observers become available (see
// https://github.com/bevyengine/bevy/issues/14649)

/// Monitors newly added ModelProperty<Robot> and inserts the relevant
/// RobotProperty components accordingly
pub fn on_add_robot_property<T: RobotProperty>(
    trigger: Trigger<OnAdd, ModelProperty<Robot>>,
    model_properties: Query<&ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    mut commands: Commands,
) {
    let description_entity = trigger.target();
    let Ok(robot) = model_properties
        .get(description_entity)
        .map(|robot| robot.0.clone())
    else {
        return;
    };

    // Update robot property
    let value = match retrieve_robot_property::<T>(robot) {
        Ok((property, value)) => {
            commands.entity(description_entity).insert(property);
            value
        }
        Err(RobotPropertyError::PropertyNotFound(_)) => {
            commands.entity(description_entity).remove::<T>();
            serde_json::Value::Object(Map::new())
        }
        Err(_) => return,
    };

    // Update robot property kinds
    commands.trigger(UpdateRobotPropertyKinds {
        entity: description_entity,
        label: T::label(),
        value,
    });
}

/// Monitors changes in a description's ModelProperty<Robot> and inserts the
/// updated RobotProperty components accordingly
pub fn on_change_robot_property<T: RobotProperty>(
    trigger: Trigger<Change<ModelProperty<Robot>>>,
    mut commands: Commands,
) {
    let description_entity = trigger.for_element;
    let robot = trigger.to_value.clone();

    // Update robot property
    let value = match retrieve_robot_property::<T>(robot.0) {
        Ok((property, value)) => {
            commands.entity(description_entity).insert(property);
            value
        }
        Err(RobotPropertyError::PropertyNotFound(_)) => {
            commands.entity(description_entity).remove::<T>();
            serde_json::Value::Object(Map::new())
        }
        Err(_) => return,
    };

    // Update robot property kinds
    commands.trigger(UpdateRobotPropertyKinds {
        entity: description_entity,
        label: T::label(),
        value,
    });
}

/// Monitors removals of a description's ModelProperty<Robot> and inserts the
/// updated RobotProperty components accordingly
pub fn on_remove_robot_property<T: RobotProperty>(
    trigger: Trigger<OnRemove, ModelProperty<Robot>>,
    mut commands: Commands,
) {
    let description_entity = trigger.target();
    commands.entity(description_entity).remove::<T>();
    commands.trigger(UpdateRobotPropertyKinds {
        entity: description_entity,
        label: T::label(),
        value: serde_json::Value::Object(Map::new()),
    });
}

/// Monitors updates to a description's RobotProperty components and updates its
/// RobotPropertyKind components accordingly
pub fn on_update_robot_property_kind<
    Kind: RobotPropertyKind,
    Property: RobotProperty,
    RecallKind: RecallPropertyKind<Kind = Kind>,
>(
    trigger: Trigger<UpdateRobotPropertyKinds>,
    mut commands: Commands,
    recall_kind: Query<&RecallKind, (With<ModelMarker>, With<Group>)>,
) {
    let event = trigger.event();
    if event.label != Property::label() {
        return;
    }

    match retrieve_robot_property_kind::<Kind, Property, RecallKind>(
        event.value.clone(),
        recall_kind.get(event.entity).ok(),
    ) {
        Ok(property_kind) => {
            commands.entity(event.entity).insert(property_kind);
        }
        Err(RobotPropertyError::PropertyKindNotFound(_)) => {
            commands.entity(event.entity).remove::<Kind>();
        }
        Err(_) => return,
    }
}

pub fn on_empty_robot_property<T: RobotProperty>(
    trigger: Trigger<UpdateRobotPropertyKinds>,
    mut commands: Commands,
) {
    let event = trigger.event();
    let empty_label = EmptyRobotProperty::<T>::label();
    if event.label != T::label() {
        return;
    }

    // Check if config contains EmptyRobotProperty
    if event
        .value
        .as_object()
        .and_then(|m| m.get("kind"))
        .and_then(|kind| kind.as_str())
        .is_some_and(|kind| kind == &empty_label)
    {
        commands
            .entity(event.entity)
            .insert(EmptyRobotProperty::<T>::default());
    } else {
        commands
            .entity(event.entity)
            .remove::<EmptyRobotProperty<T>>();
    }
}

/// This system updates ModelProperty<Robot> based on updates to the RobotProperty components
pub fn serialize_and_change_robot_property<Property: RobotProperty>(
    commands: &mut Commands,
    property: Property,
    robot: &Robot,
    description_entity: Entity,
) {
    if let Ok(new_property_value) = serialize_robot_property::<Property>(property) {
        let mut new_robot = robot.clone();
        new_robot
            .properties
            .insert(Property::label(), new_property_value);
        commands.trigger(Change::new(ModelProperty(new_robot), description_entity));
    }
}

/// This system updates ModelProperty<Robot> based on updates to the RobotProperty and
/// RobotPropertyKind components
pub fn serialize_and_change_robot_property_kind<
    Property: RobotProperty,
    Kind: RobotPropertyKind,
>(
    commands: &mut Commands,
    property_kind: Kind,
    robot: &Robot,
    description_entity: Entity,
) {
    if let Ok(new_property_value) =
        serialize_robot_property_from_kind::<Property, Kind>(property_kind)
    {
        let mut new_robot = robot.clone();
        new_robot
            .properties
            .insert(Property::label(), new_property_value);
        commands.trigger(Change::new(ModelProperty(new_robot), description_entity));
    }
}
