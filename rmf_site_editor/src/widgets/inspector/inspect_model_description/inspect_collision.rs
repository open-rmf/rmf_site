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
    inspect_robot_properties::{show_robot_property, RobotPropertyWidgets},
};
use crate::{
    site::{
        Affiliation, Change, Collision, Group, ModelMarker, ModelProperty, Robot, RobotProperty,
    },
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::Ui;

#[derive(SystemParam)]
pub struct InspectCollision<'w, 's> {
    robot_property_data: ResMut<'w, RobotPropertyWidgets>,
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<Robot>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectCollision<'w, 's> {
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
        let Ok(ModelProperty(robot)) = params.model_descriptions.get(description_entity) else {
            return;
        };
        let mut new_robot = robot.clone();
        let collision_label = Collision::label();
        let collision = robot
            .properties
            .get(&collision_label)
            .and_then(|c| serde_json::from_value::<Collision>(c.clone()).ok());

        match show_robot_property::<Collision>(ui, collision, params.robot_property_data) {
            Ok(res) => {
                if let Some(new_value) = res.map(|c| serde_json::to_value(c).ok()).flatten() {
                    new_robot.properties.insert(collision_label, new_value);
                } else {
                    new_robot.properties.remove(&collision_label);
                }
                params
                    .change_robot_property
                    .send(Change::new(ModelProperty(new_robot), description_entity));
            }
            Err(_) => {}
        }
    }
}
