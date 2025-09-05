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
        app.world_mut().register_component::<ModelProperty<Robot>>();
        let component_id = app
            .world()
            .components()
            .component_id::<ModelProperty<Robot>>()
            .unwrap();
        app.init_resource::<ModelPropertyData>()
            .world_mut()
            .resource_mut::<ModelPropertyData>()
            .optional
            .insert(
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
        app.add_event::<UpdateRobotPropertyKinds>()
            .add_systems(PreUpdate, update_model_instances::<Robot>);
    }
}

/// This system monitors changes to ModelProperty<Robot> and inserts robot property components
/// to the model descriptions
pub fn update_robot_property_components<T: RobotProperty>(
    mut commands: Commands,
    model_properties: Query<(Entity, Ref<ModelProperty<Robot>>), (With<ModelMarker>, With<Group>)>,
    mut removals: RemovedComponents<ModelProperty<Robot>>,
    mut update_robot_property_kinds: EventWriter<UpdateRobotPropertyKinds>,
) {
    let property_label = T::label();

    // Remove Robot property entirely
    for description_entity in removals.read() {
        commands.entity(description_entity).remove::<T>();
        update_robot_property_kinds.write(UpdateRobotPropertyKinds {
            entity: description_entity,
            label: property_label.clone(),
            value: serde_json::Value::Object(Map::new()),
        });
    }

    for (entity, robot) in model_properties.iter() {
        if robot.is_changed() {
            // Update robot property
            let value = match retrieve_robot_property::<T>(robot.0.clone()) {
                Ok((property, value)) => {
                    commands.entity(entity).insert(property);
                    value
                }
                Err(RobotPropertyError::PropertyNotFound(_)) => {
                    commands.entity(entity).remove::<T>();
                    serde_json::Value::Object(Map::new())
                }
                Err(_) => continue,
            };

            // Update robot property kinds
            update_robot_property_kinds.write(UpdateRobotPropertyKinds {
                entity,
                label: property_label.clone(),
                value,
            });
        }
    }
}

/// This system inserts or removes robot property kind components when a robot property is updated
pub fn update_robot_property_kind_components<
    Kind: RobotPropertyKind,
    Property: RobotProperty,
    RecallKind: RecallPropertyKind<Kind = Kind>,
>(
    mut commands: Commands,
    mut update_robot_property_kinds: EventReader<UpdateRobotPropertyKinds>,
    recall_kind: Query<&RecallKind, (With<ModelMarker>, With<Group>)>,
) {
    for update in update_robot_property_kinds.read() {
        if update.label != Property::label() {
            continue;
        }

        match retrieve_robot_property_kind::<Kind, Property, RecallKind>(
            update.value.clone(),
            recall_kind.get(update.entity).ok(),
        ) {
            Ok(property_kind) => {
                commands.entity(update.entity).insert(property_kind);
            }
            Err(RobotPropertyError::PropertyKindNotFound(_)) => {
                commands.entity(update.entity).remove::<Kind>();
            }
            Err(_) => continue,
        }
    }
}

/// This system updates ModelProperty<Robot> based on updates to the RobotProperty components
pub fn serialize_and_change_robot_property<Property: RobotProperty>(
    change_robot_property: &mut EventWriter<Change<ModelProperty<Robot>>>,
    property: Property,
    robot: &Robot,
    description_entity: Entity,
) {
    if let Ok(new_property_value) = serialize_robot_property::<Property>(property) {
        let mut new_robot = robot.clone();
        new_robot
            .properties
            .insert(Property::label(), new_property_value);
        change_robot_property.write(Change::new(ModelProperty(new_robot), description_entity));
    }
}

/// This system updates ModelProperty<Robot> based on updates to the RobotProperty and
/// RobotPropertyKind components
pub fn serialize_and_change_robot_property_kind<
    Property: RobotProperty,
    Kind: RobotPropertyKind,
>(
    change_robot_property: &mut EventWriter<Change<ModelProperty<Robot>>>,
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
        change_robot_property.write(Change::new(ModelProperty(new_robot), description_entity));
    }
}
