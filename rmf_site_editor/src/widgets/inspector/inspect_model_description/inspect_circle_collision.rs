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

use super::inspect_robot_properties::RobotPropertyData;
use crate::{
    interaction::Selection,
    site::{
        Affiliation, CircleCollision, Collision, Group, ModelMarker, ModelProperty, Pose, Robot,
        RobotProperty, RobotPropertyKind,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{DragValue, Grid, Ui};

#[derive(Default)]
pub struct InspectCircleCollisionPlugin {}

impl Plugin for InspectCircleCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.world
            .resource_mut::<RobotPropertyData>()
            .0
            .get_mut(&Collision::label())
            .map(|c_map| {
                c_map.insert(CircleCollision::label(), |config, ui| {
                    InspectCircleCollision::new(config).show(ui);
                })
            });
        app.add_systems(PostUpdate, update_view_circle_collision);
    }
}

pub struct InspectCircleCollision<'a> {
    config: &'a mut serde_json::Value,
}

impl<'a> InspectCircleCollision<'a> {
    pub fn new(config: &'a mut serde_json::Value) -> Self {
        Self { config }
    }

    pub fn show(self, ui: &mut Ui) {
        let mut new_circle_collision =
            match serde_json::from_value::<CircleCollision>(self.config.clone()) {
                Ok(circle_collision) => circle_collision,
                Err(_) => CircleCollision::default(),
            };

        ui.indent("inspect_circle_collision_properties", |ui| {
            Grid::new("inspect_circle_collision")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("Collision Radius");
                    ui.add(
                        DragValue::new(&mut new_circle_collision.radius)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(0.01),
                    );
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
                    ui.checkbox(&mut new_circle_collision.view, "View".to_string());
                    ui.end_row();
                });
        });

        if let Some(new_map) = serde_json::to_value(new_circle_collision)
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

fn update_view_circle_collision(
    model_instances: Query<&Affiliation<Entity>, (With<ModelMarker>, Without<Group>, With<Robot>)>,
    model_descriptions: Query<&ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    selection: Res<Selection>,
    poses: Query<&Pose>,
    mut gizmos: Gizmos,
) {
    let Some(selected_entity) = selection.0 else {
        return;
    };
    let mut description_entity: Option<Entity> = None;
    if model_descriptions.get(selected_entity).ok().is_some() {
        description_entity = Some(selected_entity);
    } else {
        if let Some(affiliation) = model_instances.get(selected_entity).ok().and_then(|a| a.0) {
            if model_descriptions.get(affiliation).is_ok() {
                description_entity = Some(affiliation);
            }
        }
    }
    let Some(ModelProperty(robot)) =
        description_entity.and_then(|e| model_descriptions.get(e).ok())
    else {
        return;
    };

    let Some(circle_collision) = robot
        .properties
        .get(&Collision::label())
        .and_then(|c| serde_json::from_value::<Collision>(c.clone()).ok())
        .filter(|c| c.kind() == CircleCollision::label())
        .and_then(|c| serde_json::from_value::<CircleCollision>(c.config().clone()).ok())
    else {
        return;
    };
    if !circle_collision.view {
        return;
    }
    if let Ok(pose) = poses.get(selected_entity) {
        gizmos.circle(
            Vec3::new(
                pose.trans[0] + circle_collision.offset[0],
                pose.trans[1] + circle_collision.offset[1],
                pose.trans[2] + 0.01,
            ),
            Vec3::Z,
            circle_collision.radius,
            Color::RED,
        );
    }
}
