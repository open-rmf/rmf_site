/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
    inspector::InspectPoseComponent,
    interaction::Selection,
    site::{
        update_model_instances, Affiliation, AssetSource, Change, ChangePlugin, Delete,
        DifferentialDrive, Group, MobileRobotMarker, ModelMarker, ModelProperty, Pose, Scale,
        SiteParent, Task, Tasks,
    },
    widgets::{prelude::*, Inspect, InspectionPlugin},
    Icons, ModelPropertyData,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Align, Button, Color32, ComboBox, DragValue, Frame, Layout, Stroke, Ui};
use std::any::TypeId;

use super::InspectPose;

#[derive(Default)]
pub struct InspectTaskPlugin {}

impl Plugin for InspectTaskPlugin {
    fn build(&self, app: &mut App) {
        // Allows us to toggle MobileRobotMarker as a configurable property
        // from the model description inspector
        app.register_type::<ModelProperty<MobileRobotMarker>>()
            .add_plugins(ChangePlugin::<ModelProperty<MobileRobotMarker>>::default())
            .add_systems(
                PreUpdate,
                (
                    add_remove_mobile_robot_tasks,
                    update_model_instances::<MobileRobotMarker>,
                ),
            )
            .init_resource::<ModelPropertyData>()
            .world
            .resource_mut::<ModelPropertyData>()
            .optional
            .insert(
                TypeId::of::<ModelProperty<MobileRobotMarker>>(),
                (
                    "Mobile Robot".to_string(),
                    |mut e_cmd| {
                        e_cmd.insert(ModelProperty::<MobileRobotMarker>::default());
                    },
                    |mut e_cmd| {
                        e_cmd.remove::<ModelProperty<MobileRobotMarker>>();
                    },
                ),
            );

        // Ui
        app.init_resource::<PendingEditTask>().add_plugins((
            ChangePlugin::<Tasks<Entity>>::default(),
            InspectionPlugin::<InspectTasks>::new(),
        ));
    }
}

#[derive(SystemParam)]
pub struct InspectTasks<'w, 's> {
    commands: Commands<'w, 's>,
    selection: Res<'w, Selection>,
    change_tasks: EventWriter<'w, Change<Tasks<Entity>>>,
    mobile_robots:
        Query<'w, 's, &'static mut Tasks<Entity>, (With<MobileRobotMarker>, Without<Group>)>,
    pending_task: ResMut<'w, PendingEditTask>,
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
        let Ok(mut tasks) = params.mobile_robots.get_mut(selection) else {
            return;
        };

        ui.label("Tasks");

        Frame::default()
            .inner_margin(4.0)
            .rounding(2.0)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                if tasks.0.is_empty() {
                    ui.label("No Tasks");
                } else {
                    for task in tasks.0.iter_mut() {
                        edit_task_component(ui, &mut PendingEditTask::from_task(task), true);
                    }
                }
            });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            // Only allow adding as task if valid
            ui.add_enabled_ui(params.pending_task.to_task().is_some(), |ui| {
                if ui
                    .add(Button::image_and_text(params.icons.add.egui(), "New"))
                    .clicked()
                {
                    tasks.0.push(params.pending_task.to_task().unwrap());
                }
            });
            // Select new task type
            ComboBox::from_id_source("pending_edit_task")
                .selected_text(params.pending_task.label())
                .show_ui(ui, |ui| {
                    for label in PendingEditTask::labels() {
                        if ui
                            .selectable_label(
                                label == params.pending_task.label(),
                                label.to_string(),
                            )
                            .clicked()
                        {
                            *params.pending_task = PendingEditTask::from_label(label);
                        }
                    }
                });
        });

        ui.add_space(10.0);
        edit_task_component(ui, &mut params.pending_task, false);
    }
}

