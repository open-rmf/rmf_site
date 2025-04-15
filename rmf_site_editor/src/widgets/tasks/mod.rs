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
    interaction::{Select, Selection},
    site::{
        AddedTask, Affiliation, Category, ChangeCurrentScenario, CurrentScenario, Delete,
        DispatchTaskRequest, Group, NameInSite, Pending, RecallTask, Robot, RobotTaskRequest,
        ScenarioMarker, Task, TaskModifier, TaskParams,
    },
    widgets::prelude::*,
    CurrentWorkspace, Icons, Tile, WidgetSystem,
};
use bevy::{
    ecs::system::{EntityCommands, SystemParam, SystemState},
    prelude::*,
};
use bevy_egui::egui::{
    Align, CollapsingHeader, Color32, ComboBox, DragValue, Frame, Grid, ImageButton, Layout,
    Stroke, TextEdit, Ui,
};
use rmf_site_format::InheritedTask;
use serde_json::Value;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

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
            .add_event::<EditModeEvent>()
            .add_event::<UpdateTaskEvent>()
            .add_event::<UpdateTaskModifierEvent>()
            .add_systems(
                PostUpdate,
                (
                    handle_task_modifier_updates,
                    handle_task_updates,
                    insert_new_task_modifiers,
                    update_current_scenario_tasks,
                    handle_invalid_modifiers,
                ),
            );
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

#[derive(Clone, Debug)]
pub enum EditMode {
    New(Entity),
    Edit(Option<Entity>),
}

#[derive(Clone, Debug, Event)]
pub struct EditModeEvent {
    pub scenario: Entity,
    pub mode: EditMode,
}

#[derive(Clone, Debug, Event)]
pub struct UpdateTaskEvent {
    pub scenario: Entity,
    pub entity: Entity,
    pub task: Task,
}

#[derive(Clone, Debug)]
pub enum UpdateTaskModifier {
    Include,
    Hide,
    Modify(TaskParams),
    Reset,
}

#[derive(Clone, Debug, Event)]
pub struct UpdateTaskModifierEvent {
    pub scenario: Entity,
    pub task: Entity,
    pub update: UpdateTaskModifier,
}

