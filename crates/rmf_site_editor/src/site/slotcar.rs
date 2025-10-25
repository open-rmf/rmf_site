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
    Battery, Change, DifferentialDrive, ExportHandler, ExportHandlers, ExportWith, Group, IsStatic,
    MechanicalSystem, Mobility, ModelMarker, ModelProperty, ModelPropertyQuery, PowerSource, Robot,
    RobotProperty, RobotPropertyKind,
};
use bevy::{ecs::relationship::AncestorIter, prelude::*};
use rmf_site_format::robot_properties::*;
use sdformat::{ElementData, ElementMap, XmlElement};
use serde_json::{Map, Value};

pub struct SlotcarSdfPlugin;

impl Plugin for SlotcarSdfPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_slotcar_export_handler)
            .add_systems(
                PostUpdate,
                (
                    insert_slotcar_components,
                    update_slotcar_export_with::<DifferentialDrive>,
                    update_slotcar_export_with::<Battery>,
                    update_slotcar_export_with::<AmbientSystem>,
                    update_slotcar_export_with::<MechanicalSystem>,
                ),
            );
    }
}

/// When loading SDF models, some RobotPropertyKinds may be directly inserted into
/// a model instance descendant instead of the instance or affiliated description.
/// This system checks for such cases and inserts the respective components into
/// the affiliated model description (if there is no other RobotProperty data
/// present) and remove it from the original entity.
fn insert_slotcar_components(
    mut commands: Commands,
    slotcar_property_kinds: Query<
        // All 4 components will be present (see sdf_loader.rs)
        (
            Entity,
            &DifferentialDrive,
            &Battery,
            &AmbientSystem,
            &MechanicalSystem,
        ),
        (Without<ModelMarker>, Without<Group>),
    >,
    model_descriptions: Query<
        (
            Option<&ModelProperty<Robot>>,
            Option<&ModelProperty<IsStatic>>,
        ),
        (With<ModelMarker>, With<Group>),
    >,
    model_instances: ModelPropertyQuery<Robot>,
    child_of: Query<&ChildOf>,
) {
    for (e, differential_drive, battery, ambient_system, mechanical_system) in
        slotcar_property_kinds.iter()
    {
        let description_entity = AncestorIter::new(&child_of, e)
            .find(|p| model_instances.get(*p).is_ok())
            .and_then(|parent| model_instances.get(parent).ok().and_then(|a| a.0));

        if let Some(((opt_desc_robot, opt_desc_is_static), desc)) =
            description_entity.and_then(|e| model_descriptions.get(e).ok().zip(Some(e)))
        {
            let mut robot = Robot::default();

            if let Some(desc_robot) = opt_desc_robot {
                if !desc_robot.0.properties.is_empty() {
                    // The robot properties for this model description has already
                    // been populated, do not overwrite
                    commands
                        .entity(e)
                        .remove::<DifferentialDrive>()
                        .remove::<Battery>()
                        .remove::<AmbientSystem>()
                        .remove::<MechanicalSystem>();
                    continue;
                }
                robot = desc_robot.0.clone();
            }

            // Only insert Mobility if robot is not static
            if !opt_desc_is_static.is_some_and(|is_static| is_static.0 .0) {
                if let Ok(mobility_value) = serialize_robot_property_from_kind::<
                    Mobility,
                    DifferentialDrive,
                >(differential_drive.clone())
                {
                    robot.properties.insert(Mobility::label(), mobility_value);
                }
            }

            if let Ok(power_source_value) =
                serialize_robot_property_from_kind::<PowerSource, Battery>(battery.clone())
            {
                robot
                    .properties
                    .insert(PowerSource::label(), power_source_value);
            }

            let mut power_dissipation_map = Map::new();
            if let Ok(mechanical_system_value) =
                serialize_robot_property_kind::<MechanicalSystem>(mechanical_system.clone())
            {
                power_dissipation_map.insert(MechanicalSystem::label(), mechanical_system_value);
            }
            if let Ok(ambient_system_value) =
                serialize_robot_property_kind::<AmbientSystem>(ambient_system.clone())
            {
                power_dissipation_map.insert(AmbientSystem::label(), ambient_system_value);
            }
            if !power_dissipation_map.is_empty() {
                let mut power_dissipation_config = Map::new();
                power_dissipation_config
                    .insert("config".to_string(), Value::Object(power_dissipation_map));
                robot.properties.insert(
                    PowerDissipation::label(),
                    Value::Object(power_dissipation_config),
                );
            }

            commands.trigger(Change::new(ModelProperty(robot), desc).or_insert());
        }
        commands
            .entity(e)
            .remove::<DifferentialDrive>()
            .remove::<Battery>()
            .remove::<AmbientSystem>()
            .remove::<MechanicalSystem>();
    }
}

