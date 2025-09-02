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
        RobotTaskRequest, ScenarioModifiers, SiteUpdateSet, Task, TaskKinds, TaskParams,
        TaskRequest, UpdateModifier,
    },
    AppState, CurrentWorkspace, Icons,
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_egui::{
    egui::{
        Align, Align2, CollapsingHeader, Color32, ComboBox, DragValue, Frame, Grid, ImageButton,
        Layout, RichText, ScrollArea, Stroke, TextEdit, Ui, Window,
    },
    EguiContexts,
};
use rmf_site_egui::*;
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
            .init_resource::<CreateTaskDialog>()
            .add_event::<EditModeEvent>()
            .add_systems(Update, show_create_task_dialog)
            .add_systems(
                PostUpdate,
                handle_task_edit
                    .run_if(AppState::in_displaying_mode())
                    .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
            );
    }
}

/// Contains a reference to the tasks widget.
#[derive(Resource)]
pub struct TaskWidget {
    pub id: Entity,
    pub show: bool,
}

impl TaskWidget {
    pub fn get(&self) -> Entity {
        self.id
    }
}

#[derive(Resource, Default)]
pub struct CreateTaskDialog {
    pub visible: bool,
}

impl FromWorld for TaskWidget {
    fn from_world(world: &mut World) -> Self {
        let panel_widget = PanelWidget::new(tasks_panel, world);
        let panel_id = world.spawn((panel_widget, PanelSide::Left)).id();

        let main_task_widget = Widget::new::<ViewTasks>(world);
        let id = world.spawn(main_task_widget).insert(ChildOf(panel_id)).id();

        Self { id, show: false }
    }
}

fn tasks_panel(In(PanelWidgetInput { id, context }): In<PanelWidgetInput>, world: &mut World) {
    let correct_state = world
        .get_resource::<State<AppState>>()
        .is_some_and(|state| matches!(state.get(), AppState::SiteEditor));

    if !world.resource::<TaskWidget>().show || !correct_state {
        return;
    }

    show_panel_of_tiles(In(PanelWidgetInput { id, context }), world);
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
    robots: Query<'w, 's, (Entity, &'static NameInSite, &'static Robot), Without<Group>>,
    scenarios: Query<
        'w,
        's,
        (
            Entity,
            &'static ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
    >,
    task_kinds: ResMut<'w, TaskKinds>,
    task_widget: ResMut<'w, TaskWidget>,
    tasks: Query<'w, 's, (Entity, &'static Task), Without<Pending>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewTasks<'w, 's> {
    fn show(
        Tile { id, panel }: Tile,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        ui.label(RichText::new("Tasks").size(18.0));
        ui.add_space(10.0);
        let params = state.get_mut(world);

        let Some(current_scenario_entity) = params.current_scenario.0 else {
            ui.label("No scenario selected, unable to display or create tasks.");
            return;
        };
        // Tasks are sorted by start time, then request time, then created time,
        // depending on which fields are populated
        let mut tasks = Vec::<(i32, (Entity, Task))>::new();
        let mut tasks_without_time = Vec::<(i32, (Entity, Task))>::new();

        for (e, task) in params.tasks.iter() {
            if let Some(params_modifier) =
                params.get_params_modifier.get(current_scenario_entity, e)
            {
                if let Some(start_time) = params_modifier.start_time() {
                    tasks.push((start_time, (e, task.clone())));
                    continue;
                }
                if let Some(request_time) = params_modifier.request_time() {
                    tasks.push((request_time, (e, task.clone())));
                    continue;
                }
            }
            if let Some(created_time) = task.request().created_time() {
                tasks.push((created_time, (e, task.clone())));
                continue;
            }
            // We should not reach here as created_time is populated by default,
            // but in case it comes up as None we sort these by entity index and
            // place them at the end of the task list
            tasks_without_time.push((e.index() as i32, (e, task.clone())));
        }
        tasks.sort_by(|a, b| a.0.cmp(&b.0));
        tasks_without_time.sort_by(|a, b| a.0.cmp(&b.0));
        tasks.extend(tasks_without_time);

        // View and modify tasks in current scenario
        Frame::default()
            .inner_margin(4.0)
            .corner_radius(2.0)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if tasks.is_empty() {
                    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                        ui.label("No tasks in this scenario");
                    });
                } else {
                    let max_height = ui.available_height() / 2.0;
                    ScrollArea::new([true, true])
                        .max_height(max_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (_, (task_entity, task)) in tasks.iter() {
                                show_task_widget(
                                    ui,
                                    Tile { id, panel },
                                    world,
                                    state,
                                    current_scenario_entity,
                                    *task_entity,
                                    task,
                                );
                            }
                        });
                }
            });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        let mut params = state.get_mut(world);
        if params.edit_task.0.is_none() {
            if ui.button("✚ Create New Task").clicked() {
                let new_task = params
                    .commands
                    .spawn(Task::default())
                    .insert(Category::Task)
                    .insert(TaskParams::default())
                    .insert(Inclusion::Included) // New tasks created are included by default
                    .insert(Pending)
                    .id();
                params.edit_mode.write(EditModeEvent {
                    scenario: current_scenario_entity,
                    mode: EditMode::New(new_task),
                });
            }
        }
        ui.add_space(10.0);
    }
}

