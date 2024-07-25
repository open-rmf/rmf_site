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
    site::{Change, ChangePlugin, Delete},
    widgets::{prelude::*, Inspect, InspectionPlugin},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::Ui;
use rmf_site_format::{
    AssetSource, DifferentialDrive, Group, ModelMarker, Pose, Scale, SimpleTask,
};

#[derive(Default)]
pub struct InspectTaskPlugin {}

impl Plugin for InspectTaskPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TaskPreview>().add_plugins((
            ChangePlugin::<SimpleTask>::default(),
            InspectionPlugin::<InspectTask>::new(),
        ));
    }
}

#[derive(SystemParam)]
pub struct InspectTask<'w, 's> {
    commands: Commands<'w, 's>,
    selection: Res<'w, Selection>,
    change_task: EventWriter<'w, Change<SimpleTask>>,
    change_pose: EventWriter<'w, Change<Pose>>,
    delete: EventWriter<'w, Delete>,
    model_instances: Query<
        'w,
        's,
        (
            Option<&'static mut SimpleTask>,
            &'static mut AssetSource,
            Option<&'static mut Scale>,
        ),
        (With<ModelMarker>, With<DifferentialDrive>, Without<Group>),
    >,
    task_preview: ResMut<'w, TaskPreview>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectTask<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(selection, ui);
    }
}

impl<'w, 's> InspectTask<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        // Reset viewing previews when selection changes
        if self.selection.is_changed() {
            if let Some(preview_entity) = self.task_preview.0 {
                self.delete.send(Delete::new(preview_entity));
                self.task_preview.0 = None;
            }
        }

        if let Ok((task, source, scale)) = self.model_instances.get(id) {
            // Create new task if it doesn't already exist
            let mut new_task = match task {
                Some(task) => task.clone(),
                None => {
                    self.commands.entity(id).insert(SimpleTask(None));
                    SimpleTask(None)
                }
            };

            ui.horizontal(|ui| {
                ui.label("Task");
                match new_task.0 {
                    Some(_) => {
                        if ui.button("Remove").clicked() {
                            new_task.0 = None;
                        }
                    }
                    None => {
                        if ui.button("Add").clicked() {
                            new_task.0 = Some(Pose::default());
                        }
                    }
                }
            });

            ui.add_enabled_ui(new_task.0.is_some(), |ui| {
                match &new_task.0 {
                    Some(pose) => {
                        if let Some(new_pose) = InspectPoseComponent::new(pose).show(ui) {
                            new_task.0 = Some(new_pose);
                        }
                    }
                    None => {
                        InspectPoseComponent::new(&Pose::default()).show(ui);
                    }
                }

                if new_task.0.is_none() {
                    if let Some(preview_entity) = self.task_preview.0 {
                        self.delete.send(Delete::new(preview_entity));
                        self.task_preview.0 = None;
                    }
                }
                if ui
                    .selectable_label(self.task_preview.0.is_some(), "Preview")
                    .clicked()
                {
                    match self.task_preview.0 {
                        Some(preview_entity) => {
                            self.delete.send(Delete::new(preview_entity));
                            self.task_preview.0 = None;
                        }
                        None => {
                            let task_preview_bundle = TaskPreviewBundle {
                                pose: new_task.0.unwrap_or_default(),
                                scale: scale.cloned().unwrap_or_default(),
                                source: source.clone(),
                                marker: ModelMarker,
                            };
                            let task_preview_entity = self.commands.spawn(task_preview_bundle).id();
                            self.task_preview.0 = Some(task_preview_entity);
                        }
                    }
                }
            });

            if task.is_some_and(|task| task != &new_task) {
                println!("Task: {:?}", new_task);
                self.change_task.send(Change::new(new_task.clone(), id));
            }
            if task.is_some_and(|task| task.0 != new_task.0) {
                if let Some(task_preview_entity) = self.task_preview.0 {
                    if let Some(new_pose) = new_task.0 {
                        self.change_pose
                            .send(Change::new(new_pose, task_preview_entity));
                    }
                }
            }
        }
    }
}

#[derive(Bundle, Default)]
pub struct TaskPreviewBundle {
    pub pose: Pose,
    pub scale: Scale,
    pub source: AssetSource,
    pub marker: ModelMarker,
}

#[derive(Resource, Clone, Default)]
pub struct TaskPreview(Option<Entity>);
