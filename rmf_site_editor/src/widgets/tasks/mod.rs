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
use crate::{
    site::{
        Category, CurrentScenario, Delete, DispatchTaskRequest, Group, NameInSite, Pending, Robot,
        RobotTaskRequest, Scenario, ScenarioMarker, Task,
    },
    widgets::prelude::*,
    CurrentWorkspace, Tile, WidgetSystem,
};
use bevy::{
    ecs::system::{EntityCommands, SystemParam, SystemState},
    prelude::*,
};
use bevy_egui::egui::{
    Align, CollapsingHeader, Color32, ComboBox, DragValue, Frame, Grid, Layout, Stroke, TextEdit,
    Ui,
};
use serde_json::Value;
use smallvec::SmallVec;
use std::collections::HashMap;

pub mod go_to_place;
pub use go_to_place::*;

pub mod wait_for;
pub use wait_for::*;

pub type InsertTaskKindFn = fn(EntityCommands);
pub type RemoveTaskKindFn = fn(EntityCommands);

#[derive(Resource)]
pub struct TaskKinds(pub HashMap<String, (InsertTaskKindFn, RemoveTaskKindFn)>);

impl FromWorld for TaskKinds {
    fn from_world(_world: &mut World) -> Self {
        TaskKinds(HashMap::new())
    }
}

#[derive(Default)]
pub struct StandardTasksPlugin {}

impl Plugin for StandardTasksPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MainTasksPlugin::default());
        app.add_plugins((GoToPlacePlugin::default(), WaitForPlugin::default()));
    }
}

/// This is the main Tasks widget that enables addition, removal and modification of tasks.
#[derive(Default)]
pub struct MainTasksPlugin {}

impl Plugin for MainTasksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TaskWidget>()
            .init_resource::<TaskKinds>()
            .init_resource::<EditTask>();
    }
}

/// Contains a reference to the tasks widget.
#[derive(Resource)]
pub struct TaskWidget {
    id: Entity,
}

impl TaskWidget {
    pub fn get(&self) -> Entity {
        self.id
    }
}