/// Returns true if delete
fn edit_task_component(ui: &mut Ui, task: &mut PendingEditTask, with_delete: bool) {
    Frame::default()
        .inner_margin(4.0)
        .fill(Color32::DARK_GRAY)
        .rounding(2.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Header with selection button / simple data
            ui.horizontal(|ui| {
                ui.label(task.label());

                match task {
                    PendingEditTask::GoToPlace(pose, site_parent) => {
                        ui.selectable_label(true, "Select Goal");
                    }
                    PendingEditTask::WaitFor(duration) => {
                        ui.add(
                            DragValue::new(duration)
                                .clamp_range(0_f32..=std::f32::INFINITY)
                                .speed(0.01),
                        );
                        ui.label("s");
                    }
                    PendingEditTask::PickUp(_) => {
                        ui.selectable_label(true, "Model #129");
                    }
                    PendingEditTask::DropOff((_, _)) => {
                        ui.selectable_label(true, "Model #29");
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if with_delete {
                        if ui.button("❌").on_hover_text("Delete task").clicked() {
                            *task = PendingEditTask::default();
                        }
                    }
                });
            });
        });
}

#[derive(Resource)]
pub enum PendingEditTask {
    GoToPlace(Option<Pose>, SiteParent<Entity>),
    WaitFor(f32),
    PickUp(Option<Affiliation<Entity>>),
    DropOff((Option<Affiliation<Entity>>, Option<Pose>)),
}

impl Default for PendingEditTask {
    fn default() -> Self {
        Self::WaitFor(0.0)
    }
}

impl PendingEditTask {
    fn from_task(task: &Task<Entity>) -> Self {
        match task {
            Task::GoToPlace(pose, parent) => PendingEditTask::GoToPlace(Some(*pose), *parent),
            Task::WaitFor(time) => PendingEditTask::WaitFor(*time),
            Task::PickUp(affiliation) => PendingEditTask::PickUp(Some(*affiliation)),
            Task::DropOff((affiliation, pose)) => {
                PendingEditTask::DropOff((Some(*affiliation), Some(*pose)))
            }
        }
    }

    fn to_task(&self) -> Option<Task<Entity>> {
        match self {
            PendingEditTask::GoToPlace(Some(pose), parent) => {
                Some(Task::GoToPlace(pose.clone(), parent.clone()))
            }
            PendingEditTask::WaitFor(time) => Some(Task::WaitFor(*time)),
            PendingEditTask::PickUp(Some(affiliation)) => Some(Task::PickUp(affiliation.clone())),
            PendingEditTask::DropOff((Some(affiliation), Some(pose))) => {
                Some(Task::DropOff((affiliation.clone(), pose.clone())))
            }
            _ => None,
        }
    }

    fn labels() -> Vec<&'static str> {
        vec!["Go To Place", "Wait For", "Pick Up", "Drop Off"]
    }

    fn label(&self) -> &str {
        match self {
            PendingEditTask::GoToPlace(_, _) => Self::labels()[0],
            PendingEditTask::WaitFor(_) => Self::labels()[1],
            PendingEditTask::PickUp(_) => Self::labels()[2],
            PendingEditTask::DropOff(_) => Self::labels()[3],
        }
    }

    fn from_label(label: &str) -> Self {
        let labels = Self::labels();
        match labels.iter().position(|&l| l == label) {
            Some(0) => PendingEditTask::GoToPlace(None, SiteParent::default()),
            Some(1) => PendingEditTask::WaitFor(0.0),
            Some(2) => PendingEditTask::PickUp(None),
            Some(3) => PendingEditTask::DropOff((None, None)),
            _ => PendingEditTask::WaitFor(0.0),
        }
    }
}

/// When the MobileRobotMarker is added or removed, add or remove the Tasks<Entity> component
fn add_remove_mobile_robot_tasks(
    mut commands: Commands,
    instances: Query<(Entity, Ref<MobileRobotMarker>), Without<Group>>,
    mut removals: RemovedComponents<ModelProperty<MobileRobotMarker>>,
) {
    for removal in removals.read() {
        if instances.get(removal).is_ok() {
            commands.entity(removal).remove::<Tasks<Entity>>();
        }
    }

    for (e, marker) in instances.iter() {
        if marker.is_added() {
            commands.entity(e).insert(Tasks::<Entity>::default());
        }
    }
}