#[derive(SystemParam)]
pub struct ViewTasks<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    commands: Commands<'w, 's>,
    current_scenario: ResMut<'w, CurrentScenario>,
    delete: EventWriter<'w, Delete>,
    edit_mode: EventWriter<'w, EditModeEvent>,
    edit_task: ResMut<'w, EditTask>,
    icons: Res<'w, Icons>,
    pending_tasks: Query<'w, 's, (Entity, &'static Task, &'static TaskParams), With<Pending>>,
    robots: Query<'w, 's, (Entity, &'static NameInSite), (With<Robot>, Without<Group>)>,
    scenarios: Query<'w, 's, (Entity, &'static Affiliation<Entity>), With<ScenarioMarker>>,
    task_modifiers: Query<'w, 's, (&'static mut TaskModifier, &'static Affiliation<Entity>)>,
    task_kinds: ResMut<'w, TaskKinds>,
    task_widget: ResMut<'w, TaskWidget>,
    tasks: Query<'w, 's, (Entity, &'static Task, &'static TaskParams), Without<Pending>>,
    update_task: EventWriter<'w, UpdateTaskEvent>,
    update_task_modifier: EventWriter<'w, UpdateTaskModifierEvent>,
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
            ui.label("No scenario selected, unable to display or create tasks.");
            return;
        };

        // View and modify tasks in current scenario
        Frame::default()
            .inner_margin(4.0)
            .rounding(2.0)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                // TODO(@xiyuoh) Use ordered structure instead to prevent glitching
                let task_modifier_entities = get_task_modifier_entities(
                    current_scenario_entity,
                    &self.children,
                    &self.task_modifiers,
                );
                let scenario_task_modifiers = task_modifier_entities.iter().fold(
                    HashMap::new(),
                    |mut x, (task_entity, modifier_entity)| {
                        if let Some((modifier, _)) = self
                            .task_modifiers
                            .get(*modifier_entity)
                            .ok()
                            .filter(|_| self.tasks.get(*task_entity).is_ok())
                        {
                            x.insert(*task_entity, modifier.clone());
                            x
                        } else {
                            x
                        }
                    },
                );
                for (task_entity, task_modifier) in scenario_task_modifiers.iter() {
                    let scenario_count = count_scenarios_for_tasks(
                        &self.scenarios,
                        *task_entity,
                        &self.children,
                        &self.task_modifiers,
                    );
                    show_task(
                        ui,
                        *task_entity,
                        task_modifier,
                        current_scenario_entity,
                        &self.tasks,
                        &mut self.edit_mode,
                        &mut self.update_task_modifier,
                        &mut self.delete,
                        scenario_count,
                        &self.icons,
                    );
                }
                if scenario_task_modifiers.len() == 0 {
                    ui.label("No tasks in this scenario");
                }
            });
        ui.add_space(10.0);
        ui.separator();

        let mut reset_edit: bool = false;

        if let Some(task_entity) = self.edit_task.0 {
            if let Ok((_, pending_task, pending_task_params)) =
                self.pending_tasks.get_mut(task_entity)
            {
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
                                // Add to the current scenario
                                self.commands.entity(task_entity).remove::<Pending>();
                                reset_edit = true;
                            }
                        });
                    });
                });
                ui.separator();
                edit_task(
                    ui,
                    &mut self.commands,
                    current_scenario_entity,
                    task_entity,
                    pending_task,
                    pending_task_params,
                    &self.task_kinds,
                    &self.robots,
                    &mut self.update_task,
                    &mut self.update_task_modifier,
                );
            } else {
                if let Ok((_, existing_task, existing_task_params)) =
                    self.tasks.get_mut(task_entity)
                {
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
                        current_scenario_entity,
                        task_entity,
                        existing_task,
                        existing_task_params,
                        &self.task_kinds,
                        &self.robots,
                        &mut self.update_task,
                        &mut self.update_task_modifier,
                    );
                }
            }
        } else {
            if ui.button("✚ Create New Task").clicked() {
                let new_task = self
                    .commands
                    .spawn(Task::default())
                    .insert(Category::Task)
                    .insert(TaskParams::default())
                    .insert(Pending)
                    .id();
                self.edit_mode.send(EditModeEvent {
                    scenario: current_scenario_entity,
                    mode: EditMode::New(new_task),
                });
            }
        }

        if reset_edit {
            self.edit_mode.send(EditModeEvent {
                scenario: current_scenario_entity,
                mode: EditMode::Edit(None),
            });
        }
    }
}

pub fn count_scenarios_for_tasks(
    scenarios: &Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
    task: Entity,
    children: &Query<&Children>,
    task_modifiers: &Query<(&mut TaskModifier, &Affiliation<Entity>)>,
) -> i32 {
    scenarios.iter().fold(0, |x, (e, _)| {
        if find_modifier_for_task(task, e, &children, &task_modifiers)
            .and_then(|modifier_entity| task_modifiers.get(modifier_entity).ok())
            .is_some_and(|(modifier, _)| match modifier {
                TaskModifier::Hidden => false,
                _ => true,
            })
        {
            x + 1
        } else {
            x
        }
    })
}

