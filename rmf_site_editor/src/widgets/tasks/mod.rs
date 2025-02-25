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
            .init_resource::<EditTask>()
            .add_event::<UpdateTask>()
            .add_systems(PostUpdate, handle_task_updates);
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

pub enum UpdateTaskType {
    New(Entity),
    Include(Entity),
    Hide(Entity),
    Edit(Option<Entity>),
    Reset,
}

#[derive(Event)]
pub struct UpdateTask {
    pub scenario: Entity,
    pub update_type: UpdateTaskType,
}

#[derive(SystemParam)]
pub struct ViewTasks<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    commands: Commands<'w, 's>,
    current_scenario: ResMut<'w, CurrentScenario>,
    delete: EventWriter<'w, Delete>,
    edit_task: ResMut<'w, EditTask>,
    pending_tasks: Query<'w, 's, (Entity, &'static mut Task), With<Pending>>,
    robots: Query<'w, 's, (Entity, &'static NameInSite), (With<Robot>, Without<Group>)>,
    scenarios: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static mut Scenario<Entity>),
        With<ScenarioMarker>,
    >,
    task_kinds: ResMut<'w, TaskKinds>,
    task_widget: ResMut<'w, TaskWidget>,
    tasks: Query<'w, 's, (Entity, &'static mut Task), Without<Pending>>,
    update_task: EventWriter<'w, UpdateTask>,
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
                    }
                }
            });
        ui.add_space(10.0);
    }
}

impl<'w, 's> ViewTasks<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let Some(current_scenario_entity) = self.current_scenario.0 else {
            return;
        };
        let Ok((_, scenario_name, scenario)) = self.scenarios.get(current_scenario_entity) else {
            ui.label("No scenario selected, unable to display or create tasks.");
            return;
        };

        // View and modify tasks in current scenario
        ui.horizontal(|ui| {
            ui.label(scenario_name.0.clone());
            let parent_tasks = scenario
                .parent_scenario
                .0
                .and_then(|p| self.scenarios.get(p).ok())
                .map(|(_, _, ps)| ps.tasks.clone());

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_enabled_ui(parent_tasks.is_some_and(|pt| pt != scenario.tasks), |ui| {
                    if ui
                        .button("↩")
                        .on_hover_text("Reset to parent scenario tasks")
                        .clicked()
                    {
                        self.update_task.send(UpdateTask {
                            scenario: current_scenario_entity,
                            update_type: UpdateTaskType::Reset,
                        })
                    }
                });
            });
        });
        Frame::default()
            .inner_margin(4.0)
            .rounding(2.0)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                let mut id: usize = 0;
                for task_entity in scenario.tasks.iter() {
                    let Ok((_, task)) = self.tasks.get(*task_entity) else {
                        continue;
                    };
                    let scenario_count = count_scenarios(&self.scenarios, *task_entity);
                    show_task(
                        ui,
                        *task_entity,
                        &task,
                        current_scenario_entity,
                        &mut self.update_task,
                        &mut self.delete,
                        &mut id,
                        true,
                        scenario_count,
                    );
                }
                if id == 0 {
                    ui.label("No tasks");
                }
            });
        ui.add_space(10.0);

        // View other created tasks from this site
        CollapsingHeader::new("Other Scenarios")
            .id_source("show_all_scenario_tasks")
            .default_open(false)
            .show(ui, |ui| {
                Frame::default()
                    .inner_margin(4.0)
                    .rounding(2.0)
                    .stroke(Stroke::new(1.0, Color32::GRAY))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        let mut id: usize = 0;
                        for (task_entity, task) in self.tasks.iter() {
                            if scenario.tasks.contains(&task_entity) {
                                continue;
                            }
                            let scenario_count = count_scenarios(&self.scenarios, task_entity);
                            show_task(
                                ui,
                                task_entity,
                                &task,
                                current_scenario_entity,
                                &mut self.update_task,
                                &mut self.delete,
                                &mut id,
                                false,
                                scenario_count,
                            );
                        }
                        if id == 0 {
                            ui.label("No tasks from other scenarios");
                        }
                    });
            });
        ui.separator();

        let mut reset_edit: bool = false;

        if let Some(task_entity) = self.edit_task.0 {
            if let Ok((_, mut pending_task)) = self.pending_tasks.get_mut(task_entity) {
                ui.horizontal(|ui| {
                    ui.label("Creating Task");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            reset_edit = true;
                        }
                        ui.add_enabled_ui(pending_task.is_valid(), |ui| {
                            // TODO(@xiyuoh) Also check validity of TaskKind (e.g. GoToPlace)
                            if ui
                                .button("Add Task")
                                .on_hover_text("Add this task to the current scenario")
                                .clicked()
                            {
                                self.commands.entity(task_entity).remove::<Pending>();
                                self.update_task.send(UpdateTask {
                                    scenario: current_scenario_entity,
                                    update_type: UpdateTaskType::Include(task_entity),
                                });
                                reset_edit = true;
                            }
                        });
                    });
                });
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
                if let Ok((_, mut existing_task)) = self.tasks.get_mut(task_entity) {
                    ui.horizontal(|ui| {
                        ui.label("Editing Task");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_enabled_ui(existing_task.is_valid(), |ui| {
                                if ui.button("Done").clicked() {
                                    reset_edit = true;
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
            if ui.button("✚ Create New Task").clicked() {
                let new_task = self
                    .commands
                    .spawn(Task::default())
                    .insert(Category::Task)
                    .insert(Pending)
                    .id();
                self.update_task.send(UpdateTask {
                    scenario: current_scenario_entity,
                    update_type: UpdateTaskType::New(new_task),
                });
            }
        }

        if reset_edit {
            self.update_task.send(UpdateTask {
                scenario: current_scenario_entity,
                update_type: UpdateTaskType::Edit(None),
            });
        }
    }
}

pub fn count_scenarios(
    scenarios: &Query<(Entity, &NameInSite, &mut Scenario<Entity>), With<ScenarioMarker>>,
    task: Entity,
) -> i32 {
    scenarios.iter().fold(0, |x, (_, _, s)| {
        if s.tasks.iter().any(|e| *e == task) {
            x + 1
        } else {
            x
        }
    })
}

fn show_task(
    ui: &mut Ui,
    entity: Entity,
    task: &Task,
    scenario: Entity,
    update_task: &mut EventWriter<UpdateTask>,
    delete: &mut EventWriter<Delete>,
    id: &mut usize,
    present: bool,
    scenario_count: i32,
) {
    let color = if present {
        Color32::DARK_GRAY
    } else {
        Color32::default()
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
                        delete.send(Delete::new(entity));
                    }
                    if present {
                        if ui
                            .button("Remove")
                            .on_hover_text("Remove task from this scenario")
                            .clicked()
                        {
                            update_task.send(UpdateTask {
                                scenario,
                                update_type: UpdateTaskType::Hide(entity),
                            })
                        }
                        if ui
                            .button("Edit") // Do not allow edit if not in this scenario
                            .on_hover_text("Edit task parameters")
                            .clicked()
                        {
                            update_task.send(UpdateTask {
                                scenario,
                                update_type: UpdateTaskType::Edit(Some(entity)),
                            })
                        }
                    } else {
                        if ui
                            .button("Add")
                            .on_hover_text("Add task to this scenario")
                            .clicked()
                        {
                            update_task.send(UpdateTask {
                                scenario,
                                update_type: UpdateTaskType::Include(entity),
                            })
                        }
                    }
                    ui.label(format!("[{}]", scenario_count))
                        .on_hover_text("Number of scenarios this task is included in");
                });
            });
            ui.separator();

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
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        if ui
                            .button("Add label")
                            .on_hover_text("Insert new label")
                            .clicked()
                        {
                            task_request.labels_mut().push(String::new());
                        }
                    });
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
}

