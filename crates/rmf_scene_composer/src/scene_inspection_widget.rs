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

use crate::*;
use bevy::prelude::*;
use bevy_egui::egui::{Align, Grid, Layout};
use librmf_site_editor::widgets::prelude::*;
use rmf_site_egui::WidgetSystem;

#[derive(Default)]
pub struct SceneInspectionPlugin {}

impl Plugin for SceneInspectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InspectionPlugin::<InspectScene>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectScene<'w, 's> {
    subscriber: SceneSubscriber<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectScene<'w, 's> {
    fn show(input: Inspect, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let Inspect { selection, .. } = input;
        let mut params = state.get_mut(world);
        let Some(scene) = params.subscriber.get_subscription(selection) else {
            return;
        };

        let mut topic = scene.topic_name().to_owned();
        let mut service = scene.service_name().to_owned();
        let mut prefixes = scene.prefixes().clone();
        Grid::new("inspect_scene_subscription")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Scene Topic");
                ui.text_edit_singleline(&mut topic);
                ui.end_row();

                ui.label("Resource Service");
                ui.text_edit_singleline(&mut service);
                ui.end_row();

                ui.label("Remove Prefixes");
                let mut remove_prefixes = Vec::new();
                let mut id: usize = 0;
                for prefix in prefixes.iter_mut() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("❌").clicked() {
                            remove_prefixes.push(id.clone());
                        }
                        ui.text_edit_singleline(prefix);
                    });
                    id += 1;
                    ui.end_row();
                    ui.label("");
                }
                ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                    if ui.button("Add").clicked() {
                        prefixes.push(String::new());
                    }
                });
                for i in remove_prefixes.drain(..).rev() {
                    prefixes.remove(i);
                }
                ui.end_row();
            });

        if topic != scene.topic_name()
            || service != scene.service_name()
            || prefixes != *scene.prefixes()
        {
            params
                .subscriber
                .change_subscription(selection, topic, service, prefixes);
        }
    }
}
