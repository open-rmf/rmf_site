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
        Change, DifferentialDrive, ExportHandler, ExportHandlers, ExportWith, Group, IsStatic,
        Mobility, ModelMarker, ModelProperty, Robot,
    },
    widgets::inspector::*,
};
use bevy::prelude::*;
use sdformat_rs::{ElementData, ElementMap, XmlElement};

pub struct SlotcarSdfPlugin;

impl Plugin for SlotcarSdfPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_slotcar_export_handler)
            .add_systems(
                PreUpdate,
                (
                    insert_slotcar_differential_drive,
                    update_slotcar_export_with,
                ),
            );
    }
}

pub fn insert_slotcar_differential_drive(
    mut commands: Commands,
    mut change_robot_property: EventWriter<Change<ModelProperty<Robot>>>,
    differential_drive: Query<(Entity, &DifferentialDrive), (Without<ModelMarker>, Without<Group>)>,
    is_static: Query<&ModelProperty<IsStatic>, (With<ModelMarker>, With<Group>)>,
    mobility: Query<&Mobility, (With<ModelMarker>, With<Group>)>,
    model_descriptions: Query<&ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    model_instances: ModelPropertyQuery<Robot>,
    child_of: Query<&ChildOf>,
) {
    for (e, diff_drive) in differential_drive.iter() {
        if !model_descriptions.get(e).is_ok() {
            // A non-description entity has the DifferentialDrive component, it could have been inserted into a
            // model instance descendent when processing importing robot plugins
            // Insert this component in the affiliated description and remove it from the original entity
            let mut description_entity: Option<Entity> = None;
            let mut target_entity: Entity = e;
            while let Ok(parent) = child_of.get(target_entity).map(|co| co.parent()) {
                if let Some(desc) = model_instances.get(parent).ok().and_then(|a| a.0) {
                    if !mobility.get(desc).is_ok() && is_static.get(desc).is_ok_and(|is| !is.0 .0) {
                        description_entity = Some(desc);
                    }
                    break;
                }
                target_entity = parent;
            }

            if let Some(desc) = description_entity {
                let robot = match model_descriptions.get(desc) {
                    Ok(ModelProperty(r)) => r.clone(),
                    Err(_) => Robot::default(),
                };
                serialize_and_change_robot_property_kind::<Mobility, DifferentialDrive>(
                    &mut change_robot_property,
                    diff_drive.clone(),
                    &robot,
                    desc,
                );
            }
            commands.entity(e).remove::<DifferentialDrive>();
        }
    }
}

pub fn update_slotcar_export_with(
    differential_drive: Query<(Entity, Ref<DifferentialDrive>), (With<ModelMarker>, With<Group>)>,
    mut export_with: Query<&mut ExportWith, (With<ModelMarker>, With<Group>)>,
    mut removals: RemovedComponents<DifferentialDrive>,
) {
    for desc_entity in removals.read() {
        if let Ok(mut desc_export) = export_with.get_mut(desc_entity) {
            let slotcar_label = "slotcar".to_string();
            desc_export.0.remove(&slotcar_label);
        }
    }

    for (desc_entity, diff_drive) in differential_drive.iter() {
        if diff_drive.is_changed() {
            if let Ok(mut desc_export) = export_with.get_mut(desc_entity) {
                if let Ok(diff_drive_value) = serde_json::to_value(diff_drive.clone()) {
                    desc_export
                        .0
                        .insert("slotcar".to_string(), diff_drive_value);
                }
            }
        }
    }
}

pub fn setup_slotcar_export_handler(world: &mut World) {
    world.resource_scope::<ExportHandlers, ()>(move |world, mut export_handlers| {
        export_handlers.insert(
            "slotcar".to_string(),
            ExportHandler::new(slotcar_export_handler, world),
        );
    });
}

