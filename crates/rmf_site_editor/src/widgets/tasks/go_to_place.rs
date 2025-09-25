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
use super::{EditTask, TaskWidget};
use crate::{
    mapf_rse::DebugGoal,
    site::{update_task_kind_component, LocationTags, NameInSite, Task, TaskKind, TaskKinds},
    widgets::prelude::*,
};
use bevy::{
    ecs::{
        hierarchy::ChildOf,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use bevy_egui::egui::ComboBox;
use rmf_site_egui::*;
use rmf_site_format::{GoToPlace, Robot};

#[derive(Default)]
pub struct GoToPlacePlugin {}

impl Plugin for GoToPlacePlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().resource_mut::<TaskKinds>().0.insert(
            GoToPlace::label(),
            (
                |mut e_cmd| {
                    e_cmd.insert(GoToPlace::default());
                },
                |mut e_cmd| {
                    e_cmd.remove::<GoToPlace>();
                },
            ),
        );
        let widget = Widget::<Tile>::new::<ViewGoToPlace>(&mut app.world_mut());
        let task_widget = app.world().resource::<TaskWidget>().get();
        app.world_mut().spawn(widget).insert(ChildOf(task_widget));
        app.add_systems(PostUpdate, update_task_kind_component::<GoToPlace>);
    }
}

#[derive(SystemParam)]
pub struct ViewGoToPlace<'w, 's> {
    locations: Query<'w, 's, (Entity, &'static NameInSite), With<LocationTags>>,
    edit_task: Res<'w, EditTask>,
    tasks: Query<'w, 's, (&'static mut GoToPlace, &'static mut Task)>,
    robot_debug_goal:
        Query<'w, 's, (Entity, &'static NameInSite, Option<&'static mut DebugGoal>), With<Robot>>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewGoToPlace<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);

        let Some((mut go_to_place, mut task)) =
            params.edit_task.and_then(|e| params.tasks.get_mut(e).ok())
        else {
            return;
        };
        if params.locations.is_empty() {
            ui.label("No locations available in site");
            return;
        }

        let selected_location_name = if go_to_place.location.is_empty()
            || !params
                .locations
                .iter()
                .any(|l| l.1 .0 == go_to_place.location)
        {
            "Select Location".to_string()
        } else {
            go_to_place.location.clone()
        };

        let mut new_go_to_place = go_to_place.clone();
        ui.horizontal(|ui| {
            ui.label("Location:");
            ComboBox::from_id_salt("select_go_to_location")
                .selected_text(selected_location_name)
                .show_ui(ui, |ui| {
                    for (_, location_name) in params.locations.iter() {
                        ui.selectable_value(
                            &mut new_go_to_place.location,
                            location_name.0.clone(),
                            location_name.0.clone(),
                        );
                    }
                });
        });

        if *go_to_place != new_go_to_place {
            *go_to_place = new_go_to_place.clone();
            let task_robot_name = task.robot();
            // TODO(Nielsen): Save location as entity in task and robot as entity in task to avoid iterating
            for (robot_entity, robot_name, debug_goal) in params.robot_debug_goal.iter_mut() {
                if robot_name.0 == task_robot_name {
                    let location = go_to_place.location.clone();
                    let mut entity = None;

                    if let Some((location_entity, _)) = params
                        .locations
                        .iter()
                        .find(|(_, location_name)| location_name.0 == go_to_place.location)
                    {
                        entity = Some(location_entity);
                    } else {
                        error!("Unable to find location entity from name");
                    }

                    if let Some(mut goal) = debug_goal {
                        goal.location = go_to_place.location.clone();
                        goal.entity = entity;
                    } else {
                        params
                            .commands
                            .entity(robot_entity)
                            .insert(DebugGoal { location, entity });
                    }
                    break;
                }
            }

            if let Ok(description) = serde_json::to_value(new_go_to_place.clone()) {
                *task.request_mut().description_mut() = description;
                *task.request_mut().description_display_mut() =
                    Some(format!("{}", new_go_to_place.clone()));
            }
        }
    }
}
