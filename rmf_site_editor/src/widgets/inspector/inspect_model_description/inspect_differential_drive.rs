/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use super::inspect_robot_properties::RobotPropertyData;
use crate::site::{DifferentialDrive, Mobility, RobotProperty, RobotPropertyKind};
use bevy::prelude::*;
use bevy_egui::egui::{DragValue, Grid, Ui};

#[derive(Default)]
pub struct InspectDifferentialDrivePlugin {}

impl Plugin for InspectDifferentialDrivePlugin {
    fn build(&self, app: &mut App) {
        app.world
            .resource_mut::<RobotPropertyData>()
            .0
            .get_mut(&Mobility::label())
            .map(|m_map| {
                m_map.insert(DifferentialDrive::label(), |config, ui| {
                    InspectDifferentialDrive::new(config).show(ui);
                })
            });
    }
}

pub struct InspectDifferentialDrive<'a> {
    config: &'a mut serde_json::Value,
}

impl<'a> InspectDifferentialDrive<'a> {
    pub fn new(config: &'a mut serde_json::Value) -> Self {
        Self { config }
    }

    pub fn show(self, ui: &mut Ui) {
        let mut new_differential_drive =
            match serde_json::from_value::<DifferentialDrive>(self.config.clone()) {
                Ok(diff_drive) => diff_drive,
                Err(_) => DifferentialDrive::default(),
            };

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

        if let Some(new_map) = serde_json::to_value(new_differential_drive)
            .ok()
            .map(|v| v.as_object().cloned())
            .flatten()
        {
            for (k, v) in new_map {
                self.config.as_object_mut().unwrap().insert(k, v);
            }
        }
    }
}
