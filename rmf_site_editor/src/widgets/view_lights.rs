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
    site::{Light, LightKind, RecallLightKind, SiteID, Pose, Recall, Category, Rotation, Angle},
    icons::Icons,
    widgets::{
        AppEvents,
        inspector::{SelectionWidget, InspectPose, InspectLightKind},
    },
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::egui::Ui;
use std::collections::BTreeMap;
use std::cmp::Reverse;

pub struct LightDisplay {
    pub pose: Pose,
    pub kind: LightKind,
    pub recall: RecallLightKind,
}

impl Default for LightDisplay {
    fn default() -> Self {
        Self {
            pose: Pose {
                trans: [0.0, 0.0, 2.0],
                rot: Rotation::EulerExtrinsicXYZ([
                    Angle::Deg(0.0), Angle::Deg(0.0), Angle::Deg(0.0)
                ])
            },
            kind: Default::default(),
            recall: Default::default(),
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
        ui.label("New light");
        if let Some(new_pose) = InspectPose::new(
            &self.events.light_display.pose
        ).show(ui) {
            self.events.light_display.pose = new_pose;
        }

        ui.push_id("Add Light", |ui| {
            if let Some(new_kind) = InspectLightKind::new(
                &self.events.light_display.kind, &self.events.light_display.recall,
            ).show(ui) {
                self.events.light_display.recall.remember(&new_kind);
                self.events.light_display.kind = new_kind;
            }
        });

        if ui.button("Add").clicked() {
            self.events.commands.spawn_bundle(Light {
                pose: self.events.light_display.pose,
                kind: self.events.light_display.kind,
            })
            .insert(Category::Light);
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
                SelectionWidget::new(
                    e.0, None, self.params.icons.as_ref(), self.events
                ).show(ui);
                ui.label(label);
            });
        }

        for (site_id, (e, label)) in saved_lights {
            ui.horizontal(|ui| {
                SelectionWidget::new(
                    e, Some(SiteID(site_id.0)), self.params.icons.as_ref(), self.events
                ).show(ui);
                ui.label(label);
            });
        }
    }
}
