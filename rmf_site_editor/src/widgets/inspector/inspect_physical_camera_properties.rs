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

use crate::inspector::{InspectAngle, InspectValue};
use crate::widgets::egui::RichText;
use bevy_egui::egui::{Grid, Ui};
use rmf_site_format::PhysicalCameraProperties;

pub struct InspectPhysicalCameraProperties<'a> {
    pub properties: &'a PhysicalCameraProperties,
}

impl<'a> InspectPhysicalCameraProperties<'a> {
    pub fn new(properties: &'a PhysicalCameraProperties) -> Self {
        Self { properties }
    }

    pub fn show(self, ui: &mut Ui) -> Option<PhysicalCameraProperties> {
        let mut new_properties = self.properties.clone();
        ui.label(RichText::new("Camera Properties").size(18.0));
        Grid::new("physical_camera_properties").show(ui, |ui| {
            if let Some(new_width) =
                InspectValue::<u32>::new("Width", new_properties.width)
                    .clamp_range(1..=std::u32::MAX)
                    .tooltip("Image width in pixels")
                    .show(ui)
            {
                new_properties.width = new_width;
            }
            ui.end_row();
            if let Some(new_height) =
                InspectValue::<u32>::new("Height", new_properties.height)
                    .clamp_range(1..=std::u32::MAX)
                    .tooltip("Image height in pixels")
                    .show(ui)
            {
                new_properties.height = new_height;
            }
            ui.end_row();
            if let Some(new_frame_rate) =
                InspectValue::<f32>::new("Frame rate", new_properties.frame_rate)
                    .clamp_range(0.0..=std::f32::MAX)
                    .tooltip("Frame rate in images per second")
                    .show(ui)
            {
                new_properties.frame_rate = new_frame_rate;
            }
            ui.end_row();
        });
        // Outside of main grid to avoid left padding
        ui.horizontal(|ui| {
            ui.label("Horizontal fov");
            InspectAngle::new(&mut new_properties.horizontal_fov)
                .range_degrees(0.0..=180.0)
                .show(ui);
        });
        if new_properties.width != self.properties.width
            || new_properties.height != self.properties.height
            || new_properties.horizontal_fov != self.properties.horizontal_fov
            || new_properties.frame_rate != self.properties.frame_rate
        {
            Some(new_properties)
        } else {
            None
        }
    }
}
