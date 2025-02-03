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
        serialize_and_change_robot_property, show_robot_property, RobotPropertyWidgets,
    },
};
use crate::{
    site::{
        Affiliation, Change, CircleCollision, Collision, Group, ModelMarker, ModelProperty, Pose,
        Robot, RobotProperty,
    },
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, Grid, Ui};
use smallvec::SmallVec;

#[derive(SystemParam)]
pub struct InspectCollision<'w, 's> {
    commands: Commands<'w, 's>,
    robot_property_widgets: Res<'w, RobotPropertyWidgets>,
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<Robot>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    collision: Query<'w, 's, &'static Collision, (With<ModelMarker>, With<Group>)>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectCollision<'w, 's> {
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

        show_robot_property::<Collision>(
            ui,
            params.commands,
            params.collision,
            params.change_robot_property,
            robot,
            &params.robot_property_widgets,
            description_entity,
        );

        // Show children widgets
        if let Some((inspector_id, _)) = params.robot_property_widgets.0.get(&Collision::label()) {
            let children_widgets: Result<SmallVec<[_; 16]>, _> = params
                .children
                .get(*inspector_id)
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

#[derive(SystemParam)]
pub struct InspectCircleCollision<'w, 's> {
    commands: Commands<'w, 's>,
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<Robot>),
    >,
    model_descriptions: Query<
        'w,
        's,
        (&'static ModelProperty<Robot>, &'static CircleCollision),
        (With<ModelMarker>, With<Group>),
    >,
    poses: Query<'w, 's, &'static Pose>,
    gizmos: Gizmos<'s>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectCircleCollision<'w, 's> {
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
        let Ok((ModelProperty(robot), circle_collision)) =
            params.model_descriptions.get_mut(description_entity)
        else {
            return;
        };

        // Add default circle collision config if they have not been added
        if robot
            .properties
            .get(&Collision::label())
            .and_then(|c| c.as_object())
            .and_then(|obj| obj.get("config"))
            .is_some_and(|config| config.as_object().is_none_or(|m| m.is_empty()))
        {
            serialize_and_change_robot_property::<Collision, CircleCollision>(
                params.commands,
                params.change_robot_property,
                CircleCollision::default(),
                robot,
                description_entity,
            );
            return;
        }

        let mut new_circle_collision = circle_collision.clone();

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
                        if let Ok(pose) = params.poses.get(selection) {
                            params.gizmos.circle(
                                Vec3::new(pose.trans[0], pose.trans[1], pose.trans[2] + 0.01),
                                Vec3::Z,
                                new_circle_collision.radius,
                                Color::RED,
                            );
                        }
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

        if new_circle_collision != *circle_collision {
            // Update CircleCollision component
            params
                .commands
                .entity(description_entity)
                .insert(new_circle_collision.clone());
            // Update Collision and Robot
            serialize_and_change_robot_property::<Collision, CircleCollision>(
                params.commands,
                params.change_robot_property,
                new_circle_collision,
                robot,
                description_entity,
            );
        }
    }
}
