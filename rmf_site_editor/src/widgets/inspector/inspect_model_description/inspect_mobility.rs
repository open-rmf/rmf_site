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
        serialize_and_change_robot_property, show_robot_property_widget, RecallPropertyKind,
        RobotProperty, RobotPropertyKind, RobotPropertyWidgetRegistry,
    },
    ModelPropertyQuery,
};
use crate::{
    site::{Change, Group, ModelMarker, ModelProperty, Robot},
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, Grid, Ui};
use rmf_site_format::Recall;
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

    fn kind(&self) -> Option<String> {
        Some(self.kind.clone())
    }

    fn label() -> String {
        "Mobility".to_string()
    }
}

#[derive(Clone, Debug, Default, Component, PartialEq)]
pub struct RecallMobility {
    pub kind: Option<String>,
    pub config: Option<serde_json::Value>,
}

impl Recall for RecallMobility {
    type Source = Mobility;

    fn remember(&mut self, source: &Mobility) {
        self.kind = Some(source.kind.clone());
        self.config = Some(source.config.clone());
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
    recall_mobility: Query<'w, 's, &'static RecallMobility, (With<ModelMarker>, With<Group>)>,
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

        let recall_mobility: Option<Mobility> = match params.recall_mobility.get(description_entity)
        {
            Ok(recall) => Some(Mobility {
                kind: recall.kind.clone().unwrap_or_default(),
                config: recall.config.clone().unwrap_or_default(),
            }),
            Err(_) => None,
        };

        show_robot_property_widget::<Mobility>(
            ui,
            params.mobility,
            recall_mobility,
            params.change_robot_property,
            robot,
            &params.robot_property_widgets,
            description_entity,
        );

        // Show children widgets
        if let Some(widget_registration) = params.robot_property_widgets.get(&Mobility::label()) {
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

#[derive(Clone, Debug, Default, Component, PartialEq)]
pub struct RecallDifferentialDrive {
    pub bidirectional: Option<bool>,
    pub rotation_center_offset: Option<[f32; 2]>,
    pub translational_speed: Option<f32>,
    pub rotational_speed: Option<f32>,
}

impl RecallPropertyKind for RecallDifferentialDrive {
    type Kind = DifferentialDrive;

    fn assume(&self) -> DifferentialDrive {
        DifferentialDrive {
            bidirectional: self.bidirectional.clone().unwrap_or_default(),
            rotation_center_offset: self.rotation_center_offset.clone().unwrap_or_default(),
            translational_speed: self.translational_speed.clone().unwrap_or_default(),
            rotational_speed: self.rotational_speed.clone().unwrap_or_default(),
        }
    }
}

impl Recall for RecallDifferentialDrive {
    type Source = DifferentialDrive;

    fn remember(&mut self, source: &DifferentialDrive) {
        self.bidirectional = Some(source.bidirectional);
        self.rotation_center_offset = Some(source.rotation_center_offset);
        self.translational_speed = Some(source.translational_speed);
        self.rotational_speed = Some(source.rotational_speed);
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
