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

use bevy_egui::egui::{ComboBox, DragValue, Ui};
use rmf_site_format::{MeshPrimitive, RecallMeshPrimitive};

pub struct InspectMeshPrimitive<'a> {
    pub primitive: &'a MeshPrimitive,
    pub recall: &'a RecallMeshPrimitive,
}

impl<'a> InspectMeshPrimitive<'a> {
    pub fn new(primitive: &'a MeshPrimitive, recall: &'a RecallMeshPrimitive) -> Self {
        Self { primitive, recall }
    }

    pub fn show(self, ui: &mut Ui) -> Option<MeshPrimitive> {
        let mut new_primitive = self.primitive.clone();
        // TODO(luca) implement recall plugin
        ui.horizontal(|ui| {
            ui.label("Primitive");
            ComboBox::from_id_source("Mesh Primitive")
                .selected_text(new_primitive.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        self.recall.assume_box(self.primitive),
                        self.recall.assume_cylinder(self.primitive),
                        self.recall.assume_capsule(self.primitive),
                        self.recall.assume_sphere(self.primitive),
                    ] {
                        ui.selectable_value(&mut new_primitive, variant.clone(), variant.label());
                    }
                    ui.end_row();
                });
        });
        match &mut new_primitive {
            MeshPrimitive::Box { size } => {
                ui.add(DragValue::new(&mut size[0]).clamp_range(0_f32..=std::f32::INFINITY));
                ui.add(DragValue::new(&mut size[1]).clamp_range(0_f32..=std::f32::INFINITY));
                ui.add(DragValue::new(&mut size[2]).clamp_range(0_f32..=std::f32::INFINITY));
            }
            MeshPrimitive::Cylinder { radius, length } => {}
            MeshPrimitive::Capsule { radius, length } => {}
            MeshPrimitive::Sphere { radius } => {}
        }
        if &new_primitive != self.primitive {
            Some(new_primitive)
        } else {
            None
        }
    }
}
