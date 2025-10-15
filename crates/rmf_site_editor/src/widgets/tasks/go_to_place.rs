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
    site::{
        update_task_kind_component, Affiliation, LocationTags, NameInSite, SiteID, Task, TaskKind,
        TaskKinds,
    },
    widgets::prelude::*,
};
use bevy::{
    ecs::{
        hierarchy::ChildOf,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use bevy_egui::egui::{ComboBox, SelectableLabel};
use rmf_site_egui::*;
use rmf_site_format::{GoToPlace, Robot};
use rmf_site_picking::Hover;

#[derive(Default)]
pub struct GoToPlacePlugin {}

impl Plugin for GoToPlacePlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().resource_mut::<TaskKinds>().0.insert(
            GoToPlace::<Entity>::label(),
            (
                |mut e_cmd| {
                    e_cmd.insert(GoToPlace::<Entity>::default());
                },
                |mut e_cmd| {
                    e_cmd.remove::<GoToPlace<Entity>>();
                },
                |e, world| {
                    let Some(loc_entity) = world
                        .entity(e)
                        .get::<GoToPlace<Entity>>()
                        .and_then(|go_to_place| go_to_place.location.0)
                    else {
                        return false;
                    };
                    let mut state: SystemState<Query<(), With<LocationTags>>> =
                        SystemState::new(world);
                    let locations = state.get(world);

                    locations.get(loc_entity).is_ok()
                },
            ),
        );
        let widget = Widget::<Tile>::new::<ViewGoToPlace>(&mut app.world_mut());
        let task_widget = app.world().resource::<TaskWidget>().get();
        app.world_mut().spawn(widget).insert(ChildOf(task_widget));
        app.add_systems(PostUpdate, update_task_kind_component::<GoToPlace<Entity>>)
            .add_observer(on_load_go_to_place);
    }
}

#[derive(SystemParam)]
pub struct ViewGoToPlace<'w, 's> {
    locations:
        Query<'w, 's, (Entity, &'static NameInSite, Option<&'static SiteID>), With<LocationTags>>,
    edit_task: Res<'w, EditTask>,
    tasks: Query<'w, 's, (&'static mut GoToPlace<Entity>, &'static mut Task<Entity>)>,
    hover: EventWriter<'w, Hover>,
    robot_debug_goal: Query<'w, 's, (Entity, Option<&'static mut DebugGoal>), With<Robot>>,
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

        let selected_location_name = if let Some((_, loc_name, _)) = go_to_place
            .location
            .0
            .and_then(|e| params.locations.get(e).ok())
        {
            loc_name.0.clone()
        } else {
            "Select Location".to_string()
        };

        let mut new_go_to_place = go_to_place.clone();
        ui.horizontal(|ui| {
            ui.label("Location:");
            ComboBox::from_id_salt("select_go_to_location")
                .selected_text(selected_location_name)
                .show_ui(ui, |ui| {
                    // Sort locations alphabetically
                    let mut sorted_locations = params.locations.iter().fold(
                        Vec::<(Entity, String)>::new(),
                        |mut l, (e, name, _)| {
                            l.push((e, name.0.clone()));
                            l
                        },
                    );
                    sorted_locations.sort_by(|a, b| a.1.cmp(&b.1));
                    for (loc_entity, loc_name) in sorted_locations.iter() {
                        let resp = ui.add(SelectableLabel::new(
                            new_go_to_place.location == Affiliation(Some(*loc_entity)),
                            loc_name.clone(),
                        ));
                        if resp.clicked() {
                            new_go_to_place.location = Affiliation(Some(*loc_entity));
                        } else if resp.hovered() {
                            params.hover.write(Hover(Some(*loc_entity)));
                        }
                    }
                });
        });

        if *go_to_place != new_go_to_place {
            *go_to_place = new_go_to_place.clone();

            let location_entity = new_go_to_place.location.0;
            let location_name = location_entity
                .and_then(|e| params.locations.get(e).ok())
                .map(|(_, name, _)| name.0.clone());

            // Convert Location entity to SiteID before serializing
            if let Some(description) = location_entity
                .and_then(|e| params.locations.get(e).ok())
                .and_then(|(_, _, site_id)| site_id)
                .and_then(|id| {
                    serde_json::to_value(GoToPlace::<u32> {
                        location: Affiliation(Some(id.0)),
                    })
                    .ok()
                })
            {
                *task.request_mut().description_mut() = description;
            }
            *task.request_mut().description_display_mut() = location_name.clone();

            // Update DebugGoal
            // TODO(@xiyuoh) debug why negotiation loops if goal changes for existing task
            if let Some((robot_entity, debug_goal)) = task
                .robot()
                .0
                .and_then(|e_robot| params.robot_debug_goal.get_mut(e_robot).ok())
            {
                let goal_location = location_name.unwrap_or("No Location".to_string());
                if let Some(mut goal) = debug_goal {
                    goal.location = goal_location;
                    goal.entity = location_entity;
                } else {
                    params.commands.entity(robot_entity).insert(DebugGoal {
                        location: goal_location,
                        entity: location_entity,
                    });
                }
            }
        }
    }
}

/// When loading a GoToPlace task from file, locations are stored as SiteID.
/// Since task description is serialized as JSON, we won't be able to do the
/// usual Entity <-> SiteID conversion. This observer checks that the GoToPlace
/// task/location entity loaded is valid. If not, use location name stored in
/// description display to select location entity.
fn on_load_go_to_place(
    trigger: Trigger<OnInsert, Task<Entity>>,
    mut commands: Commands,
    tasks: Query<(Entity, &Task<Entity>, Option<&GoToPlace<Entity>>)>,
    locations: Query<(Entity, &NameInSite), With<LocationTags>>,
) {
    let Ok((task_entity, task, go_to_place)) = tasks.get(trigger.target()) else {
        return;
    };
    if task.request().category() != GoToPlace::<Entity>::label() {
        return;
    }
    // Ignore if this is a valid location entity
    if go_to_place.is_some_and(|gtp| gtp.location.0.is_some_and(|e| locations.get(e).is_ok())) {
        return;
    }

    // Rely on description display for location name matching
    let Some(location_name) = task.request().description_display() else {
        return;
    };
    for (entity, name) in locations.iter() {
        if location_name == *name.0 {
            commands.entity(task_entity).insert(GoToPlace::<Entity> {
                location: Affiliation(Some(entity)),
            });
            return;
        }
    }
}
