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

use super::CollisionKinds;
use crate::site::{CircleCollision, Collision};
use bevy::prelude::*;
use bevy_egui::egui::{DragValue, Grid, Ui};

#[derive(Default)]
pub struct InspectCircleCollisionPlugin {}

impl Plugin for InspectCircleCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.world.resource_mut::<CollisionKinds>().0.insert(
            CircleCollision::label(),
            |collision, ui| {
                InspectCircleCollision::new(collision).show(ui);
            },
        );
    }
}

pub struct InspectCircleCollision<'a> {
    collision: &'a mut Collision,
}

impl<'a> InspectCircleCollision<'a> {
    pub fn new(collision: &'a mut Collision) -> Self {
        Self { collision }
    }

    pub fn show(self, ui: &mut Ui) {
        let mut new_circle_collision =
            match serde_json::from_value::<CircleCollision>(self.collision.config.clone()) {
                Ok(circle_collision) => circle_collision,
                Err(_) => CircleCollision::default(),
            };

        ui.indent("inspect_circle_collision_properties", |ui| {
            Grid::new("inspect_circle_collision")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("Collision Radius");
                    if ui
                        .add(
                            DragValue::new(&mut new_circle_collision.radius)
                                .clamp_range(0_f32..=std::f32::INFINITY)
                                .speed(0.01),
                        )
                        .is_pointer_button_down_on()
                    {
                        // TODO(@xiyuoh) bring in poses and gizmos
                        // if let Ok(pose) = params.poses.get(selection) {
                        //     params.gizmos.circle(
                        //         Vec3::new(pose.trans[0], pose.trans[1], pose.trans[2] + 0.01),
                        //         Vec3::Z,
                        //         new_circle_collision.radius,
                        //         Color::RED,
                        //     );
                        // }
                    };
                    ui.label("m");
                    ui.end_row();

                    ui.label("Offset");
                    ui.label("x");
                    ui.label("y");
                    ui.end_row();

                    ui.label("");
                    ui.add(
                        DragValue::new(&mut new_circle_collision.offset[0])
                            .clamp_range(std::f32::NEG_INFINITY..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.add(
                        DragValue::new(&mut new_circle_collision.offset[1])
                            .clamp_range(std::f32::NEG_INFINITY..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.end_row();
                });
        });

        if let Ok(new_value) = serde_json::to_value(new_circle_collision) {
            self.collision.config = new_value;
        }
    }
}