fn show_task_widget(
    ui: &mut Ui,
    Tile { id, panel }: Tile,
    world: &mut World,
    state: &mut SystemState<ViewTasks>,
    scenario: Entity,
    task_entity: Entity,
    task: &Task,
) {
    let params = state.get_mut(world);
    let present = params
        .get_inclusion_modifier
        .get(scenario, task_entity)
        .map(|i_modifier| **i_modifier == Inclusion::Included)
        .unwrap_or(false);
    let color = if present {
        Color32::DARK_GRAY
    } else {
        Color32::default()
    };

    Frame::default()
        .inner_margin(4.0)
        .fill(color)
        .corner_radius(2.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let mut params = state.get_mut(world);

            let in_edit_mode = params.edit_task.0.is_some_and(|e| e == task_entity);
            let scenario_count = count_scenarios_with_inclusion(
                &params.scenarios,
                task_entity,
                &params.get_inclusion_modifier,
            );
            show_task_params(
                ui,
                &mut params.commands,
                task_entity,
                task,
                scenario,
                &params.get_inclusion_modifier,
                &params.get_params_modifier,
                &mut params.change_task,
                &mut params.delete,
                &mut params.edit_mode,
                &mut params.task_kinds,
                &params.robots,
                scenario_count,
                &params.icons,
                present,
                in_edit_mode,
            );

            // Display children widgets if editing existing task
            if in_edit_mode {
                ui.separator();
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
}

/// Displays the task data and params, and allows users to enter edit mode to modify values
fn show_task_params(
    ui: &mut Ui,
    commands: &mut Commands,
    task_entity: Entity,
    task: &Task,
    scenario: Entity,
    get_inclusion_modifier: &GetModifier<Modifier<Inclusion>>,
    get_params_modifier: &GetModifier<Modifier<TaskParams>>,
    change_task: &mut EventWriter<Change<Task>>,
    delete: &mut EventWriter<Delete>,
    edit_mode: &mut EventWriter<EditModeEvent>,
    task_kinds: &ResMut<TaskKinds>,
    robots: &Query<(Entity, &NameInSite, &Robot), Without<Group>>,
    scenario_count: i32,
    icons: &Res<Icons>,
    present: bool,
    in_edit_mode: bool,
) {
    ui.horizontal(|ui| {
        ui.label("Task ".to_owned() + &task_entity.index().to_string()) // TODO(@xiyuoh) better identifier
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
                        commands.entity(task_entity).insert(Inclusion::Included);
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
                            commands
                                .trigger(UpdateModifier::<Inclusion>::reset(scenario, task_entity));
                        } else {
                            // Otherwise, toggle to Hidden
                            commands.entity(task_entity).insert(Inclusion::Hidden);
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
                    commands.entity(task_entity).insert(Inclusion::Hidden);
                }
            }

            if !in_edit_mode {
                if present {
                    // Do not allow edit if not in current scenario
                    if ui
                        .add(ImageButton::new(icons.edit.egui()))
                        .on_hover_text("Edit task parameters")
                        .clicked()
                    {
                        edit_mode.write(EditModeEvent {
                            scenario: scenario,
                            mode: EditMode::Edit(Some(task_entity)),
                        });
                    }
                }
            } else {
                // Exit edit mode
                if ui
                    .add(ImageButton::new(icons.confirm.egui()))
                    .on_hover_text("Exit edit mode")
                    .clicked()
                {
                    edit_mode.write(EditModeEvent {
                        scenario: scenario,
                        mode: EditMode::Edit(None),
                    });
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

    show_editable_task(
        ui,
        commands,
        task_entity,
        task,
        &task_params,
        scenario,
        in_edit_mode,
        get_params_modifier,
        robots,
        task_kinds,
        change_task,
    );
}

fn show_editable_task(
    ui: &mut Ui,
    commands: &mut Commands,
    task_entity: Entity,
    task: &Task,
    task_params: &TaskParams,
    scenario: Entity,
    in_edit_mode: bool,
    get_params_modifier: &GetModifier<Modifier<TaskParams>>,
    robots: &Query<(Entity, &NameInSite, &Robot), Without<Group>>,
    task_kinds: &ResMut<TaskKinds>,
    change_task: &mut EventWriter<Change<Task>>,
) {
    let mut new_task = task.clone();
    let task_request = new_task.request();
    Grid::new("show_editable_task_".to_owned() + &task_entity.index().to_string())
        .num_columns(2)
        .show(ui, |ui| {
            // Request Type
            ui.label("Request Type:");
            if !in_edit_mode {
                match task {
                    Task::Dispatch(_) => {
                        ui.horizontal(|ui| {
                            let _ = ui.selectable_label(true, "Dispatch");
                            ui.label(
                                task_request
                                    .fleet_name()
                                    .unwrap_or("Unassigned".to_string()),
                            );
                        });
                    }
                    Task::Direct(_) => {
                        ui.horizontal(|ui| {
                            let _ = ui.selectable_label(true, "Direct");
                            ui.label(task.fleet().to_owned() + "/" + &task.robot());
                        });
                    }
                }
            } else {
                edit_request_type_widget(
                    ui,
                    &mut new_task,
                    &task_request,
                    robots,
                    task.robot(),
                    task.fleet(),
                );
            }
            ui.end_row();

            // Task Kind
            ui.label("Task Kind:");
            if !in_edit_mode {
                ui.label(task_request.category());
            } else {
                edit_task_kind_widget(ui, commands, &mut new_task, task_entity, task_kinds);
            }
            ui.end_row();

            // Task Description
            // Only displayed when not editing; the TaskKind widget will appear when editing
            if !in_edit_mode {
                ui.label("Description:");
                ui.label(
                    task_request
                        .description_display()
                        .unwrap_or("None".to_string()),
                );
                ui.end_row();
            }

            // Requester
            ui.label("Requester:")
                .on_hover_text("(Optional) An identifier for the entity that requested this task");
            if !in_edit_mode {
                ui.label(task_request.requester().unwrap_or("None".to_string()));
            } else {
                edit_requester_widget(ui, &mut new_task);
            }
            ui.end_row();
        });

    // More
    let mut new_task_params = task_params.clone();
    CollapsingHeader::new("More details")
        .id_salt("task_details_".to_owned() + &task_entity.index().to_string())
        .default_open(false)
        .show(ui, |ui| {
            // Fleet name
            if task.is_dispatch() {
                ui.horizontal(|ui| {
                    ui.label("Fleet:").on_hover_text(
                        "(Optional) The name of the fleet for this robot. \
                            If specified, other fleets will not bid for this task.",
                    );
                    if !in_edit_mode {
                        ui.label(task_request.fleet_name().unwrap_or("None".to_string()));
                    } else {
                        edit_fleet_widget(ui, &mut new_task);
                    }
                });
            }

            // Start time
            // TODO(@xiyuoh) Add status/queued information
            ui.horizontal(|ui| {
                ui.label("Start time:")
                    .on_hover_text("(Optional) The earliest time that this task may start");
                if !in_edit_mode {
                    ui.label(
                        task_params
                            .start_time()
                            .map(|st| format!("{:?}", st))
                            .unwrap_or("None".to_string()),
                    );
                } else {
                    edit_start_time_widget(ui, &mut new_task_params);
                }
            });

            // Request time
            ui.horizontal(|ui| {
                ui.label("Request time:")
                    .on_hover_text("(Optional) The time that this request was initiated");
                if !in_edit_mode {
                    ui.label(
                        task_params
                            .request_time()
                            .map(|rt| format!("{:?}", rt))
                            .unwrap_or("None".to_string()),
                    );
                } else {
                    edit_request_time_widget(ui, &mut new_task_params);
                }
            });

            // Priority
            ui.horizontal(|ui| {
                ui.label("Priority:").on_hover_text(
                    "(Optional) The priority of this task. \
                        This must match a priority schema supported by a fleet.",
                );
                if !in_edit_mode {
                    ui.label(
                        task_params
                            .priority()
                            .map(|p| p.to_string())
                            .unwrap_or("None".to_string()),
                    );
                } else {
                    edit_priority_widget(ui, &mut new_task_params);
                }
            });

            // Labels
            ui.horizontal(|ui| {
                ui.label("Labels:").on_hover_text(
                    "Labels to describe the purpose of the task dispatch request, \
                        items can be a single value like `dashboard` or a key-value pair \
                        like `app=dashboard`, in the case of a single value, it will be \
                        interpreted as a key-value pair with an empty string value.",
                );
                if !in_edit_mode {
                    ui.label(format!("{:?}", task_params.labels()));
                } else {
                    edit_labels_widget(ui, &mut new_task_params);
                }
            });

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
                        .on_hover_text("Reset task parameters to parent scenario params")
                        .clicked()
                    {
                        commands
                            .trigger(UpdateModifier::<TaskParams>::reset(scenario, task_entity));
                    }
                }
            }
        });

    // Trigger appropriate events if changes have been made in edit mode
    if in_edit_mode {
        if new_task != *task {
            change_task.write(Change::new(new_task, task_entity));
        }

        if new_task_params != *task_params {
            commands.entity(task_entity).insert(new_task_params);
        }
    }
}

fn show_create_task_dialog(
    world: &mut World,
    task_state: &mut SystemState<(
        Res<CurrentScenario>,
        Res<EditTask>,
        Query<(&Task, &TaskParams), With<Pending>>,
    )>,
    egui_context_state: &mut SystemState<EguiContexts>,
    edit_state: &mut SystemState<(
        Commands,
        GetModifier<Modifier<TaskParams>>,
        Query<(Entity, &NameInSite, &Robot), Without<Group>>,
        ResMut<TaskKinds>,
        EventWriter<Change<Task>>,
    )>,
    widget_state: &mut SystemState<(Query<&Children>, Res<TaskWidget>)>,
    dialog_state: &mut SystemState<(ResMut<CreateTaskDialog>, EventWriter<EditModeEvent>)>,
) {
    let (create_task_dialog, _) = dialog_state.get_mut(world);
    if !create_task_dialog.visible {
        return;
    }

    let (current_scenario, edit_task, pending_tasks) = task_state.get_mut(world);
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };
    let Some(task_entity) = edit_task.0 else {
        return;
    };
    let Ok((pending_task, pending_task_params)) = pending_tasks.get(task_entity) else {
        return;
    };
    let (pending_task, pending_task_params) = (pending_task.clone(), pending_task_params.clone());

    let mut egui_context = egui_context_state.get_mut(world);
    let mut ctx = egui_context.ctx_mut().clone();
    Window::new("Creating New Task")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(&mut ctx, |ui| {
            let (mut commands, get_params_modifier, robots, task_kinds, mut change_task) =
                edit_state.get_mut(world);
            show_editable_task(
                ui,
                &mut commands,
                task_entity,
                &pending_task,
                &pending_task_params,
                current_scenario_entity,
                true,
                &get_params_modifier,
                &robots,
                &task_kinds,
                &mut change_task,
            );
            let task_request_category = pending_task.request().category();
            let task_kind_is_valid =
                if let Some((_, _, valid_fn)) = task_kinds.0.get(&task_request_category) {
                    Some(valid_fn.clone())
                } else {
                    None
                };
            edit_state.apply(world);
            ui.separator();

            let (children, task_widget) = widget_state.get_mut(world);
            let children: Result<SmallVec<[_; 16]>, _> = children
                .get(task_widget.id)
                .map(|children| children.iter().collect());
            let Ok(children) = children else {
                return;
            };

            let widget_entity = task_widget.id;
            for child in children {
                let tile = Tile {
                    id: widget_entity,
                    panel: PanelSide::Top, // Any panel will do
                };
                let _ = world.try_show_in(child, tile, ui);
            }

            let mut reset_edit: bool = false;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    reset_edit = true;
                }
                let task_is_valid = if let Some(task_kind_is_valid) = task_kind_is_valid {
                    pending_task.is_valid() && task_kind_is_valid(task_entity, world)
                } else {
                    // If task kind valid fn cannot be retrieved, task is invalid
                    false
                };
                ui.add_enabled_ui(task_is_valid, |ui| {
                    if ui
                        .button("Add Task")
                        .on_hover_text("Add this task to the current scenario")
                        .clicked()
                    {
                        world.entity_mut(task_entity).remove::<Pending>();
                        reset_edit = true;
                    }
                });
            });

            if reset_edit {
                let (mut create_task_dialog, mut edit_mode) = dialog_state.get_mut(world);
                edit_mode.write(EditModeEvent {
                    scenario: current_scenario_entity,
                    mode: EditMode::Edit(None),
                });
                create_task_dialog.visible = false;
            }
        });
}

