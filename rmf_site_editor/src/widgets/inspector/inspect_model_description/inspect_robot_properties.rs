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
    site::{
        update_model_instances, Affiliation, Change, ChangePlugin, Group, ModelMarker,
        ModelProperty, Robot, Tasks,
    },
    widgets::{prelude::*, Inspect},
    ModelPropertyData,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{RichText, Ui};

#[derive(Default)]
pub struct InspectRobotPropertiesPlugin {}

impl Plugin for InspectRobotPropertiesPlugin {
    fn build(&self, app: &mut App) {
        // Allows us to toggle Robot as a configurable property
        // from the model description inspector
        app.world.init_component::<ModelProperty<Robot>>();
        let component_id = app
            .world
            .components()
            .component_id::<ModelProperty<Robot>>()
            .unwrap();
        app.add_plugins(ChangePlugin::<ModelProperty<Robot>>::default())
            .add_systems(
                PreUpdate,
                (add_remove_robot_tasks, update_model_instances::<Robot>),
            )
            .init_resource::<ModelPropertyData>()
            .world
            .resource_mut::<ModelPropertyData>()
            .optional
            .insert(
                component_id,
                (
                    "Robot".to_string(),
                    |mut e_cmd| {
                        e_cmd.insert(ModelProperty::<Robot>::default());
                    },
                    |mut e_cmd| {
                        e_cmd.remove::<ModelProperty<Robot>>();
                    },
                ),
            );
        // Ui
        app.add_plugins(InspectionPlugin::<InspectRobotProperties>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectRobotProperties<'w, 's> {
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<Robot>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectRobotProperties<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
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
        // Ensure that this widget is displayed only when there is a valid Robot property
        let Ok(ModelProperty(_robot)) = params.model_descriptions.get(description_entity) else {
            return;
        };
        ui.separator();
        ui.label(RichText::new(format!("Robot Properties")).size(18.0));
        ui.add_space(10.0);
    }
}

// TODO(@xiyuoh) get rid of this and use checkbox to enable tasks instead?
/// When the Robot is added or removed, add or remove the Tasks component
fn add_remove_robot_tasks(
    mut commands: Commands,
    instances: Query<(Entity, Ref<Robot>), Without<Group>>,
    tasks: Query<&Tasks, (With<Robot>, Without<Group>)>,
    mut removals: RemovedComponents<ModelProperty<Robot>>,
) {
    // all instances with this description - add/remove Tasks component

    for removal in removals.read() {
        if instances.get(removal).is_ok() {
            commands.entity(removal).remove::<Tasks>();
        }
    }

    for (e, marker) in instances.iter() {
        if marker.is_added() {
            if let Ok(_) = tasks.get(e) {
                continue;
            }
            commands.entity(e).insert(Tasks::default());
        }
    }
}
