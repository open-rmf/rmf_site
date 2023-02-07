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
    interaction::{ChangeMode, SelectAnchor},
    widgets::AppEvents,
};
use bevy_egui::egui::Ui;

pub struct CreateWidget<'a, 'w, 's> {
    pub events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> CreateWidget<'a, 'w, 's> {
    pub fn new(events: &'a mut AppEvents<'w, 's>) -> Self {
        Self { events }
    }

    pub fn show(self, ui: &mut Ui) {
        ui.vertical(|ui| {
            if ui.button("Lane").clicked() {
                self.events.request.change_mode.send(ChangeMode::To(
                    SelectAnchor::create_new_edge_sequence().for_lane().into(),
                ));
            }

            if ui.button("Location").clicked() {
                self.events.request.change_mode.send(ChangeMode::To(
                    SelectAnchor::create_new_point().for_location().into(),
                ));
            }

            if ui.button("Wall").clicked() {
                self.events.request.change_mode.send(ChangeMode::To(
                    SelectAnchor::create_new_edge_sequence().for_wall().into(),
                ));
            }

            if ui.button("Door").clicked() {
                self.events.request.change_mode.send(ChangeMode::To(
                    SelectAnchor::create_one_new_edge().for_door().into(),
                ));
            }

            if ui.button("Lift").clicked() {
                self.events.request.change_mode.send(ChangeMode::To(
                    SelectAnchor::create_one_new_edge().for_lift().into(),
                ));
            }

            if ui.button("Floor").clicked() {
                self.events.request.change_mode.send(ChangeMode::To(
                    SelectAnchor::create_new_path().for_floor().into(),
                ));
            }

            if ui.button("Measurement").clicked() {
                self.events.request.change_mode.send(ChangeMode::To(
                    SelectAnchor::create_one_new_edge().for_measurement().into(),
                ));
            }
        });
    }
}
