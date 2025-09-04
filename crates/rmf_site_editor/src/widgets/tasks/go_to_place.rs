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
use rmf_site_format::GoToPlace;
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
        app.add_systems(PostUpdate, update_task_kind_component::<GoToPlace<Entity>>);
    }
}

#[derive(SystemParam)]
pub struct ViewGoToPlace<'w, 's> {
    locations:
        Query<'w, 's, (Entity, &'static NameInSite, Option<&'static SiteID>), With<LocationTags>>,
    edit_task: Res<'w, EditTask>,
    tasks: Query<'w, 's, (&'static mut GoToPlace<Entity>, &'static mut Task<Entity>)>,
    hover: EventWriter<'w, Hover>,
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

            // Convert Location entity to SiteID before serializing
            if let Some(description) = new_go_to_place
                .location
                .0
                .and_then(|e| params.locations.get(e).ok())
                .and_then(|(_, _, site_id)| site_id)
                .and_then(|id| {
                    serde_json::to_value(GoToPlace::<u32> {
                        location: Affiliation(Some(**id)),
                    })
                    .ok()
                })
            {
                *task.request_mut().description_mut() = description;
                *task.request_mut().description_display_mut() = new_go_to_place
                    .location
                    .0
                    .and_then(|e| params.locations.get(e).ok())
                    .map(|(_, name, _)| name.0.clone());
            }
        }
    }
}
