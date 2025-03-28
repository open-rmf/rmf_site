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
        Change, ExportHandler, ExportHandlers, ExportWith, Group, IsStatic, ModelMarker,
        ModelProperty, Robot,
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
    parents: Query<&Parent>,
) {
    for (e, diff_drive) in differential_drive.iter() {
        if !model_descriptions.get(e).is_ok() {
            // A non-description entity has the DifferentialDrive component, it could have been inserted into a
            // model instance descendent when processing importing robot plugins
            // Insert this component in the affiliated description and remove it from the original entity
            let mut description_entity: Option<Entity> = None;
            let mut target_entity: Entity = e;
            while let Ok(parent) = parents.get(target_entity).map(|p| p.get()) {
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
                serialize_and_change_robot_property::<Mobility, DifferentialDrive>(
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
    // Remove from ExportWith
    for desc_entity in removals.read() {
        if let Ok(mut desc_export) = export_with.get_mut(desc_entity) {
            let slotcar_label = "slotcar".to_string();
            desc_export.0.remove(&slotcar_label);
        }
    }

    // Add to ExportWith
    for (desc_entity, diff_drive) in differential_drive.iter() {
        if diff_drive.is_changed() {
            // check ExportWith of this description
            if let Ok(mut desc_export) = export_with.get_mut(desc_entity) {
                // Serialize DifferentialDrive
                if let Ok(diff_drive_value) = serde_json::to_value(diff_drive.clone()) {
                    desc_export
                        .0
                        .insert("slotcar".to_string(), diff_drive_value);
                }
            }
        }
    }
}

pub fn setup_slotcar_export_handler(mut export_handlers: ResMut<ExportHandlers>) {
    export_handlers.insert(
        "slotcar".to_string(),
        ExportHandler(Box::new(IntoSystem::into_system(slotcar_export_handler))),
    );
}

fn slotcar_export_handler(In(input): In<(Entity, serde_json::Value)>) -> sdformat_rs::XmlElement {
    let (_entity, diff_drive_config) = input;
    let mut element_map = ElementMap::default();

    if let Some(reversible) = diff_drive_config
        .get("bidirectional")
        .and_then(|b| match b {
            serde_json::Value::Bool(rev) => Some(rev),
            _ => None,
        })
    {
        element_map.push(XmlElement {
            name: "reversible".into(),
            data: ElementData::String(format!("{}", reversible)),
            ..Default::default()
        })
    }
    if let Some(translational_speed) =
        diff_drive_config
            .get("translational_speed")
            .and_then(|b| match b {
                serde_json::Value::Number(speed) => speed.as_f64(),
                _ => None,
            })
    {
        element_map.push(XmlElement {
            name: "translational_speed".into(),
            data: ElementData::String(translational_speed.to_string()),
            ..Default::default()
        })
    }
    if let Some(rotational_speed) =
        diff_drive_config
            .get("rotational_speed")
            .and_then(|b| match b {
                serde_json::Value::Number(speed) => speed.as_f64(),
                _ => None,
            })
    {
        element_map.push(XmlElement {
            name: "rotational_speed".into(),
            data: ElementData::String(rotational_speed.to_string()),
            ..Default::default()
        })
    }
    // Add all elements to ElementData -> XmlElement
    XmlElement {
        data: ElementData::Nested(element_map),
        ..Default::default()
    }
}
