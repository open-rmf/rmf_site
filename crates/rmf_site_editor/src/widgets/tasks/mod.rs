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
        count_scenarios_with_inclusion, Affiliation, Category, Change, CurrentScenario, Delete,
        DispatchTaskRequest, GetModifier, Group, Inclusion, Modifier, NameInSite, Pending, Robot,
        RobotTaskRequest, ScenarioMarker, ScenarioModifiers, Task, TaskKinds, TaskParams,
        UpdateModifier, UpdateTaskModifier,
    },
    widgets::prelude::*,
    Icons, 
};
use bevy::{
    ecs::{
        hierarchy::ChildOf,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use bevy_egui::egui::{
    Align, CollapsingHeader, Color32, ComboBox, DragValue, Frame, Grid, ImageButton, Layout,
    Stroke, TextEdit, Ui,
};
use rmf_site_ui::*;
use serde_json::Value;
use smallvec::SmallVec;

pub mod go_to_place;
pub use go_to_place::*;

pub mod wait_for;
pub use wait_for::*;

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
            .add_event::<EditModeEvent>();
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
        let id = world.spawn(widget).insert(ChildOf(properties_panel)).id();
        Self { id }
    }
}

/// Points to any task entity that is currently in edit mode
#[derive(Resource, Deref, DerefMut)]
pub struct EditTask(pub Option<Entity>);

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

