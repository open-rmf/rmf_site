/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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

use crate::widgets::inspector::InspectAngle;
use bevy::math::Quat;
use bevy_egui::egui::{ComboBox, DragValue, Grid, Ui};
use rmf_site_format::{Pose, Rotation};

pub struct InspectPose<'a> {
    pub pose: &'a Pose,
    pub for_rotation: &'a bool,
}

impl<'a> InspectPose<'a> {
    pub fn new(pose: &'a Pose) -> Self {
        Self { pose, for_rotation: &false }
    }

    pub fn for_rotation(mut self) -> Self {
        self.for_rotation = &true;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<Pose> {
        let mut new_pose = self.pose.clone();
        if !self.for_rotation {
            Grid::new("inspect_pose_translation").show(ui, |ui| {
                ui.label("x");
                ui.label("y");
                ui.label("z");
                ui.end_row();

                ui.add(DragValue::new(&mut new_pose.trans[0]).speed(0.01));
                ui.add(DragValue::new(&mut new_pose.trans[1]).speed(0.01));
                ui.add(DragValue::new(&mut new_pose.trans[2]).speed(0.01));
                ui.end_row();
            });
            ui.add_space(5.0);
        }

        ui.horizontal(|ui| {
            ui.label("Rotation");
            ComboBox::from_id_source("pose_rotation")
                .selected_text(new_pose.rot.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        new_pose.rot.as_yaw(),
                        new_pose.rot.as_euler_extrinsic_xyz(),
                        new_pose.rot.as_quat(),
                    ] {
                        ui.selectable_value(&mut new_pose.rot, *variant, variant.label());
                    }
                });
        });

        match &mut new_pose.rot {
            Rotation::Yaw(yaw) => {
                InspectAngle::new(yaw).show(ui);
            }
            Rotation::EulerExtrinsicXYZ([roll, pitch, yaw]) => {
                Grid::new("inspect_rotation_euler_xyz").show(ui, |ui| {
                    ui.label("roll");
                    ui.label("pitch");
                    ui.label("yaw");
                    ui.end_row();

                    InspectAngle::new(roll).show(ui);
                    InspectAngle::new(pitch).show(ui);
                    InspectAngle::new(yaw).show(ui);
                });
            }
            Rotation::Quat([x, y, z, w]) => {
                Grid::new("inspect_rotation_quat").show(ui, |ui| {
                    ui.label("x");
                    ui.label("y");
                    ui.label("z");
                    ui.label("w");
                    ui.end_row();

                    ui.add(DragValue::new(x).speed(0.01).clamp_range(-1.0..=1.0));
                    ui.add(DragValue::new(y).speed(0.01).clamp_range(-1.0..=1.0));
                    ui.add(DragValue::new(z).speed(0.01).clamp_range(-1.0..=1.0));
                    ui.add(DragValue::new(w).speed(0.01).clamp_range(-1.0..=1.0));
                    ui.end_row();
                });

                if ui.button("normalize").clicked() {
                    let normalized = Quat::from_array([*x, *y, *z, *w]).normalize();
                    [*x, *y, *z, *w] = normalized.to_array();
                }
            }
        }

        if new_pose != *self.pose {
            return Some(new_pose);
        }

        None
    }
}
