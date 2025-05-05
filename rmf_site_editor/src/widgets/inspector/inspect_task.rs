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
    site::{ChangePlugin, Group, LocationTags, ModelProperty, NameInSite, Robot, Task, Tasks},
    widgets::{prelude::*, Inspect, InspectionPlugin},
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Align, Button, Color32, ComboBox, Frame, Layout, Stroke, Ui};
use std::collections::HashMap;

/// Function that displays a widget to configure Task
type ShowTaskWidgetFn = fn(&usize, &mut Task, &Vec<String>, &mut Ui);

/// Function that checks whether a given Task is valid
type CheckTaskValidFn = fn(&Task, &Vec<String>) -> bool;

#[derive(Resource)]
pub struct TaskKinds(pub HashMap<String, (ShowTaskWidgetFn, CheckTaskValidFn)>);

impl FromWorld for TaskKinds {
    fn from_world(_world: &mut World) -> Self {
        TaskKinds(HashMap::new())
    }
}

#[derive(Default)]
pub struct InspectTaskPlugin {}

impl Plugin for InspectTaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, add_remove_robot_tasks)
            .init_resource::<PendingTask>()
            .init_resource::<TaskKinds>()
            .add_plugins((
                ChangePlugin::<Tasks>::default(),
                InspectionPlugin::<InspectTasks>::new(),
            ));
    }
}

#[derive(SystemParam)]
pub struct InspectTasks<'w, 's> {
    robots: Query<'w, 's, &'static mut Tasks, (With<Robot>, Without<Group>)>,
    locations: Query<'w, 's, (Entity, &'static NameInSite, &'static LocationTags)>,
    pending_task: ResMut<'w, PendingTask>,
    tasks: ResMut<'w, TaskKinds>,
    icons: Res<'w, Icons>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectTasks<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Ok(mut tasks) = params.robots.get_mut(selection) else {
            return;
        };

        let current_locations = get_location_names(&params.locations);

        ui.label("Tasks");
        ui.indent("inspect_tasks", |ui| {
            Frame::default()
                .inner_margin(4.0)
                .corner_radius(2.0)
                .stroke(Stroke::new(1.0, Color32::GRAY))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    if tasks.0.is_empty() {
                        ui.label("No tasks");
                    } else {
                        let mut deleted_ids = Vec::new();
                        for (id, task) in tasks.0.iter_mut().enumerate() {
                            if let Some((show_widget, _)) = params.tasks.0.get(&task.kind) {
                                edit_task_component(
                                    ui,
                                    &id,
                                    task,
                                    *show_widget,
                                    &current_locations,
                                    Some(&mut deleted_ids),
                                );
                            }
                        }
                        for id in deleted_ids {
                            tasks.0.remove(id);
                        }
                    }
                });

            ui.add_space(10.0);
            ui.label("Add Task");
            ui.horizontal(|ui| {
                ui.add_enabled_ui(
                    params
                        .tasks
                        .0
                        .get(&params.pending_task.0.kind)
                        .is_some_and(|(_, is_valid)| {
                            is_valid(&params.pending_task.0, &current_locations)
                        }),
                    |ui| {
                        if ui
                            .add(Button::image_and_text(params.icons.add.egui(), "New"))
                            .clicked()
                        {
                            tasks.0.push(params.pending_task.0.clone());
                        }
                    },
                );
                ComboBox::from_id_salt("select_task_kind")
                    .selected_text(params.pending_task.0.kind.clone())
                    .show_ui(ui, |ui| {
                        for (kind, _) in params.tasks.0.iter() {
                            ui.selectable_value(
                                &mut params.pending_task.0.kind,
                                kind.clone(),
                                kind.clone(),
                            );
                        }
                    });
            });
            if let Some((show_widget, _)) = params.tasks.0.get(&params.pending_task.0.kind) {
                edit_task_component(
                    ui,
                    &tasks.0.len(),
                    &mut params.pending_task.0,
                    *show_widget,
                    &current_locations,
                    None,
                );
            }
        });

        ui.add_space(10.0);
    }
}

fn get_location_names(locations: &Query<(Entity, &NameInSite, &LocationTags)>) -> Vec<String> {
    let mut location_names = Vec::new();
    for (_, location_name, _) in locations.iter() {
        location_names.push(location_name.0.clone());
    }
    location_names
}

fn edit_task_component(
    ui: &mut Ui,
    id: &usize,
    task: &mut Task,
    show_widget: ShowTaskWidgetFn,
    locations: &Vec<String>,
    deleted_ids: Option<&mut Vec<usize>>,
) {
    Frame::default()
        .inner_margin(4.0)
        .fill(Color32::DARK_GRAY)
        .corner_radius(2.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(task.kind.clone());
                show_widget(id, task, &locations, ui);
                if let Some(pending_delete) = deleted_ids {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("❌").on_hover_text("Delete task").clicked() {
                            pending_delete.push(*id);
                        }
                    });
                }
            });
        });
}

#[derive(Resource)]
pub struct PendingTask(Task);

impl FromWorld for PendingTask {
    fn from_world(_world: &mut World) -> Self {
        PendingTask(Task::default())
    }
}

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
