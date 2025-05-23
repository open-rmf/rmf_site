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

use crate::{
    interaction::{Select, Selection},
    site::*,
    Tile, WidgetSystem,
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, Color32, Frame, Stroke, Ui};

#[derive(SystemParam)]
pub struct ViewTasks<'w, 's> {
    robots: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            Option<&'static SiteID>,
            &'static mut Tasks,
        ),
        (With<Robot>, Without<Group>),
    >,
    site_entities: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            &'static Category,
            Option<&'static SiteID>,
        ),
        Without<ModelMarker>,
    >,
    selection: Res<'w, Selection>,
    select: EventWriter<'w, Select>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewTasks<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        CollapsingHeader::new("Tasks")
            .default_open(false)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewTasks<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        Frame::default()
            .inner_margin(4.0)
            .corner_radius(2.0)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                let mut total_task_count: u32 = 0;
                for (robot_entity, robot_name, robot_site_id, robot_tasks) in self.robots.iter() {
                    for task in robot_tasks.0.iter() {
                        show_task(
                            ui,
                            task,
                            &robot_name.0,
                            &robot_entity,
                            robot_site_id,
                            &self.site_entities,
                            &self.selection,
                            &mut self.select,
                            &mut total_task_count,
                        );
                    }
                }
                if total_task_count == 0 {
                    ui.label("No tasks");
                }
            });
    }
}

fn show_task(
    ui: &mut Ui,
    task: &Task,
    robot_name: &str,
    robot_entity: &Entity,
    robot_site_id: Option<&SiteID>,
    site_entities: &Query<(Entity, &NameInSite, &Category, Option<&SiteID>), Without<ModelMarker>>,
    selected: &Selection,
    select: &mut EventWriter<Select>,
    task_count: &mut u32,
) {
    Frame::default()
        .inner_margin(4.0)
        .fill(Color32::DARK_GRAY)
        .corner_radius(2.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Robot
            ui.horizontal(|ui| {
                ui.label("Robot");
                if ui
                    .selectable_label(
                        selected.0.is_some_and(|s| s == *robot_entity),
                        format!(
                            "Model #{} [{}]",
                            robot_site_id
                                .map(|id| id.to_string())
                                .unwrap_or("unsaved".to_string()),
                            robot_name
                        )
                        .to_string(),
                    )
                    .clicked()
                {
                    select.write(Select::new(Some(*robot_entity)));
                }
            });

            // Additional context for default tasks
            if task.kind == GoToPlace::label() {
                if let Ok(go_to_place) = serde_json::from_value::<GoToPlace>(task.config.clone()) {
                    ui.horizontal(|ui| {
                        ui.label("Go To Place: ");
                        for (entity, name, _, site_id) in site_entities.iter() {
                            if *name.0 == go_to_place.location {
                                if ui
                                    .selectable_label(
                                        selected.0.is_some_and(|s| s == entity),
                                        format!(
                                            "Location #{} [{}]",
                                            site_id
                                                .map(|id| id.to_string())
                                                .unwrap_or("unsaved".to_string()),
                                            name.0
                                        ),
                                    )
                                    .clicked()
                                {
                                    select.write(Select::new(Some(entity)));
                                }
                                break;
                            }
                        }
                    });
                }
            } else if task.kind == WaitFor::label() {
                if let Ok(wait_for) = serde_json::from_value::<WaitFor>(task.config.clone()) {
                    ui.label(format!("Wait For: {} seconds", wait_for.duration));
                }
            }
        });
    *task_count += 1;
}
