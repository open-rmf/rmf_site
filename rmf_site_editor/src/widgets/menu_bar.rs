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
    interaction::VisibilityCategoriesSettings, AppState, CreateNewWorkspace, CurrentWorkspace,
    FileEvents, LoadWorkspace, SaveWorkspace,
};

use bevy_egui::{
    egui::{self, Button},
    EguiContext,
};

pub fn top_menu_bar(
    mut egui_context: &mut EguiContext,
    mut file_events: &mut FileEvents,
    mut category_settings: &mut VisibilityCategoriesSettings,
) {
    egui::TopBottomPanel::top("top_panel").show(egui_context.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.add(Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                    file_events.new_workspace.send(CreateNewWorkspace);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui
                        .add(Button::new("Save").shortcut_text("Ctrl+S"))
                        .clicked()
                    {
                        file_events
                            .save
                            .send(SaveWorkspace::new().to_default_file());
                    }
                    if ui
                        .add(Button::new("Save As").shortcut_text("Ctrl+Shift+S"))
                        .clicked()
                    {
                        file_events.save.send(SaveWorkspace::new().to_dialog());
                    }
                }
                if ui
                    .add(Button::new("Open").shortcut_text("Ctrl+O"))
                    .clicked()
                {
                    file_events.load_workspace.send(LoadWorkspace::Dialog);
                }
            });
            ui.menu_button("View", |ui| {
                // TODO(luca) consider cloning to avoid change detection?
                ui.checkbox(&mut category_settings.0.doors, "Doors");
                ui.checkbox(&mut category_settings.0.floors, "Floors");
                ui.checkbox(&mut category_settings.0.lanes, "Lanes");
                ui.checkbox(&mut category_settings.0.lifts, "Lifts");
                ui.checkbox(&mut category_settings.0.locations, "Locations");
                ui.checkbox(&mut category_settings.0.measurements, "Measurements");
                ui.checkbox(&mut category_settings.0.models, "Models");
                ui.checkbox(&mut category_settings.0.walls, "Walls");
            });
        });
    });
}