fn show_task(
    ui: &mut Ui,
    task_entity: Entity,
    task_modifier: &TaskModifier,
    scenario: Entity,
    tasks: &Query<(Entity, &Task, &TaskParams), Without<Pending>>,
    edit_mode: &mut EventWriter<EditModeEvent>,
    update_task_modifier: &mut EventWriter<UpdateTaskModifierEvent>,
    delete: &mut EventWriter<Delete>,
    scenario_count: i32,
    icons: &Res<Icons>,
) {
    let (color, present) = match task_modifier {
        TaskModifier::Added(_) | TaskModifier::Inherited(_) => (Color32::DARK_GRAY, true),
        TaskModifier::Hidden => (Color32::default(), false),
    };
    Frame::default()
        .inner_margin(4.0)
        .fill(color)
        .rounding(2.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label("Task ".to_owned() + &task_entity.index().to_string())  // TODO(@xiyuoh) better identifier
                    .on_hover_text(format!("Task is included in {} scenarios", scenario_count));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(ImageButton::new(icons.trash.egui()))
                        .on_hover_text("Remove task from all scenarios")
                        .clicked()
                    {
                        delete.send(Delete::new(task_entity));
                    }
                    if present {
                        if ui
                            .add(ImageButton::new(icons.show.egui()))
                            .on_hover_text("Exclude task from current scenario")
                            .clicked()
                        {
                            update_task_modifier.send(UpdateTaskModifierEvent {
                                scenario,
                                task: task_entity,
                                update: UpdateTaskModifier::Hide,
                            })
                        }
                        // Do not allow edit if not in current scenario
                        if ui
                            .add(ImageButton::new(icons.edit.egui()))
                            .on_hover_text("Edit task parameters")
                            .clicked()
                        {
                            edit_mode.send(EditModeEvent {
                                scenario,
                                mode: EditMode::Edit(Some(task_entity)),
                            })
                        }
                    } else {
                        if ui
                            .add(ImageButton::new(icons.hide.egui()))
                            .on_hover_text("Include task in current scenario")
                            .clicked()
                        {
                            update_task_modifier.send(UpdateTaskModifierEvent {
                                scenario,
                                task: task_entity,
                                update: UpdateTaskModifier::Include,
                            })
                        }
                    }
                });
            });
            if !present {
                return;
            }
            ui.separator();

            let Ok((_, task, task_params)) = tasks.get(task_entity) else {
                return;
            };
            let task_request = task.request();
            Grid::new("show_task_".to_owned() + &task_entity.index().to_string())
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
                    ui.label(
                        task_request
                            .description_display()
                            .unwrap_or("None".to_string()),
                    );
                    ui.end_row();

                    ui.label("Requester:");
                    ui.label(task_request.requester().unwrap_or("None".to_string()));
                    ui.end_row();

                    ui.label("Fleet name:");
                    ui.label(task_request.fleet_name().unwrap_or("None".to_string()));
                    ui.end_row();
                });

            CollapsingHeader::new("More details")
                .id_source("task_details_".to_owned() + &task_entity.index().to_string())
                .default_open(false)
                .show(ui, |ui| {
                    Grid::new("task_details_".to_owned() + &task_entity.index().to_string())
                        .num_columns(2)
                        .show(ui, |ui| {
                            // TODO(@xiyuoh) Add status/queued information
                            ui.label("Start time:");
                            ui.label(
                                task_params
                                    .start_time()
                                    .map(|rt| format!("{:?}", rt))
                                    .unwrap_or("None".to_string()),
                            );
                            ui.end_row();

                            ui.label("Request time:");
                            ui.label(
                                task_params
                                    .request_time()
                                    .map(|rt| format!("{:?}", rt))
                                    .unwrap_or("None".to_string()),
                            );
                            ui.end_row();

                            ui.label("Priority:");
                            ui.label(
                                task_params
                                    .priority()
                                    .map(|st| st.to_string())
                                    .unwrap_or("None".to_string()),
                            );
                            ui.end_row();

                            ui.label("Labels:");
                            ui.label(format!("{:?}", task_params.labels()));
                            ui.end_row();
                        });
                });
        });
}

