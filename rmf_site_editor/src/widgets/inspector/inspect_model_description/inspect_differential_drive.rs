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

use super::get_selected_description_entity;
use crate::{
    site::{Affiliation, Change, DifferentialDrive, Group, ModelMarker, ModelProperty},
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Align, Direction, DragValue, Grid, InnerResponse, Layout};
use serde::de::value;

#[derive(SystemParam)]
pub struct InspectModelDifferentialDrive<'w, 's> {
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<DifferentialDrive>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<DifferentialDrive>, (With<ModelMarker>, With<Group>)>,
    change_differential_drive: EventWriter<'w, Change<ModelProperty<DifferentialDrive>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelDifferentialDrive<'w, 's> {
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
        let Ok(ModelProperty(differential_drive)) =
            params.model_descriptions.get(description_entity)
        else {
            return;
        };

        let mut new_differential_drive = differential_drive.clone();

        ui.label("Differential Drive");

        ui.set_min_width(ui.available_width());

        Grid::new("inspect_differential_drive")
            .num_columns(2)
            .show(ui, |ui| {
                // Max Velocity
                ui.label("max velocity");
                ui.end_row();

                value_name(ui, |ui| {
                    ui.label("translation");
                });
                ui.add(
                    DragValue::new(&mut new_differential_drive.translational_speed)
                        .clamp_range(0_f32..=std::f32::INFINITY)
                        .speed(0.01),
                );
                ui.label("m/s");
                ui.end_row();
                value_name(ui, |ui| {
                    ui.label("angular");
                });
                ui.add(
                    DragValue::new(&mut new_differential_drive.rotational_speed)
                        .clamp_range(0_f32..=std::f32::INFINITY)
                        .speed(0.01),
                );
                ui.label("rad/s");
                ui.end_row();

                // Center Offset
                ui.label("center offset");
                ui.end_row();
                value_name(ui, |ui| {
                    ui.label("x");
                });
                ui.add(
                    DragValue::new(&mut new_differential_drive.rotation_center_offset[0])
                        .clamp_range(0_f32..=std::f32::INFINITY)
                        .speed(0.01),
                );
                ui.label("m");
                ui.end_row();
                value_name(ui, |ui| {
                    ui.label("y");
                });
                ui.add(
                    DragValue::new(&mut new_differential_drive.rotation_center_offset[1])
                        .clamp_range(0_f32..=std::f32::INFINITY)
                        .speed(0.01),
                );
                ui.label("m");
                ui.end_row();

                // Bidirectional
                ui.label("bidirectional");
                ui.end_row();
                value_name(ui, |ui| ui.label("enabled"));
                ui.checkbox(&mut new_differential_drive.bidirectional, "");
            });

        if new_differential_drive != *differential_drive {
            params.change_differential_drive.send(Change::new(
                ModelProperty(new_differential_drive),
                description_entity,
            ));
        }
    }
}

/// Displays a right indented value name
fn value_name<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.with_layout(
        Layout::from_main_dir_and_cross_align(Direction::RightToLeft, Align::Max),
        add_contents,
    )
}