impl FromWorld for TaskWidget {
    fn from_world(world: &mut World) -> Self {
        let widget = Widget::new::<ViewTasks>(world);
        let properties_panel = world.resource::<PropertiesPanel>().id();
        let id = world.spawn(widget).set_parent(properties_panel).id();
        Self { id }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct EditTask(Option<Entity>);

impl FromWorld for EditTask {
    fn from_world(_world: &mut World) -> Self {
        EditTask(None)
    }
}

#[derive(SystemParam)]
pub struct ViewTasks<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    commands: Commands<'w, 's>,
    current_scenario: ResMut<'w, CurrentScenario>,
    current_workspace: Res<'w, CurrentWorkspace>,
    delete: EventWriter<'w, Delete>,
    edit_task: ResMut<'w, EditTask>,
    pending_tasks: Query<'w, 's, (Entity, &'static mut Task), With<Pending>>,
    robots: Query<'w, 's, (Entity, &'static NameInSite), (With<Robot>, Without<Group>)>,
    scenarios: Query<'w, 's, (Entity, &'static mut Scenario<Entity>), With<ScenarioMarker>>,
    task_kinds: ResMut<'w, TaskKinds>,
    task_widget: ResMut<'w, TaskWidget>,
    tasks: Query<'w, 's, (Entity, &'static mut Task), Without<Pending>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewTasks<'w, 's> {
    fn show(
        Tile { id, panel }: Tile,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        CollapsingHeader::new("Tasks")
            .default_open(true)
            .show(ui, |ui| {
                let mut params = state.get_mut(world);
                params.show_widget(ui);

                if params.edit_task.0.is_some() {
                    let children: Result<SmallVec<[_; 16]>, _> = params
                        .children
                        .get(params.task_widget.id)
                        .map(|children| children.iter().copied().collect());
                    let Ok(children) = children else {
                        return;
                    };

                    for child in children {
                        let tile = Tile { id, panel };
                        let _ = world.try_show_in(child, tile, ui);
                        // ui.add_space(10.0);
                    }
                }
            });
    }
}

impl<'w, 's> ViewTasks<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let Some((_, mut scenario)) = self
            .current_scenario
            .0
            .and_then(|e| self.scenarios.get_mut(e).ok())
        else {
            ui.label("No scenario selected, unable to display or create tasks.");
            return;
        };

        // View and modify existing tasks
        let mut edit_existing_task: Option<Entity> = None;
        Frame::default()
            .inner_margin(4.0)
            .rounding(2.0)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                let mut id: usize = 0;
                for (task_entity, task) in self.tasks.iter() {
                    let present = scenario.tasks.contains(&task_entity);
                    let (edit, delete) =
                        show_task(ui, task_entity, &task, &mut scenario, &mut id, present);
                    if delete {
                        self.delete.send(Delete::new(task_entity));
                    } else if edit {
                        edit_existing_task = Some(task_entity);
                    }
                }
                if id == 0 {
                    ui.label("No tasks");
                }
            });
        ui.add_space(10.0);

        let mut cancel_task = false;
        if let Some(task_entity) = self.edit_task.0 {
            if let Ok((_, mut pending_task)) = self.pending_tasks.get_mut(task_entity) {
                if let Some(existing_task_entity) = edit_existing_task {
                    self.commands.entity(task_entity).despawn_recursive();
                    self.edit_task.0 = Some(existing_task_entity);
                    return;
                }
                ui.horizontal(|ui| {
                    ui.label("Creating Task");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            cancel_task = true;
                        }
                        ui.add_enabled_ui(pending_task.is_valid(), |ui| {
                            // TODO(@xiyuoh) Also check validity of TaskKind (e.g. GoToPlace)
                            if ui
                                .button("Add Task")
                                .on_hover_text("Add this task to the current scenario")
                                .clicked()
                            {
                                self.commands.entity(task_entity).remove::<Pending>();
                                scenario.tasks.insert(task_entity);
                                self.edit_task.0 = None;
                                return;
                            }
                        });
                    });
                });
                if cancel_task {
                    self.edit_task.0 = None;
                    return;
                }
                ui.separator();
                edit_task(
                    ui,
                    &mut self.commands,
                    task_entity,
                    &mut pending_task,
                    &self.task_kinds,
                    &self.robots,
                );
            } else {
                if let Some(existing_task) = edit_existing_task {
                    self.edit_task.0 = Some(existing_task);
                    return;
                }
                if let Ok((_, mut existing_task)) = self.tasks.get_mut(task_entity) {
                    ui.horizontal(|ui| {
                        ui.label("Editing Task");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_enabled_ui(existing_task.is_valid(), |ui| {
                                if ui.button("Done").clicked() {
                                    self.edit_task.0 = None;
                                    return;
                                }
                            });
                        });
                    });
                    ui.separator();
                    edit_task(
                        ui,
                        &mut self.commands,
                        task_entity,
                        &mut existing_task,
                        &self.task_kinds,
                        &self.robots,
                    );
                }
            }
        } else {
            self.edit_task.0 = edit_existing_task;
            if self.edit_task.0.is_none() {
                if ui.button("✚ Create New Task").clicked() {
                    self.edit_task.0 = Some(
                        self.commands
                            .spawn(Task::default())
                            .insert(Category::Task)
                            .insert(Pending)
                            .id(),
                    );
                    if let Some(site_entity) = self.current_workspace.root {
                        self.commands
                            .entity(self.edit_task.0.unwrap())
                            .set_parent(site_entity);
                    }
                }
            }
        }
    }
}