fn edit_task(
    ui: &mut Ui,
    commands: &mut Commands,
    scenario: Entity,
    task_entity: Entity,
    task: &Task,
    task_params: &TaskParams,
    task_kinds: &ResMut<TaskKinds>,
    robots: &Query<(Entity, &NameInSite), (With<Robot>, Without<Group>)>,
    update_task: &mut EventWriter<UpdateTaskEvent>,
    update_task_modifier: &mut EventWriter<UpdateTaskModifierEvent>,
) {
    Grid::new("edit_task_".to_owned() + &task_entity.index().to_string())
        .num_columns(2)
        .show(ui, |ui| {
            let mut new_task = task.clone();

            // Select Request Type
            let mut is_robot_task_request = new_task.is_direct();
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
            let task_request = new_task.request();
            if is_robot_task_request {
                if !new_task.is_direct() {
                    let robot = task.robot();
                    let fleet = task.fleet();
                    new_task =
                        Task::Direct(RobotTaskRequest::new(robot, fleet, task_request.clone()));
                }
                if let Task::Direct(ref mut robot_task_request) = new_task {
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
                if !new_task.is_dispatch() {
                    new_task = Task::Dispatch(DispatchTaskRequest::new(task_request.clone()));
                }
            }
            // Show TaskRequest editing widget
            let current_category = new_task.request().category();
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
                            new_task.request_mut().category_mut(),
                            kind.clone(),
                            kind.clone(),
                        );
                    }
                });
            ui.end_row();
            // Insert selected TaskKind component
            let new_category = new_task.request().category();
            if new_category != current_category {
                if let Some(remove_fn) = task_kinds.0.get(&current_category).map(|(_, rm_fn)| rm_fn)
                {
                    remove_fn(commands.entity(task_entity));
                }
                if let Some(insert_fn) = task_kinds.0.get(&new_category).map(|(is_fn, _)| is_fn) {
                    insert_fn(commands.entity(task_entity));
                }
            }

            let new_task_request = new_task.request_mut();

            // Requester
            ui.label("Requester:")
                .on_hover_text("(Optional) An identifier for the entity that requested this task");
            let requester = new_task_request
                .requester_mut()
                .get_or_insert(String::new());
            ui.text_edit_singleline(requester);
            if requester.is_empty() {
                *new_task_request.requester_mut() = None;
            }
            ui.end_row();

            // Fleet name
            ui.label("Fleet name:").on_hover_text(
                "(Optional) The name of the fleet that should perform this task. \
                If specified, other fleets will not bid for this task.",
            );
            // TODO(@xiyuoh) when available, insert combobox of registered fleets
            let fleet_name = new_task_request
                .fleet_name_mut()
                .get_or_insert(String::new());
            ui.text_edit_singleline(fleet_name);
            if fleet_name.is_empty() {
                *new_task_request.fleet_name_mut() = None;
            }
            ui.end_row();

            if new_task != *task {
                update_task.send(UpdateTaskEvent {
                    scenario,
                    entity: task_entity,
                    task: new_task,
                });
            } else {
            }
        });

    // More
    CollapsingHeader::new("More")
        .default_open(false)
        .show(ui, |ui| {
            Grid::new("edit_task_details")
                .num_columns(2)
                .show(ui, |ui| {
                    let mut new_task_params = task_params.clone();

                    // Start time
                    ui.label("Start Time:")
                        .on_hover_text("(Optional) The earliest time that this task may start");
                    let start_time = new_task_params.start_time();
                    let mut has_start_time = start_time.is_some();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut has_start_time, "");
                        if has_start_time {
                            let new_start_time = new_task_params.start_time_mut().get_or_insert(0);
                            ui.add(
                                DragValue::new(new_start_time)
                                    .clamp_range(0_i32..=std::i32::MAX)
                                    .speed(1),
                            );
                        } else if start_time.is_some() {
                            *new_task_params.start_time_mut() = None;
                        }
                    });
                    ui.end_row();

                    // Request time
                    ui.label("Request Time:")
                        .on_hover_text("(Optional) The time that this request was initiated");
                    let request_time = new_task_params.request_time();
                    let mut has_request_time = request_time.is_some();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut has_request_time, "");
                        if has_request_time {
                            let new_request_time =
                                new_task_params.request_time_mut().get_or_insert(0);
                            ui.add(
                                DragValue::new(new_request_time)
                                    .clamp_range(0_i32..=std::i32::MAX)
                                    .speed(1),
                            );
                        } else if request_time.is_some() {
                            *new_task_params.request_time_mut() = None;
                        }
                    });
                    ui.end_row();

                    // Priority
                    ui.label("Priority:").on_hover_text(
                        "(Optional) The priority of this task. \
                        This must match a priority schema supported by a fleet.",
                    );
                    let priority = new_task_params.priority();
                    let mut has_priority = priority.is_some();
                    ui.checkbox(&mut has_priority, "");
                    ui.end_row();
                    if has_priority {
                        if priority.is_none() {
                            *new_task_params.priority_mut() = Some(Value::Null);
                        }
                        // TODO(@xiyuoh) Expand on this to create fleet-specific priority widgets
                    } else if priority.is_some() {
                        *new_task_params.priority_mut() = None;
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
                    for label in new_task_params.labels_mut() {
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
                            new_task_params.labels_mut().push(String::new());
                        }
                    });
                    ui.end_row();
                    for i in remove_labels.drain(..).rev() {
                        new_task_params.labels_mut().remove(i);
                    }

                    if new_task_params != *task_params {
                        update_task_modifier.send(UpdateTaskModifierEvent {
                            scenario,
                            task: task_entity,
                            update: UpdateTaskModifier::Modify(new_task_params),
                        });
                    }
                });
        });
}

