/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

use crate::{site::generate_site, AppState, CurrentWorkspace};
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemState},
    prelude::*,
};
use bevy_egui::{egui, EguiContexts};
use rmf_site_egui::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Blob, BlobPropertyBag, Clipboard, HtmlAnchorElement, Url, Window};

#[derive(Resource)]
pub struct JsonExportMenu {
    export_json: Entity,
    show_dialog: bool,
}

impl JsonExportMenu {
    pub fn get(&self) -> Entity {
        self.export_json
    }
}

impl FromWorld for JsonExportMenu {
    fn from_world(world: &mut World) -> Self {
        let file_header = world.resource::<FileMenu>().get();
        let export_json = world
            .spawn((
                MenuItem::Text(TextMenuItem::new("Export Json")),
                ChildOf(file_header),
            ))
            .id();

        JsonExportMenu {
            export_json,
            show_dialog: false,
        }
    }
}

fn handle_export_json_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    mut json_export: ResMut<JsonExportMenu>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == json_export.get() {
            json_export.show_dialog = true;
        }
    }
}

fn show_export_json_dialog(mut world: &mut World) {
    let mut state: SystemState<(Res<JsonExportMenu>, Res<CurrentWorkspace>)> =
        SystemState::new(world);
    let (json_export, current_workspace) = state.get_mut(world);

    if !json_export.show_dialog {
        return;
    }

    let Some(ws_root) = current_workspace.root else {
        warn!("Failed saving workspace, no current workspace found");
        return;
    };

    let mut site = match generate_site(world, ws_root) {
        Ok(site) => site,
        Err(err) => {
            error!("Unable to compile site: {err}");
            return;
        }
    };

    let mut site_str = match site.to_string_json_pretty() {
        Ok(json) => json,
        Err(err) => {
            error!("Unable to serialize site to JSON: {err}");
            return;
        }
    };

    let mut state: SystemState<EguiContexts> = SystemState::new(world);
    let mut contexts = state.get_mut(world);

    let mut close_dialog = false;
    egui::Window::new("Export Site JSON")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut(), |ui| {
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut site_str)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );
                });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui.button("Copy to clipboard").clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        ui.ctx().copy_text(site_str);

                        #[cfg(target_arch = "wasm32")]
                        if let Some(window) = web_sys::window() {
                            let _ = window.navigator().clipboard().write_text(&site_str);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    if ui.button("Download file").clicked() {
                        if download_site_file(site.properties.name.0.clone(), site_str.clone())
                            .is_err()
                        {
                            error!("Failed to download site JSON file");
                        }
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_dialog = true;
                    }
                });
            });
        });

    if close_dialog {
        let mut state: SystemState<ResMut<JsonExportMenu>> = SystemState::new(world);
        let mut json_export = state.get_mut(world);
        json_export.show_dialog = false;
    }
}

#[cfg(target_arch = "wasm32")]
fn download_site_file(site_name: String, site_str: String) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("Window not found for download")?;
    let document = window.document().ok_or("Document not found for download")?;
    let mut parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(&site_str));
    let blob =
        Blob::new_with_str_sequence_and_options(&parts, BlobPropertyBag::new().type_("text/json"))?;
    let url = Url::create_object_url_with_blob(&blob)?;
    let anchor = document
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()?;
    anchor.set_href(&url);
    let filename = site_name + ".site.json";
    anchor.set_download(&filename);
    anchor.click();

    Url::revoke_object_url(&url)?;

    Ok(())
}

#[derive(Default)]
pub struct JsonExportMenuPlugin {}

impl Plugin for JsonExportMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JsonExportMenu>().add_systems(
            Update,
            (
                handle_export_json_menu_events.run_if(AppState::in_displaying_mode()),
                show_export_json_dialog,
            ),
        );
    }
}