pub fn update_slotcar_export_with<T: RobotPropertyKind>(
    robot_property_kind: Query<(Entity, Ref<T>), (With<ModelMarker>, With<Group>)>,
    mut export_with: Query<&mut ExportWith, (With<ModelMarker>, With<Group>)>,
    mut removals: RemovedComponents<T>,
) {
    let slotcar_label = "slotcar".to_string();

    for desc_entity in removals.read() {
        if let Ok(mut desc_export) = export_with.get_mut(desc_entity) {
            let slotcar_entry = desc_export
                .0
                .entry(slotcar_label.clone())
                .or_insert(Value::Object(Map::new()));

            if let Some(entry_map) = slotcar_entry.as_object_mut() {
                entry_map.remove(&T::label());
            }
            // If slotcar entry is empty, remove label entirely
            if slotcar_entry.as_object().is_some_and(|obj| obj.is_empty()) {
                desc_export.0.remove(&slotcar_label);
            }
        }
    }

    for (desc_entity, property_kind) in robot_property_kind.iter() {
        if property_kind.is_changed() {
            if let Ok(mut desc_export) = export_with.get_mut(desc_entity) {
                let slotcar_entry = desc_export
                    .0
                    .entry(slotcar_label.clone())
                    .or_insert(Value::Object(Map::new()));

                if let Some(entry_map) = slotcar_entry.as_object_mut() {
                    if let Ok(value) = serde_json::to_value(property_kind.clone()) {
                        entry_map.insert(T::label(), value);
                    }
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
    pub mass: f32,
    pub inertia: f32,
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
            mass: 20.0,
            inertia: 10.0,
            friction_coefficient: 0.22,
            nominal_power: 20.0,
        }
    }
}

impl SlotcarParams {
    fn with_differential_drive(mut self, diff_drive: &Value) -> Self {
        if let Some(reversible) = diff_drive.get("bidirectional").and_then(|b| match b {
            Value::Bool(rev) => Some(rev),
            _ => None,
        }) {
            self.reversible = reversible.clone();
        }
        if let Some(translational_speed) =
            diff_drive.get("translational_speed").and_then(|b| match b {
                Value::Number(speed) => speed.as_f64(),
                _ => None,
            })
        {
            self.nominal_drive_speed = translational_speed as f32;
        }
        if let Some(rotational_speed) = diff_drive.get("rotational_speed").and_then(|b| match b {
            Value::Number(speed) => speed.as_f64(),
            _ => None,
        }) {
            self.nominal_turn_speed = rotational_speed as f32;
        }

        self
    }

    fn with_battery(mut self, battery: &Value) -> Self {
        if let Some(voltage) = battery.get("voltage").and_then(|v| match v {
            Value::Number(vol) => vol.as_f64(),
            _ => None,
        }) {
            self.nominal_voltage = voltage as f32;
        }
        if let Some(capacity) = battery.get("capacity").and_then(|c| match c {
            Value::Number(capacity) => capacity.as_f64(),
            _ => None,
        }) {
            self.nominal_capacity = capacity as f32;
        }
        if let Some(charging_current) = battery.get("charging_current").and_then(|c| match c {
            Value::Number(current) => current.as_f64(),
            _ => None,
        }) {
            self.charging_current = charging_current as f32;
        }
        if let Some(power) = battery.get("power").and_then(|p| match p {
            Value::Number(power) => power.as_f64(),
            _ => None,
        }) {
            self.nominal_power = power as f32;
        }

        self
    }

    fn with_mechanical_system(mut self, mechanical_system: &Value) -> Self {
        if let Some(mass) = mechanical_system.get("mass").and_then(|m| match m {
            Value::Number(mass) => mass.as_f64(),
            _ => None,
        }) {
            self.mass = mass as f32;
        }
        if let Some(moment_of_inertia) =
            mechanical_system
                .get("moment_of_inertia")
                .and_then(|i| match i {
                    Value::Number(inertia) => inertia.as_f64(),
                    _ => None,
                })
        {
            self.inertia = moment_of_inertia as f32;
        }
        if let Some(friction_coefficient) =
            mechanical_system
                .get("friction_coefficient")
                .and_then(|c| match c {
                    Value::Number(coefficient) => coefficient.as_f64(),
                    _ => None,
                })
        {
            self.friction_coefficient = friction_coefficient as f32;
        }

        self
    }

    fn with_ambient_system(mut self, ambient_system: &Value) -> Self {
        if let Some(nominal_power) = ambient_system.get("idle_power").and_then(|p| match p {
            Value::Number(power) => power.as_f64(),
            _ => None,
        }) {
            self.nominal_power = nominal_power as f32;
        }

        self
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
            name: "mass".into(),
            data: ElementData::String(self.mass.to_string()),
            ..Default::default()
        });
        element_map.push(XmlElement {
            name: "inertia".into(),
            data: ElementData::String(self.inertia.to_string()),
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

fn slotcar_export_handler(In(input): In<(Entity, Value)>) -> sdformat::XmlElement {
    let (_, slotcar_config) = input;
    let mut slotcar_params = SlotcarParams::default();

    if let Some(config_map) = slotcar_config.as_object() {
        if let Some(diff_drive_config) = config_map.get(&DifferentialDrive::label()) {
            slotcar_params = slotcar_params.with_differential_drive(diff_drive_config);
        }
        if let Some(battery_config) = config_map.get(&Battery::label()) {
            slotcar_params = slotcar_params.with_battery(battery_config);
        }
        if let Some(ambient_sys_config) = config_map.get(&AmbientSystem::label()) {
            slotcar_params = slotcar_params.with_ambient_system(ambient_sys_config);
        }
        if let Some(mech_sys_config) = config_map.get(&MechanicalSystem::label()) {
            slotcar_params = slotcar_params.with_mechanical_system(mech_sys_config);
        }
    }

    slotcar_params.into_xml()
}