/// This system climbs up the scenario tree to retrieve inherited params for a task, if any
fn retrieve_parent_params(
    task_entity: Entity,
    scenario_entity: Entity,
    children: &Query<&Children>,
    recall_task: &Query<&RecallTask>,
    scenarios: &Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
    task_modifiers: &Query<(&mut TaskModifier, &Affiliation<Entity>)>,
) -> Option<TaskParams> {
    let mut parent_params: Option<TaskParams> = None;
    let mut entity = scenario_entity;
    while parent_params.is_none() {
        let Some(parent_entity) = scenarios.get(entity).ok().and_then(|(_, a)| a.0) else {
            break;
        };

        if let Some(modifier_entity) =
            find_modifier_for_task(task_entity, parent_entity, children, task_modifiers)
        {
            parent_params = task_modifiers.get(modifier_entity).ok().and_then(|(t, _)| {
                t.params().or_else(|| {
                    recall_task
                        .get(modifier_entity)
                        .ok()
                        .and_then(|r| r.params.clone())
                })
            });
        }
        entity = parent_entity;
    }
    parent_params
}

/// This system searches for the TaskModifier affiliated with a specific task if any
fn find_modifier_for_task(
    task: Entity,
    scenario: Entity,
    children: &Query<&Children>,
    task_modifiers: &Query<(&mut TaskModifier, &Affiliation<Entity>)>,
) -> Option<Entity> {
    if let Ok(scenario_children) = children.get(scenario) {
        for child in scenario_children.iter() {
            if task_modifiers
                .get(*child)
                .is_ok_and(|(_, a)| a.0.is_some_and(|e| e == task))
            {
                return Some(*child);
            }
        }
    };
    None
}

/// This system searches for scenario children entities with the TaskModifier component
/// and maps the affiliated task entity to the corresponding task modifier entity
// TODO(@xiyuoh) Generalize this system to all modifiers
fn get_task_modifier_entities(
    scenario: Entity,
    children: &Query<&Children>,
    task_modifiers: &Query<(&mut TaskModifier, &Affiliation<Entity>)>,
) -> HashMap<Entity, Entity> {
    let mut task_to_modifier_entities = HashMap::<Entity, Entity>::new();
    if let Ok(scenario_children) = children.get(scenario) {
        for child in scenario_children.iter() {
            if let Some(affiliated_entity) = task_modifiers.get(*child).ok().and_then(|(_, a)| a.0)
            {
                task_to_modifier_entities.insert(affiliated_entity, *child);
            }
        }
    };
    task_to_modifier_entities
}

