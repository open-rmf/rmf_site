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

use super::get_selected_description_entity;
use crate::{
    site::{Affiliation, Change, Collision, Group, ModelMarker, ModelProperty, Pose, Robot},
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, Ui};
use std::collections::HashMap;

#[derive(Resource)]
pub struct CollisionKinds(pub HashMap<String, fn(&mut Collision, &mut Ui)>);

impl FromWorld for CollisionKinds {
    fn from_world(_world: &mut World) -> Self {
        CollisionKinds(HashMap::new())
    }
}

#[derive(Default)]
pub struct InspectCollisionPlugin {}

impl Plugin for InspectCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionKinds>()
            .add_plugins(InspectionPlugin::<InspectCollision>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectCollision<'w, 's> {
    collision: ResMut<'w, CollisionKinds>,
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<Robot>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    poses: Query<'w, 's, &'static Pose>,
    gizmos: Gizmos<'s>,
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
        let collision = robot
            .properties
            .get(&Collision::label())
            .and_then(|c| serde_json::from_value::<Collision>(c.clone()).ok());
        let mut has_collision = collision.is_some();
        // TODO(@xiyuoh) a lot of duplicate code with inspect_mobility, consider consolidating
        // some of them to inspect_robot_properties

        ui.checkbox(&mut has_collision, "Collision");

        if !has_collision {
            let mut new_robot = robot.clone();
            new_robot.properties.remove(&Collision::label());
            params
                .change_robot_property
                .send(Change::new(ModelProperty(new_robot), description_entity));
            return;
        }

        let mut new_collision = match collision {
            Some(ref c) => c.clone(),
            None => Collision::default(),
        };

        let selected_collision_kind = if !new_collision.is_empty() {
            new_collision.kind.clone()
        } else {
            "Select Kind".to_string()
        };

        ui.indent("configure_collision", |ui| {
            ui.horizontal(|ui| {
                ui.label("Collision Kind");
                ComboBox::from_id_source("select_collision_kind")
                    .selected_text(selected_collision_kind)
                    .show_ui(ui, |ui| {
                        for (kind, _) in params.collision.0.iter() {
                            ui.selectable_value(
                                &mut new_collision.kind,
                                kind.clone(),
                                kind.clone(),
                            );
                        }
                    });
            });
            if !new_collision.is_default() {
                if let Some(show_widget) = params.collision.0.get(&new_collision.kind) {
                    show_widget(&mut new_collision, ui);
                }
            }
        });

        if collision.is_none()
            || collision.is_some_and(|m| m != new_collision && !new_collision.is_empty())
        {
            if let Ok(new_value) = serde_json::to_value(new_collision) {
                let mut new_robot = robot.clone();
                new_robot.properties.insert(Collision::label(), new_value);
                params
                    .change_robot_property
                    .send(Change::new(ModelProperty(new_robot), description_entity));
            }
        }
        ui.add_space(10.0);
    }
}