fn show_task(
    ui: &mut Ui,
    entity: Entity,
    task: &Task,
    scenario: &mut Scenario<Entity>,
    id: &mut usize,
    present: bool,
) -> (bool, bool) {
    let mut edit_task = false;
    let mut delete_task = false;
    let mut include_task = present.clone();
    let color = if present {
        Color32::DARK_GRAY
    } else {
        Color32::BLACK
    };
    Frame::default()
        .inner_margin(4.0)
        .fill(color)
        .rounding(2.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let id_string = id.to_string();

            ui.horizontal(|ui| {
                ui.label("Task ".to_owned() + &entity.index().to_string()); // TODO(@xiyuoh) better identifier
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button("❌")
                        .on_hover_text("Delete task from site")
                        .clicked()
                    {
                        delete_task = true;
                    }
                    if ui.button("Edit").clicked() {
                        edit_task = true;
                    }
                    ui.checkbox(&mut include_task, "")
                        .on_hover_text("Add to/Remove from current scenario");
                });
            });

            let task_request = task.request();
            Grid::new("show_task_".to_owned() + &id_string)
                .num_columns(2)
                .show(ui, |ui| {
                    match task {
                        Task::Dispatch(_) => {
                            ui.label("Dispatch:");
                            ui.label(
                                task_request
                                    .fleet_name()
                                    .unwrap_or("Unassigned".to_string()),
                            );
                            ui.end_row();
                        }
                        Task::Direct(_) => {
                            ui.label("Direct:");
                            ui.label(task.fleet().to_owned() + "/" + &task.robot());
                            ui.end_row();
                        }
                    }

                    ui.label("Kind:");
                    ui.label(task_request.category());
                    ui.end_row();

                    ui.label("Description:");
                    if let Some(description) = task_request.description_display() {
                        ui.label(description);
                    } else {
                        ui.label(format!("{:?}", task_request.description()));
                    }
                    ui.end_row();
                });

            CollapsingHeader::new("More details")
                .id_source("task_details_".to_owned() + &id_string)
                .default_open(false)
                .show(ui, |ui| {
                    Grid::new("task_details_".to_owned() + &id_string)
                        .num_columns(2)
                        .show(ui, |ui| {
                            // TODO(@xiyuoh) Add status/queued information

                            if let Some(start_time) = task_request.start_time() {
                                ui.label("Start time:");
                                ui.label(format!("{:?}", start_time));
                                ui.end_row();
                            }

                            if let Some(request_time) = task_request.request_time() {
                                ui.label("Request time:");
                                ui.label(format!("{:?}", request_time));
                                ui.end_row();
                            }

                            if let Some(priority) = task_request.priority() {
                                ui.label("Priority:");
                                ui.label(format!("{:?}", priority));
                                ui.end_row();
                            }

                            let labels = task_request.labels();
                            if !labels.is_empty() {
                                ui.label("Labels:");
                                ui.label(format!("{:?}", labels));
                                ui.end_row();
                            }

                            if let Some(requester) = task_request.requester() {
                                ui.label("Requester:");
                                ui.label(format!("{}", requester));
                                ui.end_row();
                            }

                            if let Some(fleet_name) = task_request.fleet_name() {
                                ui.label("Fleet name:");
                                ui.label(format!("{}", fleet_name));
                                ui.end_row();
                            }
                        });
                });
        });
    *id += 1;
    if include_task && !present {
        scenario.tasks.insert(entity);
    } else if !include_task && present {
        scenario.tasks.remove(&entity);
        return (false, false);
    }
    (edit_task, delete_task)
}