// TODO(@xiyuoh) Generalize this system to all modifiers
fn insert_new_task_modifiers(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut task_modifiers: Query<(&mut TaskModifier, &Affiliation<Entity>)>,
    children: Query<&Children>,
    current_scenario: Res<CurrentScenario>,
    scenarios: Query<(Entity, Ref<Affiliation<Entity>>), With<ScenarioMarker>>,
    tasks: Query<(Entity, Ref<TaskParams>), With<Task>>,
) {
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };
    // Insert task modifier entities when new scenarios are created
    for (scenario_entity, parent_scenario) in scenarios.iter() {
        if parent_scenario.is_added() {
            if let Some(parent_scenario_entity) = parent_scenario.0 {
                // Inherit any task modifiers from the parent scenario
                if let Ok(children) = children.get(parent_scenario_entity) {
                    children.iter().for_each(|e| {
                        if let Ok((task_modifier, affiliation)) = task_modifiers
                            .get(*e)
                            .map(|(_, a)| (TaskModifier::inherited(), a.clone()))
                        {
                            commands
                                .spawn(task_modifier)
                                .insert(affiliation)
                                .set_parent(scenario_entity);
                        }
                    });
                }
            } else {
                // If root scenario, mark all task modifiers as Hidden
                for (task_entity, _) in tasks.iter() {
                    commands
                        .spawn(TaskModifier::Hidden)
                        .insert(Affiliation(Some(task_entity)))
                        .set_parent(scenario_entity);
                }
            }
            change_current_scenario.send(ChangeCurrentScenario(scenario_entity));
        }
    }

    // Insert task modifier entities for new pending tasks
    for (task_entity, task_params) in tasks.iter() {
        if task_params.is_added() {
            if let Some((mut task_modifier, _)) = find_modifier_for_task(
                task_entity,
                current_scenario_entity,
                &children,
                &task_modifiers,
            )
            .and_then(|modifier_entity| task_modifiers.get_mut(modifier_entity).ok())
            {
                // If a task modifier entity already exists for this scenario, update it
                let task_modifier = task_modifier.as_mut();
                match task_modifier {
                    TaskModifier::Added(_) => {
                        *task_modifier = TaskModifier::added(task_params.clone())
                    }
                    TaskModifier::Inherited(inherited) => {
                        inherited.modified_params = Some(task_params.clone())
                    }
                    TaskModifier::Hidden => {}
                }
            } else {
                // If task modifier entity does not exist in this scenario, spawn one
                commands
                    .spawn(TaskModifier::added(task_params.clone()))
                    .insert(Affiliation(Some(task_entity)))
                    .set_parent(current_scenario_entity);
            }

            // Insert task modifier into remaining scenarios
            for (scenario_entity, parent_scenario) in scenarios.iter() {
                if scenario_entity == current_scenario_entity {
                    continue;
                }

                // Crawl up scenario tree to check if this is a descendent of the current scenario
                let mut parent_entity: Option<Entity> = parent_scenario.0.clone();
                while parent_entity.is_some() {
                    if parent_entity.is_some_and(|e| e == current_scenario_entity) {
                        break;
                    }
                    parent_entity = parent_entity
                        .and_then(|e| scenarios.get(e).ok())
                        .and_then(|(_, a)| a.0);
                }

                // If task modifier entity does not exist in this child scenario, spawn one
                // Do nothing if it already exists, as it may be modified
                if find_modifier_for_task(task_entity, scenario_entity, &children, &task_modifiers)
                    .is_none()
                {
                    if parent_entity.is_some_and(|e| e == current_scenario_entity) {
                        // Insert this new task modifier into children scenarios as Inherited
                        commands
                            .spawn(TaskModifier::inherited())
                            .insert(Affiliation(Some(task_entity)))
                            .set_parent(scenario_entity);
                    } else {
                        // Insert this new task modifier into other scenarios as Hidden
                        commands
                            .spawn(TaskModifier::Hidden)
                            .insert(Affiliation(Some(task_entity)))
                            .set_parent(scenario_entity);
                    }
                }
            }
        }
    }
}

fn handle_task_updates(
    mut commands: Commands,
    mut edit_mode: EventReader<EditModeEvent>,
    mut edit_task: ResMut<EditTask>,
    mut pending_tasks: Query<&mut Task, With<Pending>>,
    mut tasks: Query<&mut Task, Without<Pending>>,
    mut update_task: EventReader<UpdateTaskEvent>,
    current_workspace: Res<CurrentWorkspace>,
) {
    // Update pending task for editing
    for edit in edit_mode.read() {
        match edit.mode {
            EditMode::New(task_entity) => {
                if let Some(site_entity) = current_workspace.root {
                    commands.entity(task_entity).set_parent(site_entity);
                }
                edit_task.0 = Some(task_entity);
            }
            EditMode::Edit(task_entity) => {
                if let Some(pending_task) = edit_task.0.filter(|e| pending_tasks.get(*e).is_ok()) {
                    commands.entity(pending_task).despawn_recursive();
                }
                edit_task.0 = task_entity;
            }
        }
    }

    // Update Task
    for update in update_task.read() {
        if let Ok(mut task) = tasks
            .get_mut(update.entity)
            .or(pending_tasks.get_mut(update.entity))
        {
            *task = update.task.clone();
        } else {
        }
    }
}