fn handle_task_updates(
    mut commands: Commands,
    mut scenarios: Query<&mut Scenario<Entity>>,
    mut update_task: EventReader<UpdateTask>,
    mut edit_task: ResMut<EditTask>,
    current_workspace: Res<CurrentWorkspace>,
    pending_tasks: Query<(Entity, &mut Task), With<Pending>>,
    parents: Query<&Parent>,
) {
    for update in update_task.read() {
        let parent_tasks = parents
            .get(update.scenario)
            .and_then(|p| scenarios.get(p.get()))
            .map(|ps| ps.tasks.clone());

        let Ok(mut scenario) = scenarios.get_mut(update.scenario) else {
            return;
        };

        match update.update_type {
            UpdateTaskType::New(task_entity) => {
                if let Some(site_entity) = current_workspace.root {
                    commands.entity(task_entity).set_parent(site_entity);
                }
                edit_task.0 = Some(task_entity);
            }
            UpdateTaskType::Include(task_entity) => {
                scenario.tasks.push(task_entity);
            }
            UpdateTaskType::Hide(task_entity) => {
                scenario.tasks.retain(|e| *e != task_entity);
            }
            UpdateTaskType::Edit(task_entity) => {
                if let Some(pending_task) = edit_task.0.filter(|e| pending_tasks.get(*e).is_ok()) {
                    commands.entity(pending_task).despawn_recursive();
                }
                edit_task.0 = task_entity;
            }
            UpdateTaskType::Reset => {
                if let Ok(parent_tasks) = parent_tasks {
                    if scenario.tasks != parent_tasks {
                        scenario.tasks = parent_tasks;
                    }
                }
            }
        }
    }
}
