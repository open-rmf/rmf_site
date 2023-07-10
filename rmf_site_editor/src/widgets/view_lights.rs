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
    icons::Icons,
    interaction::Select,
    site::{
        Angle, Category, ExportLights, Light, LightKind, Pose, Recall, RecallLightKind, Rotation,
        SiteID,
    },
    widgets::{
        inspector::{InspectLightKind, InspectPose, SelectionWidget},
        AppEvents,
    },
};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::egui::Ui;
use futures_lite::future;
#[cfg(not(target_arch = "wasm32"))]
use rfd::AsyncFileDialog;
use std::cmp::Reverse;
use std::collections::BTreeMap;

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

#[derive(SystemParam)]
pub struct LightParams<'w, 's> {
    pub lights: Query<'w, 's, (Entity, &'static LightKind, Option<&'static SiteID>)>,
    pub icons: Res<'w, Icons>,
}

pub struct ViewLights<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a LightParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewLights<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a LightParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        let mut use_headlight = self.events.request.toggle_headlights.0;
        ui.checkbox(&mut use_headlight, "Use Headlight");
        if use_headlight != self.events.request.toggle_headlights.0 {
            self.events.request.toggle_headlights.0 = use_headlight;
        }

        let mut use_physical_lights = self.events.request.toggle_physical_lights.0;
        ui.checkbox(&mut use_physical_lights, "Use Physical Lights");
        if use_physical_lights != self.events.request.toggle_physical_lights.0 {
            self.events.request.toggle_physical_lights.0 = use_physical_lights;
        }

        ui.separator();

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.horizontal(|ui| {
                if let Some(export_file) = &self.events.display.light.export_file {
                    if ui.button("Export").clicked() {
                        self.events
                            .request
                            .export_lights
                            .send(ExportLights(export_file.clone()));
                    }
                }
                if ui.button("Export Lights As...").clicked() {
                    match &self.events.display.light.choosing_file_for_export {
                        Some(_) => {
                            warn!("A file is already being chosen!");
                        }
                        None => {
                            let future = AsyncComputeTaskPool::get().spawn(async move {
                                let file = match AsyncFileDialog::new().save_file().await {
                                    Some(file) => file,
                                    None => return None,
                                };

                                Some(file.path().to_path_buf())
                            });
                            self.events.display.light.choosing_file_for_export = Some(future);
                        }
                    }
                }
            });
            match &self.events.display.light.export_file {
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
        if let Some(new_pose) = InspectPose::new(&self.events.display.light.pose).show(ui) {
            self.events.display.light.pose = new_pose;
        }

        ui.push_id("Add Light", |ui| {
            if let Some(new_kind) = InspectLightKind::new(
                &self.events.display.light.kind,
                &self.events.display.light.recall,
            )
            .show(ui)
            {
                self.events.display.light.recall.remember(&new_kind);
                self.events.display.light.kind = new_kind;
            }
        });

        // TODO(MXG): Add a + icon to this button to make it more visible
        if ui.button("Add").clicked() {
            let new_light = self
                .events
                .commands
                .spawn(Light {
                    pose: self.events.display.light.pose,
                    kind: self.events.display.light.kind,
                })
                .insert(Category::Light)
                .id();
            self.events.request.select.send(Select(Some(new_light)));
        }

        ui.separator();

        let mut unsaved_lights = BTreeMap::new();
        let mut saved_lights = BTreeMap::new();
        for (e, kind, site_id) in &self.params.lights {
            if let Some(site_id) = site_id {
                saved_lights.insert(Reverse(site_id.0), (e, kind.label()));
            } else {
                unsaved_lights.insert(Reverse(e), kind.label());
            }
        }

        for (e, label) in unsaved_lights {
            ui.horizontal(|ui| {
                SelectionWidget::new(e.0, None, self.params.icons.as_ref(), self.events).show(ui);
                ui.label(label);
            });
        }

        for (site_id, (e, label)) in saved_lights {
            ui.horizontal(|ui| {
                SelectionWidget::new(
                    e,
                    Some(SiteID(site_id.0)),
                    self.params.icons.as_ref(),
                    self.events,
                )
                .show(ui);
                ui.label(label);
            });
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