fn handle_task_modifier_updates(
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut task_modifiers: Query<(&mut TaskModifier, &Affiliation<Entity>)>,
    mut update_task_modifier: EventReader<UpdateTaskModifierEvent>,
    children: Query<&Children>,
    current_scenario: Res<CurrentScenario>,
    recall_task: Query<&RecallTask>,
    scenarios: Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
) {
    for update in update_task_modifier.read() {
        let Some(((mut task_modifier, _), modifier_entity)) =
            find_modifier_for_task(update.task, update.scenario, &children, &task_modifiers)
                .and_then(|modifier_entity| {
                    task_modifiers
                        .get_mut(modifier_entity)
                        .ok()
                        .zip(Some(modifier_entity))
                })
        else {
            continue;
        };
        let task_modifier = task_modifier.as_mut();
        let has_parent = scenarios
            .get(update.scenario)
            .is_ok_and(|(_, a)| a.0.is_some());

        match &update.update {
            UpdateTaskModifier::Include => {
                let recall_modifier = recall_task.get(modifier_entity).ok();
                let task_params = task_modifier
                    .params()
                    .or(recall_modifier.and_then(|r| r.params.clone()));
                if has_parent
                    && recall_modifier.is_some_and(|m| match m.modifier {
                        Some(TaskModifier::Inherited(_)) => true,
                        _ => false,
                    })
                {
                    *task_modifier = TaskModifier::Inherited(InheritedTask {
                        modified_params: recall_modifier.and_then(|r| r.params.clone()),
                    });
                } else if let Some(task_params) = task_params {
                    *task_modifier = TaskModifier::added(task_params);
                } else {
                    error!(
                        "Unable to retrieve task params for task {:?}, \
                        setting TaskModifier::Added to default task params in current scenario",
                        update.task.index() // TODO(@xiyuoh) use better identifier
                    );
                    *task_modifier = TaskModifier::added(TaskParams::default());
                }
            }
            UpdateTaskModifier::Hide => {
                *task_modifier = TaskModifier::Hidden;
            }
            UpdateTaskModifier::Modify(task_params) => match task_modifier {
                TaskModifier::Added(_) => {
                    *task_modifier = TaskModifier::added(task_params.clone());
                }
                TaskModifier::Inherited(inherited) => {
                    inherited.modified_params = Some(task_params.clone());
                }
                TaskModifier::Hidden => {}
            },
            UpdateTaskModifier::Reset => {
                if has_parent {
                    match task_modifier {
                        TaskModifier::Inherited(inherited) => inherited.modified_params = None,
                        _ => {}
                    }
                }
            }
        }
        if current_scenario.0.is_some_and(|e| e == update.scenario) {
            change_current_scenario.send(ChangeCurrentScenario(update.scenario));
        };
    }
}

/// Apply tasks changes to the current scenario
fn update_current_scenario_tasks(
    mut commands: Commands,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    children: Query<&Children>,
    recall_task: Query<&RecallTask>,
    scenarios: Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
    tasks: Query<Entity, With<Task>>,
    task_modifiers: Query<(&mut TaskModifier, &Affiliation<Entity>)>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        for task_entity in tasks.iter() {
            let Some((task_modifier, _)) =
                find_modifier_for_task(task_entity, *scenario_entity, &children, &task_modifiers)
                    .and_then(|modifier_entity| task_modifiers.get(modifier_entity).ok())
            else {
                continue;
            };
            if let Some(task_params) = task_modifier.params() {
                commands.entity(task_entity).insert(task_params.clone());
            } else if let Some(task_params) = retrieve_parent_params(
                task_entity,
                *scenario_entity,
                &children,
                &recall_task,
                &scenarios,
                &task_modifiers,
            ) {
                commands.entity(task_entity).insert(task_params.clone());
            }
        }
    }
}

// TODO(@xiyuoh) Generalize this system to all modifiers
/// Check for modifiers affiliated with deleted Task or missing valid affiliations.
pub fn handle_invalid_modifiers(
    mut delete: EventWriter<Delete>,
    parent: Query<&Parent>,
    scenarios: Query<(Entity, &NameInSite), With<ScenarioMarker>>,
    tasks: Query<Entity, With<Task>>,
    task_modifiers: Query<(Entity, &Affiliation<Entity>), With<TaskModifier>>,
) {
    for (modifier_entity, affiliation) in task_modifiers.iter() {
        if affiliation.0.is_some_and(|e| !tasks.get(e).is_ok()) || affiliation.0.is_none() {
            // Task modifier has no affiliated Task or is affiliated to non-existing Task
            delete.send(Delete::new(modifier_entity));
        } else if !parent
            .get(modifier_entity)
            .map(|p| p.get())
            .and_then(|e| scenarios.get(e))
            .is_ok()
        {
            // Task modifier does not have a valid parent scenario
            delete.send(Delete::new(modifier_entity));
        }
    }
}