#[derive(SystemParam)]
pub struct ViewTasks<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    commands: Commands<'w, 's>,
    change_task: EventWriter<'w, Change<Task>>,
    current_scenario: ResMut<'w, CurrentScenario>,
    delete: EventWriter<'w, Delete>,
    edit_mode: EventWriter<'w, EditModeEvent>,
    edit_task: Res<'w, EditTask>,
    get_inclusion_modifier: GetModifier<'w, 's, Modifier<Inclusion>>,
    get_params_modifier: GetModifier<'w, 's, Modifier<TaskParams>>,
    icons: Res<'w, Icons>,
    pending_tasks: Query<'w, 's, (Entity, &'static Task, &'static TaskParams), With<Pending>>,
    robots: Query<'w, 's, (Entity, &'static NameInSite), (With<Robot>, Without<Group>)>,
    scenarios: Query<
        'w,
        's,
        (
            Entity,
            &'static ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
        With<ScenarioMarker>,
    >,
    task_kinds: ResMut<'w, TaskKinds>,
    task_widget: ResMut<'w, TaskWidget>,
    tasks: Query<'w, 's, (Entity, &'static Task), Without<Pending>>,
    update_task_modifier: EventWriter<'w, UpdateModifier<UpdateTaskModifier>>,
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
                        .map(|children| children.iter().collect());
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
            .corner_radius(2.0)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for (task_entity, task) in self.tasks.iter() {
                    let scenario_count = count_scenarios_with_inclusion(
                        &self.scenarios,
                        task_entity,
                        &self.get_inclusion_modifier,
                    );
                    if show_task(
                        ui,
                        task_entity,
                        task,
                        current_scenario_entity,
                        &self.get_inclusion_modifier,
                        &self.get_params_modifier,
                        &mut self.update_task_modifier,
                        &mut self.delete,
                        scenario_count,
                        &self.icons,
                    ) {
                        self.edit_mode.write(EditModeEvent {
                            scenario: current_scenario_entity,
                            mode: EditMode::Edit(Some(task_entity)),
                        });
                    }
                }
                if self.tasks.is_empty() {
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
                    &pending_task_params,
                    &self.task_kinds,
                    &self.robots,
                    &mut self.change_task,
                    &mut self.update_task_modifier,
                );
            } else {
                if let Ok((_, existing_task)) = self.tasks.get_mut(task_entity) {
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
                    let Some(existing_task_params) = self
                        .get_params_modifier
                        .get(current_scenario_entity, task_entity)
                        .map(|modifier| (**modifier).clone())
                    else {
                        return;
                    };
                    ui.separator();
                    edit_task(
                        ui,
                        &mut self.commands,
                        current_scenario_entity,
                        task_entity,
                        existing_task,
                        &existing_task_params,
                        &self.task_kinds,
                        &self.robots,
                        &mut self.change_task,
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
                self.edit_mode.write(EditModeEvent {
                    scenario: current_scenario_entity,
                    mode: EditMode::New(new_task),
                });
            }
        }

        if reset_edit {
            self.edit_mode.write(EditModeEvent {
                scenario: current_scenario_entity,
                mode: EditMode::Edit(None),
            });
        }
    }
}

/// Displays the task data and params and returns a boolean indicating if the user
/// wishes to edit the task.
fn show_task(
    ui: &mut Ui,
    task_entity: Entity,
    task: &Task,
    scenario: Entity,
    get_inclusion_modifier: &GetModifier<Modifier<Inclusion>>,
    get_params_modifier: &GetModifier<Modifier<TaskParams>>,
    update_task_modifier: &mut EventWriter<UpdateModifier<UpdateTaskModifier>>,
    delete: &mut EventWriter<Delete>,
    scenario_count: i32,
    icons: &Res<Icons>,
) -> bool {
    let present = get_inclusion_modifier
        .get(scenario, task_entity)
        .map(|i_modifier| **i_modifier == Inclusion::Included)
        .unwrap_or(false);
    let color = if present {
        Color32::DARK_GRAY
    } else {
        Color32::default()
    };
    let mut edit = false;
    Frame::default()
        .inner_margin(4.0)
        .fill(color)
        .corner_radius(2.0)
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
                        delete.write(Delete::new(task_entity));
                    }
                    // Include/hide task
                    // Toggle between 3 inclusion modes: Included -> None (inherit from parent) -> Hidden
                    // If this is a root scenario, we won't include the None option
                    let inclusion_modifier = get_inclusion_modifier
                        .scenarios
                        .get(scenario)
                        .ok()
                        .and_then(|(scenario_modifiers, _)| scenario_modifiers.get(&task_entity))
                        .and_then(|e| get_inclusion_modifier.modifiers.get(*e).ok());
                    if let Some(inclusion_modifier) = inclusion_modifier {
                        // Either explicitly included or hidden
                        if **inclusion_modifier == Inclusion::Hidden {
                            if ui
                                .add(ImageButton::new(icons.hide.egui()))
                                .on_hover_text("Task is hidden in this scenario")
                                .clicked()
                            {
                                update_task_modifier.write(UpdateModifier::new(
                                    scenario,
                                    task_entity,
                                    UpdateTaskModifier::Include,
                                ));
                            }
                        } else {
                            if ui
                                .add(ImageButton::new(icons.show.egui()))
                                .on_hover_text("Task is included in this scenario")
                                .clicked()
                            {
                                if get_inclusion_modifier
                                    .scenarios
                                    .get(scenario)
                                    .is_ok_and(|(_, a)| a.0.is_some())
                                {
                                    // If parent scenario exists, clicking this button toggles to ResetInclusion
                                    update_task_modifier.write(UpdateModifier::new(
                                        scenario,
                                        task_entity,
                                        UpdateTaskModifier::ResetInclusion,
                                    ));
                                } else {
                                    // Otherwise, toggle to Hidden
                                    update_task_modifier.write(UpdateModifier::new(
                                        scenario,
                                        task_entity,
                                        UpdateTaskModifier::Hide,
                                    ));
                                }
                            }
                        }
                    } else {
                        // Modifier is inherited
                        if ui
                            .add(ImageButton::new(icons.link.egui()))
                            .on_hover_text("Task inclusion is inherited in this scenario")
                            .clicked()
                        {
                            update_task_modifier.write(UpdateModifier::new(
                                scenario,
                                task_entity,
                                UpdateTaskModifier::Hide,
                            ));
                        }
                    }
                    if present {
                        // Do not allow edit if not in current scenario
                        if ui
                            .add(ImageButton::new(icons.edit.egui()))
                            .on_hover_text("Edit task parameters")
                            .clicked()
                        {
                            edit = true;
                        }
                    }
                });
            });
            if !present {
                return;
            }
            ui.separator();

            let Some(task_params) = get_params_modifier
                .get(scenario, task_entity)
                .map(|m| (**m).clone())
            else {
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
                .id_salt("task_details_".to_owned() + &task_entity.index().to_string())
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

                            // Reset task parameters to parent scenario params (if any)
                            if let Ok((scenario_modifiers, parent_scenario)) =
                                get_params_modifier.scenarios.get(scenario)
                            {
                                if scenario_modifiers
                                    .get(&task_entity)
                                    .is_some_and(|e| get_params_modifier.modifiers.get(*e).is_ok())
                                    && parent_scenario.0.is_some()
                                {
                                    // Only display reset button if this task has a TaskParams modifier
                                    // and this is not a root scenario
                                    if ui
                                        .button("Reset Task Params")
                                        .on_hover_text(
                                            "Reset task parameters to parent scenario params",
                                        )
                                        .clicked()
                                    {
                                        update_task_modifier.write(UpdateModifier::new(
                                            scenario,
                                            task_entity,
                                            UpdateTaskModifier::ResetParams,
                                        ));
                                    }
                                    ui.end_row();
                                }
                            }
                        });
                });
        });
    edit
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
    change_task: &mut EventWriter<Change<Task>>,
    update_task_modifier: &mut EventWriter<UpdateModifier<UpdateTaskModifier>>,
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
                    ComboBox::from_id_salt("select_robot_for_task")
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
            ComboBox::from_id_salt("select_task_kind")
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
                change_task.write(Change::new(new_task, task_entity));
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
                                    .range(0_i32..=std::i32::MAX)
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
                                    .range(0_i32..=std::i32::MAX)
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
                        update_task_modifier.write(UpdateModifier::new(
                            scenario,
                            task_entity,
                            UpdateTaskModifier::Modify(new_task_params),
                        ));
                    }
                });
        });
}
