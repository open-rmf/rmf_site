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
    update_model_instances, Change, Group, ModelMarker, ModelProperty, ModelPropertyData, Robot,
};
use bevy::prelude::*;
use rmf_site_format::robot_properties::*;
use serde_json::Map;
use std::fmt::Debug;

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
        app.add_event::<UpdateRobotPropertyKinds>()
            .add_systems(PostUpdate, update_model_instances::<Robot>);
    }
}

// TODO(@xiyuoh) Combine on_insert and on_remove observers for RobotProperty
// when multi-event observers become available (see
// https://github.com/bevyengine/bevy/issues/14649)

/// Monitors changes in a description's ModelProperty<Robot> and inserts the
/// updated RobotProperty components accordingly
pub fn on_insert_robot_property<T: RobotProperty>(
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
