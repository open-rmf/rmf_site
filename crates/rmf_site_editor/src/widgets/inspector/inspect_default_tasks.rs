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

use crate::TaskKinds;
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, DragValue, Ui};
use rmf_site_format::{GoToPlace, Task, WaitFor};

#[derive(Default)]
pub struct InspectDefaultTasksPlugin {}

impl Plugin for InspectDefaultTasksPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().resource_mut::<TaskKinds>().0.insert(
            GoToPlace::label(),
            (
                |id, task, locations, ui| {
                    InspectGoToPlace::new(id, task, locations).show(ui);
                },
                |task, locations| InspectGoToPlace::is_valid(task, locations),
            ),
        );
        app.world_mut().resource_mut::<TaskKinds>().0.insert(
            WaitFor::label(),
            (
                |id, task, locations, ui| {
                    InspectWaitFor::new(id, task, locations).show(ui);
                },
                |task, locations| InspectWaitFor::is_valid(task, locations),
            ),
        );
    }
}

pub struct InspectGoToPlace<'a> {
    id: &'a usize,
    task: &'a mut Task,
    locations: &'a Vec<String>,
}

impl<'a> InspectGoToPlace<'a> {
    pub fn new(id: &'a usize, task: &'a mut Task, locations: &'a Vec<String>) -> Self {
        Self {
            id,
            task,
            locations,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let mut new_go_to_place =
            match serde_json::from_value::<GoToPlace>(self.task.config.clone()) {
                Ok(go_to_place) => go_to_place,
                Err(_) => GoToPlace::default(),
            };

        let selected_location_name = if new_go_to_place.is_default()
            || !self.locations.contains(&new_go_to_place.location)
        {
            "Select Location".to_string()
        } else {
            new_go_to_place.location.clone()
        };
        ComboBox::from_id_salt(self.id.to_string() + "select_go_to_location")
            .selected_text(selected_location_name)
            .show_ui(ui, |ui| {
                for location_name in self.locations.iter() {
                    ui.selectable_value(
                        &mut new_go_to_place.location,
                        location_name.clone(),
                        location_name.clone(),
                    );
                }
            });
        if !new_go_to_place.is_default() && self.locations.contains(&new_go_to_place.location) {
            if let Ok(new_value) = serde_json::to_value(new_go_to_place) {
                self.task.config = new_value;
            }
        }
    }

    pub fn is_valid(task: &Task, locations: &Vec<String>) -> bool {
        match serde_json::from_value::<GoToPlace>(task.config.clone()) {
            Ok(go_to_place) => locations.contains(&go_to_place.location),
            Err(_) => false,
        }
    }
}

pub struct InspectWaitFor<'a> {
    task: &'a mut Task,
}

impl<'a> InspectWaitFor<'a> {
    pub fn new(_id: &'a usize, task: &'a mut Task, _locations: &'a Vec<String>) -> Self {
        Self { task }
    }

    pub fn show(self, ui: &mut Ui) {
        let mut new_wait_for = match serde_json::from_value::<WaitFor>(self.task.config.clone()) {
            Ok(wait_for) => wait_for,
            Err(_) => WaitFor::default(),
        };
        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(&mut new_wait_for.duration)
                    .range(0_f32..=std::f32::INFINITY)
                    .speed(0.01),
            );
            ui.label(" seconds");
        });

        if let Ok(new_value) = serde_json::to_value(new_wait_for) {
            self.task.config = new_value;
        }
    }

    pub fn is_valid(task: &Task, _locations: &Vec<String>) -> bool {
        match serde_json::from_value::<WaitFor>(task.config.clone()) {
            Ok(_wait_for) => true,
            Err(_) => false,
        }
    }
}