fn edit_request_type_widget(
    ui: &mut Ui,
    task: &mut Task,
    task_request: &TaskRequest,
    robots: &Query<(Entity, &NameInSite, &Robot), Without<Group>>,
    robot: String,
    fleet: String,
) {
    let mut is_robot_task_request = task.is_direct();
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
    // Update Request Type and show RobotTaskRequest widget
    if is_robot_task_request {
        if !task.is_direct() {
            *task = Task::Direct(RobotTaskRequest::new(robot, fleet, task_request.clone()));
        }
        if let Task::Direct(ref mut robot_task_request) = task {
            ui.end_row();

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
                    for (_, robot_name, robot) in robots.iter() {
                        ui.selectable_value(
                            robot_task_request.robot_mut(),
                            robot_name.0.clone(),
                            robot_name.0.clone(),
                        );
                        // Update fleet according to selected robot
                        if robot_task_request.robot() == robot_name.0 {
                            *robot_task_request.fleet_mut() = robot.fleet.clone();
                        }
                    }
                });
        } else {
            warn!("Unable to select Direct task!");
        }
    } else {
        if !task.is_dispatch() {
            *task = Task::Dispatch(DispatchTaskRequest::new(task_request.clone()));
        }
    }
}

fn edit_task_kind_widget(
    ui: &mut Ui,
    commands: &mut Commands,
    task: &mut Task,
    task_entity: Entity,
    task_kinds: &ResMut<TaskKinds>,
) {
    let current_category = task.request().category();
    let selected_task_kind = if task_kinds.0.contains_key(&current_category) {
        current_category.clone()
    } else {
        "Select Kind".to_string()
    };
    ComboBox::from_id_salt("select_task_kind")
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
    // Insert selected TaskKind component
    let new_category = task.request().category();
    if new_category != current_category {
        if let Some(remove_fn) = task_kinds
            .0
            .get(&current_category)
            .map(|(_, rm_fn, _)| rm_fn)
        {
            remove_fn(commands.entity(task_entity));
        }
        if let Some(insert_fn) = task_kinds.0.get(&new_category).map(|(is_fn, _, _)| is_fn) {
            insert_fn(commands.entity(task_entity));
        }
    }
}

