/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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
    interaction::{HeadlightToggle, Select},
    site::{
        Angle, Category, ExportLights, Light, LightKind, PhysicalLightToggle, Pose, Recall,
        RecallLightKind, Rotation, SiteID,
    },
    widgets::{
        inspector::{InspectLightKind, InspectPoseComponent},
        prelude::*,
        SelectorWidget,
    },
    AppState,
};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::egui::{CollapsingHeader, Ui};
use futures_lite::future;
#[cfg(not(target_arch = "wasm32"))]
use rfd::AsyncFileDialog;
use std::cmp::Reverse;
use std::collections::BTreeMap;

/// Add a plugin for viewing and editing a list of all lights
#[derive(Default)]
pub struct ViewLightsPlugin {}

impl Plugin for ViewLightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightDisplay>()
            .add_plugins(PropertiesTilePlugin::<ViewLights>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewLights<'w, 's> {
    lights: Query<'w, 's, (Entity, &'static LightKind, Option<&'static SiteID>)>,
    toggle_headlights: ResMut<'w, HeadlightToggle>,
    toggle_physical_lights: ResMut<'w, PhysicalLightToggle>,
    export_lights: EventWriter<'w, ExportLights>,
    display_light: ResMut<'w, LightDisplay>,
    selector: SelectorWidget<'w, 's>,
    commands: Commands<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewLights<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        CollapsingHeader::new("Lights")
            .default_open(false)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewLights<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let mut use_headlight = self.toggle_headlights.0;
        ui.checkbox(&mut use_headlight, "Use Headlight");
        if use_headlight != self.toggle_headlights.0 {
            self.toggle_headlights.0 = use_headlight;
        }

        let mut use_physical_lights = self.toggle_physical_lights.0;
        ui.checkbox(&mut use_physical_lights, "Use Physical Lights");
        if use_physical_lights != self.toggle_physical_lights.0 {
            self.toggle_physical_lights.0 = use_physical_lights;
        }

        ui.separator();

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.horizontal(|ui| {
                if let Some(export_file) = &self.display_light.export_file {
                    if ui.button("Export").clicked() {
                        self.export_lights.send(ExportLights(export_file.clone()));
                    }
                }
                if ui.button("Export Lights As...").clicked() {
                    match &self.display_light.choosing_file_for_export {
                        Some(_) => {
                            warn!("A file is already being chosen!");
                        }
                        None => {
                            let future = AsyncComputeTaskPool::get().spawn(async move {
                                // TODO(luca) change this to use FileDialogServices
                                let file = match AsyncFileDialog::new().save_file().await {
                                    Some(file) => file,
                                    None => return None,
                                };

                                Some(file.path().to_path_buf())
                            });
                            self.display_light.choosing_file_for_export = Some(future);
                        }
                    }
                }
            });
            match &self.display_light.export_file {
                Some(path) => match path.to_str() {
                    Some(s) => {
                        ui.label(s);
                    }
                    None => {
                        ui.label("unable to render path");
                    }
                },
                None => {
                    ui.label("<no file chosen>");
                }
            }
            ui.separator();
        }

        ui.heading("Create new light");
        if let Some(new_pose) = InspectPoseComponent::new(&self.display_light.pose).show(ui) {
            self.display_light.pose = new_pose;
        }

        ui.push_id("Add Light", |ui| {
            if let Some(new_kind) =
                InspectLightKind::new(&self.display_light.kind, &self.display_light.recall).show(ui)
            {
                self.display_light.recall.remember(&new_kind);
                self.display_light.kind = new_kind;
            }
        });

        // TODO(MXG): Add a + icon to this button to make it more visible
        if ui.button("Add").clicked() {
            let new_light = self
                .commands
                .spawn(Light {
                    pose: self.display_light.pose,
                    kind: self.display_light.kind,
                })
                .insert(Category::Light)
                .id();
            self.selector.select.send(Select::new(Some(new_light)));
        }

        ui.separator();

        let mut unsaved_lights = BTreeMap::new();
        let mut saved_lights = BTreeMap::new();
        for (e, kind, site_id) in &self.lights {
            if let Some(site_id) = site_id {
                saved_lights.insert(Reverse(site_id.0), (e, kind.label()));
            } else {
                unsaved_lights.insert(Reverse(e), kind.label());
            }
        }

        for (e, label) in unsaved_lights {
            ui.horizontal(|ui| {
                self.selector.show_widget(e.0, ui);
                ui.label(label);
            });
        }

        for (_, (e, label)) in saved_lights {
            ui.horizontal(|ui| {
                self.selector.show_widget(e, ui);
                ui.label(label);
            });
        }
    }
}

#[derive(Resource)]
pub struct LightDisplay {
    pub pose: Pose,
    pub kind: LightKind,
    pub recall: RecallLightKind,
    pub choosing_file_for_export: Option<Task<Option<std::path::PathBuf>>>,
    pub export_file: Option<std::path::PathBuf>,
}

impl Default for LightDisplay {
    fn default() -> Self {
        Self {
            pose: Pose {
                trans: [0.0, 0.0, 2.6],
                rot: Rotation::EulerExtrinsicXYZ([
                    Angle::Deg(0.0),
                    Angle::Deg(0.0),
                    Angle::Deg(0.0),
                ]),
            },
            kind: Default::default(),
            recall: Default::default(),
            choosing_file_for_export: None,
            export_file: None,
        }
    }
}

pub fn resolve_light_export_file(
    mut light_display: ResMut<LightDisplay>,
    mut export_lights: EventWriter<ExportLights>,
) {
    let mut resolved = false;
    if let Some(task) = &mut light_display.choosing_file_for_export {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            resolved = true;

            if let Some(result) = result {
                export_lights.send(ExportLights(result.clone()));
                light_display.export_file = Some(result);
            }
        }
    }

    if resolved {
        light_display.choosing_file_for_export = None;
    }
}
