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

use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::egui::{Align, Grid, Layout};

use librmf_site_editor::{widgets::prelude::*, AppState};
use rmf_site_egui::{HeaderTilePlugin, Tile, WidgetSystem};

use crate::*;

#[derive(Default)]
pub struct SceneCreationPlugin {}

impl Plugin for SceneCreationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingSubscription>()
            .add_plugins(HeaderTilePlugin::<SceneCreationWidget>::new());
    }
}

#[derive(SystemParam)]
struct SceneCreationWidget<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    pending: ResMut<'w, PendingSubscription>,
    subscriber: SceneSubscriber<'w, 's>,
    placement: ScenePlacement<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for SceneCreationWidget<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        if !matches!(params.app_state.get(), AppState::SiteEditor) {
            return;
        }

        ui.menu_button("🎬", |ui| {
            // The menu_button style isn't good for general widgets, so
            // we reset the style before drawing the inner widgets.
            ui.reset_style();

            ui.vertical(|ui| {
                egui::Resize::default()
                    .default_width(300.0)
                    .default_height(0.0)
                    .show(ui, |ui| {
                        ui.add_space(10.0);
                        Grid::new("scene_subscription")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Scene Topic");
                                ui.text_edit_singleline(&mut params.pending.scene_topic);
                                ui.end_row();

                                ui.label("Resource Service");
                                ui.text_edit_singleline(&mut params.pending.resource_service);
                                ui.end_row();

                                ui.label("Remove Prefixes");
                                let mut remove_prefixes = Vec::new();
                                let mut id: usize = 0;
                                for prefix in params.pending.prefixes.iter_mut() {
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
                                        params.pending.prefixes.push(String::new());
                                    }
                                });
                                for i in remove_prefixes.drain(..).rev() {
                                    params.pending.prefixes.remove(i);
                                }
                                ui.end_row();
                            });

                        if ui
                            .button("Subscribe")
                            .on_hover_text("Create the scene")
                            .clicked()
                        {
                            let scene_root = params.subscriber.spawn_scene(
                                params.pending.scene_topic.clone(),
                                params.pending.resource_service.clone(),
                                params.pending.prefixes.clone(),
                            );

                            params.placement.place_scene(scene_root);
                            ui.close_menu();
                        }
                    });
            });
        })
        .response
        .on_hover_text("Add Scene");
    }
}

#[derive(Resource, Default)]
struct PendingSubscription {
    scene_topic: String,
    resource_service: String,
    prefixes: Vec<String>,
}