fn edit_requester_widget(ui: &mut Ui, task: &mut Task) {
    let new_task_request = task.request_mut();
    let requester = new_task_request
        .requester_mut()
        .get_or_insert(String::new());
    TextEdit::singleline(requester)
        .desired_width(ui.available_width())
        .show(ui);
    if requester.is_empty() {
        *new_task_request.requester_mut() = None;
    }
}

fn edit_fleet_widget(ui: &mut Ui, task: &mut Task) {
    // TODO(@xiyuoh) when available, insert combobox of registered fleets
    let new_task_request = task.request_mut();
    let fleet_name = new_task_request
        .fleet_name_mut()
        .get_or_insert(String::new());
    TextEdit::singleline(fleet_name)
        .desired_width(ui.available_width())
        .show(ui);
    if fleet_name.is_empty() {
        *new_task_request.fleet_name_mut() = None;
    }
}

fn edit_start_time_widget(ui: &mut Ui, task_params: &mut TaskParams) {
    let start_time = task_params.start_time();
    let mut has_start_time = start_time.is_some();
    ui.horizontal(|ui| {
        ui.checkbox(&mut has_start_time, "");
        if has_start_time {
            let new_start_time = task_params.start_time_mut().get_or_insert(0);
            ui.add(
                DragValue::new(new_start_time)
                    .range(0_i32..=std::i32::MAX)
                    .speed(1),
            );
        } else if start_time.is_some() {
            *task_params.start_time_mut() = None;
        }
    });
}