#[derive(Clone, Debug)]
pub struct SlotcarParams {
    pub nominal_drive_speed: f32,
    pub nominal_drive_acceleration: f32,
    pub nominal_turn_speed: f32,
    pub nominal_turn_acceleration: f32,
    pub reversible: bool,
    pub nominal_voltage: f32,
    pub nominal_capacity: f32,
    pub charging_current: f32,
    pub friction_coefficient: f32,
    pub nominal_power: f32,
}

impl Default for SlotcarParams {
    fn default() -> Self {
        Self {
            nominal_drive_speed: 0.5,
            nominal_drive_acceleration: 0.25,
            nominal_turn_speed: 1.0,
            nominal_turn_acceleration: 1.5,
            reversible: false,
            nominal_voltage: 12.0,
            nominal_capacity: 24.0,
            charging_current: 5.0,
            friction_coefficient: 0.22,
            nominal_power: 20.0,
        }
    }
}

impl SlotcarParams {
    fn from_differential_drive(diff_drive: serde_json::Value) -> Self {
        let mut slotcar_params = Self::default();

        if let Some(reversible) = diff_drive.get("bidirectional").and_then(|b| match b {
            serde_json::Value::Bool(rev) => Some(rev),
            _ => None,
        }) {
            slotcar_params.reversible = reversible.clone();
        }
        if let Some(translational_speed) =
            diff_drive.get("translational_speed").and_then(|b| match b {
                serde_json::Value::Number(speed) => speed.as_f64(),
                _ => None,
            })
        {
            slotcar_params.nominal_drive_speed = translational_speed as f32;
        }
        if let Some(translational_acceleration) = diff_drive
            .get("translational_acceleration")
            .and_then(|b| match b {
                serde_json::Value::Number(acceleration) => acceleration.as_f64(),
                _ => None,
            })
        {
            slotcar_params.nominal_drive_acceleration = translational_acceleration as f32;
        }
        if let Some(rotational_speed) = diff_drive.get("rotational_speed").and_then(|b| match b {
            serde_json::Value::Number(speed) => speed.as_f64(),
            _ => None,
        }) {
            slotcar_params.nominal_turn_speed = rotational_speed as f32;
        }
        if let Some(rotational_acceleration) =
            diff_drive
                .get("rotational_acceleration")
                .and_then(|b| match b {
                    serde_json::Value::Number(acceleration) => acceleration.as_f64(),
                    _ => None,
                })
        {
            slotcar_params.nominal_turn_acceleration = rotational_acceleration as f32;
        }

        slotcar_params
    }

    fn into_xml(&self) -> XmlElement {
        let mut element_map = ElementMap::default();

        element_map.push(XmlElement {
            name: "nominal_drive_speed".into(),
            data: ElementData::String(self.nominal_drive_speed.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "nominal_drive_acceleration".into(),
            data: ElementData::String(self.nominal_drive_acceleration.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "nominal_turn_speed".into(),
            data: ElementData::String(self.nominal_turn_speed.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "nominal_turn_acceleration".into(),
            data: ElementData::String(self.nominal_turn_acceleration.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "reversible".into(),
            data: ElementData::String(format!("{}", self.reversible)),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "nominal_voltage".into(),
            data: ElementData::String(self.nominal_voltage.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "nominal_capacity".into(),
            data: ElementData::String(self.nominal_capacity.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "charging_current".into(),
            data: ElementData::String(self.charging_current.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "friction_coefficient".into(),
            data: ElementData::String(self.friction_coefficient.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "nominal_power".into(),
            data: ElementData::String(self.nominal_power.to_string()),
            ..Default::default()
        });

        XmlElement {
            data: ElementData::Nested(element_map),
            ..Default::default()
        }
    }
}

fn slotcar_export_handler(In(input): In<(Entity, serde_json::Value)>) -> sdformat_rs::XmlElement {
    let (_, diff_drive_config) = input;
    SlotcarParams::from_differential_drive(diff_drive_config).into_xml()
}