fn edit_task(
    ui: &mut Ui,
    commands: &mut Commands,
    task_entity: Entity,
    task: &mut Task,
    task_kinds: &ResMut<TaskKinds>,
    robots: &Query<(Entity, &NameInSite), (With<Robot>, Without<Group>)>,
) {
    Grid::new("edit_task_".to_owned() + &task_entity.index().to_string())
        .num_columns(2)
        .show(ui, |ui| {
            // Select Request Type
            let mut is_robot_task_request = task.is_direct();
            ui.label("Request Type:");
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(!is_robot_task_request, "Dispatch")
                    .on_hover_text("Create a Dispatch Task for any robot in the site")
                    .clicked()
                {
                    is_robot_task_request = false;
                };
                if ui
                    .selectable_label(is_robot_task_request, "Direct")
                    .on_hover_text("Create a Direct Task for a specific robot in a fleet")
                    .clicked()
                {
                    is_robot_task_request = true;
                }
            });
            ui.end_row();

            // Update Request Type and show RobotTaskRequest widget
            let current_request = task.request();
            if is_robot_task_request {
                if !task.is_direct() {
                    let robot = task.robot();
                    let fleet = task.fleet();
                    *task =
                        Task::Direct(RobotTaskRequest::new(robot, fleet, current_request.clone()));
                }
                if let Task::Direct(robot_task_request) = task {
                    ui.label("Fleet:");
                    ui.add(TextEdit::singleline(robot_task_request.fleet_mut()));
                    ui.end_row();

                    ui.label("Robot:");
                    let selected_robot = if robot_task_request.robot().is_empty() {
                        "Select Robot".to_string()
                    } else {
                        robot_task_request.robot()
                    };
                    ComboBox::from_id_source("select_robot_for_task")
                        .selected_text(selected_robot)
                        .show_ui(ui, |ui| {
                            for (_, robot) in robots.iter() {
                                ui.selectable_value(
                                    robot_task_request.robot_mut(),
                                    robot.0.clone(),
                                    robot.0.clone(),
                                );
                            }
                        });
                    ui.end_row();
                } else {
                    warn!("Unable to select Direct task!");
                }
            } else {
                if !task.is_dispatch() {
                    *task = Task::Dispatch(DispatchTaskRequest::new(current_request.clone()));
                }
            }
            // Show TaskRequest editing widget
            let current_category = task.request().category();
            let selected_task_kind = if task_kinds.0.contains_key(&current_category) {
                current_category.clone()
            } else {
                "Select Kind".to_string()
            };
            ui.label("Task Kind:");
            ComboBox::from_id_source("select_task_kind")
                .selected_text(selected_task_kind)
                .show_ui(ui, |ui| {
                    for (kind, _) in task_kinds.0.iter() {
                        ui.selectable_value(
                            task.request_mut().category_mut(),
                            kind.clone(),
                            kind.clone(),
                        );
                    }
                });
            ui.end_row();
            // Insert selected TaskKind component
            let new_category = task.request().category();
            if new_category != current_category {
                if let Some(remove_fn) = task_kinds.0.get(&current_category).map(|(_, rm_fn)| rm_fn)
                {
                    remove_fn(commands.entity(task_entity));
                }
                if let Some(insert_fn) = task_kinds.0.get(&new_category).map(|(is_fn, _)| is_fn) {
                    insert_fn(commands.entity(task_entity));
                }
            }
        });

    // More
    CollapsingHeader::new("More")
        .default_open(false)
        .show(ui, |ui| {
            let task_request = task.request_mut();

            Grid::new("edit_task_details")
                .num_columns(2)
                .show(ui, |ui| {
                    // Start time
                    ui.label("Start Time:")
                        .on_hover_text("(Optional) The earliest time that this task may start");
                    let mut has_start_time = task_request.start_time().is_some();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut has_start_time, "");
                        if has_start_time {
                            let start_time = task_request.start_time_mut().get_or_insert(0);
                            ui.add(
                                DragValue::new(start_time)
                                    .clamp_range(0_i32..=std::i32::MAX)
                                    .speed(1),
                            );
                        } else if task_request.start_time().is_some() {
                            *task_request.start_time_mut() = None;
                        }
                    });
                    ui.end_row();

                    // Request time
                    ui.label("Request Time:")
                        .on_hover_text("(Optional) The time that this request was initiated");
                    let mut has_request_time = task_request.request_time().is_some();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut has_request_time, "");
                        if has_request_time {
                            let request_time = task_request.request_time_mut().get_or_insert(0);
                            ui.add(
                                DragValue::new(request_time)
                                    .clamp_range(0_i32..=std::i32::MAX)
                                    .speed(1),
                            );
                        } else if task_request.request_time().is_some() {
                            *task_request.request_time_mut() = None;
                        }
                    });
                    ui.end_row();

                    // Priority
                    ui.label("Priority:").on_hover_text(
                        "(Optional) The priority of this task. \
                        This must match a priority schema supported by a fleet.",
                    );
                    let mut has_priority = task_request.priority().is_some();
                    ui.checkbox(&mut has_priority, "");
                    ui.end_row();
                    if has_priority {
                        if task_request.priority().is_none() {
                            *task_request.priority_mut() = Some(Value::Null);
                        }
                        // TODO(@xiyuoh) Expand on this to create fleet-specific priority widgets
                    } else if task_request.priority().is_some() {
                        *task_request.priority_mut() = None;
                    }

                    // Labels
                    ui.label("Labels:").on_hover_text(
                        "Labels to describe the purpose of the task dispatch request, \
                        items can be a single value like `dashboard` or a key-value pair \
                        like `app=dashboard`, in the case of a single value, it will be \
                        interpreted as a key-value pair with an empty string value.",
                    );
                    let mut remove_labels = Vec::new();
                    let mut id: usize = 0;
                    for label in task_request.labels_mut() {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("❌").on_hover_text("Remove label").clicked() {
                                remove_labels.push(id.clone());
                            }
                            ui.text_edit_singleline(label);
                        });
                        id += 1;
                        ui.end_row();
                        ui.label("");
                    }
                    if ui.button("✚").on_hover_text("Insert new label").clicked() {
                        task_request.labels_mut().push(String::new());
                    }
                    ui.end_row();
                    for i in remove_labels.drain(..).rev() {
                        task_request.labels_mut().remove(i);
                    }

                    // Requester
                    ui.label("Requester:").on_hover_text(
                        "(Optional) An identifier for the entity that requested this task",
                    );
                    let requester = task_request.requester_mut().get_or_insert(String::new());
                    ui.text_edit_singleline(requester);
                    if requester.is_empty() {
                        *task_request.requester_mut() = None;
                    }
                    ui.end_row();

                    // Fleet name
                    ui.label("Fleet name:").on_hover_text(
                        "(Optional) The name of the fleet that should perform this task. \
                        If specified, other fleets will not bid for this task.",
                    );
                    // TODO(@xiyuoh) when available, insert combobox of registered fleets
                    let fleet_name = task_request.fleet_name_mut().get_or_insert(String::new());
                    ui.text_edit_singleline(fleet_name);
                    if fleet_name.is_empty() {
                        *task_request.fleet_name_mut() = None;
                    }
                    ui.end_row();
                });
        });
    ui.separator();
}
