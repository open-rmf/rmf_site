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

use super::{
    get_selected_description_entity,
    inspect_robot_properties::{
        serialize_and_change_robot_property, show_robot_property_widget, RobotProperty,
        RobotPropertyKind, RobotPropertyWidgetRegistry,
    },
    ModelPropertyQuery,
};
use crate::{
    site::{Change, Group, ModelMarker, ModelProperty, Robot},
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, Grid, Ui};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use smallvec::SmallVec;

#[derive(Serialize, Deserialize, Debug, Clone, Component, PartialEq)]
pub struct Mobility {
    pub kind: String,
    pub config: serde_json::Value,
}

impl Default for Mobility {
    fn default() -> Self {
        Self {
            kind: String::new(),
            config: serde_json::Value::Object(Map::new()),
        }
    }
}

impl RobotProperty for Mobility {
    fn new(kind: String, config: serde_json::Value) -> Self {
        Self { kind, config }
    }

    fn is_default(&self) -> bool {
        if *self == Self::default() {
            return true;
        }
        false
    }

    fn is_empty(&self) -> bool {
        if self.kind.is_empty() {
            return true;
        }
        return self.config.as_object().is_none_or(|m| m.is_empty());
    }

    fn kind(&self) -> String {
        self.kind.clone()
    }

    fn kind_mut(&mut self) -> &mut String {
        &mut self.kind
    }

    fn config(&self) -> serde_json::Value {
        self.config.clone()
    }

    fn config_mut(&mut self) -> &mut serde_json::Value {
        &mut self.config
    }

    fn label() -> String {
        "Mobility".to_string()
    }
}

#[derive(SystemParam)]
pub struct InspectMobility<'w, 's> {
    robot_property_widgets: Res<'w, RobotPropertyWidgetRegistry>,
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    mobility: Query<'w, 's, &'static Mobility, (With<ModelMarker>, With<Group>)>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectMobility<'w, 's> {
    fn show(
        Inspect {
            selection,
            inspection: _,
            panel,
        }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let params = state.get_mut(world);
        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };
        let Ok(ModelProperty(robot)) = params.model_descriptions.get(description_entity) else {
            return;
        };

        show_robot_property_widget::<Mobility>(
            ui,
            params.mobility,
            params.change_robot_property,
            robot,
            &params.robot_property_widgets,
            description_entity,
        );

        // Show children widgets
        if let Some(widget_registration) = params.robot_property_widgets.0.get(&Mobility::label()) {
            let children_widgets: Result<SmallVec<[_; 16]>, _> = params
                .children
                .get(widget_registration.property_widget)
                .map(|c| c.iter().copied().collect());
            let Ok(children_widgets) = children_widgets else {
                return;
            };

            for child in children_widgets {
                let inspect = Inspect {
                    selection,
                    inspection: child,
                    panel,
                };
                ui.add_space(10.0);
                let _ = world.try_show_in(child, inspect, ui);
            }
        }
    }
}

// Supported kinds of Mobility
#[derive(Serialize, Deserialize, Debug, Clone, Component, PartialEq, Reflect)]
pub struct DifferentialDrive {
    pub bidirectional: bool,
    pub rotation_center_offset: [f32; 2],
    pub translational_speed: f32,
    pub rotational_speed: f32,
}

impl Default for DifferentialDrive {
    fn default() -> Self {
        Self {
            bidirectional: false,
            rotation_center_offset: [0.0, 0.0],
            translational_speed: 0.5,
            rotational_speed: 1.0,
        }
    }
}

impl RobotPropertyKind for DifferentialDrive {
    fn label() -> String {
        "Differential Drive".to_string()
    }
}

#[derive(SystemParam)]
pub struct InspectDifferentialDrive<'w, 's> {
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
    model_descriptions: Query<
        'w,
        's,
        (&'static ModelProperty<Robot>, &'static DifferentialDrive),
        (With<ModelMarker>, With<Group>),
    >,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectDifferentialDrive<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };
        let Ok((ModelProperty(robot), differential_drive)) =
            params.model_descriptions.get_mut(description_entity)
        else {
            return;
        };

        // Add default diff drive config if they have not been added
        if robot
            .properties
            .get(&Mobility::label())
            .and_then(|c| c.as_object())
            .and_then(|obj| obj.get("config"))
            .is_some_and(|config| config.as_object().is_none_or(|m| m.is_empty()))
        {
            serialize_and_change_robot_property::<Mobility, DifferentialDrive>(
                params.change_robot_property,
                DifferentialDrive::default(),
                robot,
                description_entity,
            );
            return;
        }

        let mut new_differential_drive = differential_drive.clone();

        ui.indent("inspect_differential_drive_properties", |ui| {
            Grid::new("inspect_differential_drive")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("Center Offset");
                    ui.label("x");
                    ui.label("y");
                    ui.end_row();

                    ui.label("");
                    ui.add(
                        DragValue::new(&mut new_differential_drive.rotation_center_offset[0])
                            .clamp_range(std::f32::NEG_INFINITY..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.add(
                        DragValue::new(&mut new_differential_drive.rotation_center_offset[1])
                            .clamp_range(std::f32::NEG_INFINITY..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.end_row();

                    ui.label("Bidirectional");
                    ui.checkbox(&mut new_differential_drive.bidirectional, "");
                    ui.end_row();

                    ui.label("Max Velocity");
                    ui.add(
                        DragValue::new(&mut new_differential_drive.translational_speed)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.label("m/s");
                    ui.end_row();

                    ui.label("Max Angular");
                    ui.add(
                        DragValue::new(&mut new_differential_drive.rotational_speed)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.label("rad/s");
                    ui.end_row();
                });
        });

        if new_differential_drive != *differential_drive {
            serialize_and_change_robot_property::<Mobility, DifferentialDrive>(
                params.change_robot_property,
                new_differential_drive,
                robot,
                description_entity,
            );
        }
    }
}