fn edit_request_time_widget(ui: &mut Ui, task_params: &mut TaskParams) {
    let request_time = task_params.request_time();
    let mut has_request_time = request_time.is_some();
    ui.horizontal(|ui| {
        ui.checkbox(&mut has_request_time, "");
        if has_request_time {
            let new_request_time = task_params.request_time_mut().get_or_insert(0);
            ui.add(
                DragValue::new(new_request_time)
                    .range(0_i32..=std::i32::MAX)
                    .speed(1),
            );
        } else if request_time.is_some() {
            *task_params.request_time_mut() = None;
        }
    });
}

fn edit_priority_widget(ui: &mut Ui, task_params: &mut TaskParams) {
    let priority = task_params.priority();
    let mut has_priority = priority.is_some();
    ui.checkbox(&mut has_priority, "");
    if has_priority {
        if priority.is_none() {
            *task_params.priority_mut() = Some(Value::Null);
        }
        // TODO(@xiyuoh) Expand on this to create fleet-specific priority widgets
    } else if priority.is_some() {
        *task_params.priority_mut() = None;
    }
}

fn edit_labels_widget(ui: &mut Ui, task_params: &mut TaskParams) {
    let mut remove_labels = Vec::new();
    let mut id: usize = 0;
    ui.vertical(|ui| {
        for label in task_params.labels_mut() {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("❌").on_hover_text("Remove label").clicked() {
                    remove_labels.push(id.clone());
                }
                TextEdit::singleline(label)
                    .desired_width(ui.available_width())
                    .show(ui);
            });
            id += 1;
        }
        ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
            if ui
                .button("Add label")
                .on_hover_text("Insert new label")
                .clicked()
            {
                task_params.labels_mut().push(String::new());
            }
        });
    });
    for i in remove_labels.drain(..).rev() {
        task_params.labels_mut().remove(i);
    }
}

/// Updates the current EditTask entity based on the triggered edit mode event
pub fn handle_task_edit(
    mut commands: Commands,
    mut create_task_dialog: ResMut<CreateTaskDialog>,
    mut delete: EventWriter<Delete>,
    mut edit_mode: EventReader<EditModeEvent>,
    mut edit_task: ResMut<EditTask>,
    pending_tasks: Query<&mut Task, With<Pending>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    // TODO(@xiyuoh) fix bug where the egui panel glitches when the EditTask resource is being accessed
    if let Some(edit) = edit_mode.read().last() {
        match edit.mode {
            EditMode::New(task_entity) => {
                if let Some(site_entity) = current_workspace.root {
                    commands.entity(task_entity).insert(ChildOf(site_entity));
                }
                edit_task.0 = Some(task_entity);
                create_task_dialog.visible = true;
            }
            EditMode::Edit(task_entity) => {
                if let Some(pending_task) = edit_task.0.filter(|e| pending_tasks.get(*e).is_ok()) {
                    delete.write(Delete::new(pending_task));
                }
                edit_task.0 = task_entity;
            }
        }
    }
}
