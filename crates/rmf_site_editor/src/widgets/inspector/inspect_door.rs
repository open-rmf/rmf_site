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
    site::Change,
    widgets::{
        Inspect,
        inspector::{InspectAngle, InspectSide},
        prelude::*,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, DragValue, Slider, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::{DoorType, RecallDoorType, Swing};

#[derive(SystemParam)]
pub struct InspectDoor<'w, 's> {
    commands: Commands<'w, 's>,
    doors: Query<'w, 's, (&'static DoorType, &'static RecallDoorType)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectDoor<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Ok((door, recall)) = params.doors.get(selection) else {
            return;
        };

        if let Some(new_door) = InspectDoorType::new(door, recall).show(ui) {
            params.commands.trigger(Change::new(new_door, selection));
        }
        ui.add_space(10.0);
    }
}

pub struct InspectDoorType<'a> {
    pub kind: &'a DoorType,
    pub recall: &'a RecallDoorType,
}

impl<'a> InspectDoorType<'a> {
    pub fn new(kind: &'a DoorType, recall: &'a RecallDoorType) -> Self {
        Self { kind, recall }
    }

    pub fn show(self, ui: &mut Ui) -> Option<DoorType> {
        let mut new_kind = self.kind.clone();
        ui.horizontal(|ui| {
            ui.label("Door Type:");
            ComboBox::from_id_salt("Door Type")
                .selected_text(self.kind.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        self.recall.assume_single_sliding(self.kind),
                        self.recall.assume_double_sliding(self.kind),
                        self.recall.assume_single_swing(self.kind),
                        self.recall.assume_double_swing(self.kind),
                        self.recall.assume_model(self.kind),
                    ] {
                        ui.selectable_value(&mut new_kind, variant.clone(), variant.label());
                    }
                });
        });

        fn left_right_ratio_ui(ui: &mut Ui, ratio: &mut f32) {
            ui.horizontal(|ui| {
                ui.label("Left : Right");
                ui.add(
                    DragValue::new(ratio)
                        .speed(0.01)
                        .range(0.01..=std::f32::INFINITY),
                )
                .on_hover_text("(Left Door Length)/(Right Door Length)");
            });
        }

        match &mut new_kind {
            DoorType::SingleSliding(door) => {
                ui.horizontal(|ui| {
                    ui.label("Direction:")
                        .on_hover_text("The direction the door will slide towards");
                    InspectSide::new(&mut door.towards).show(ui);
                });
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    InspectPosition::new(&mut door.position).show(ui);
                });
            }
            DoorType::DoubleSliding(door) => {
                left_right_ratio_ui(ui, &mut door.left_right_ratio);

                ui.horizontal(|ui| {
                    ui.label("Left Position:");
                    InspectPosition::new(&mut door.left_position).show(ui);
                });
                ui.horizontal(|ui| {
                    ui.label("Right Position:");
                    InspectPosition::new(&mut door.right_position).show(ui);
                });
            }
            DoorType::SingleSwing(door) => {
                ui.horizontal(|ui| {
                    ui.label("Pivot Side: ");
                    InspectSide::new(&mut door.pivot_on).show(ui);
                });
                ui.add_space(5.0);
                InspectSwing::new(&mut door.swing).show(ui);

                ui.horizontal(|ui| {
                    ui.label("Position:");
                    InspectPosition::new(&mut door.position).show(ui);
                });
            }
            DoorType::DoubleSwing(door) => {
                InspectSwing::new(&mut door.swing).show(ui);
                left_right_ratio_ui(ui, &mut door.left_right_ratio);

                ui.horizontal(|ui| {
                    ui.label("Left Position:");
                    InspectPosition::new(&mut door.left_position).show(ui);
                });
                ui.horizontal(|ui| {
                    ui.label("Right Position:");
                    InspectPosition::new(&mut door.right_position).show(ui);
                });
            }
            DoorType::Model(_) => {
                ui.label("Not yet supported");
            }
        }

        if new_kind != *self.kind {
            Some(new_kind)
        } else {
            None
        }
    }
}

pub struct InspectPosition<'a> {
    pub position: &'a mut f32,
}

impl<'a> InspectPosition<'a> {
    pub fn new(position: &'a mut f32) -> Self {
        Self { position }
    }

    pub fn show(self, ui: &mut Ui) {
        ui.add(Slider::new(self.position, (0.0)..=(1.0)));
    }
}

pub struct InspectSwing<'a> {
    pub swing: &'a mut Swing,
}

impl<'a> InspectSwing<'a> {
    pub fn new(swing: &'a mut Swing) -> Self {
        Self { swing }
    }

    pub fn show(self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Swing:");
            ComboBox::from_id_salt("Door Swing")
                .selected_text(self.swing.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        self.swing.assume_forward(),
                        self.swing.assume_backward(),
                        self.swing.assume_both(),
                    ] {
                        ui.selectable_value(self.swing, *variant, variant.label());
                    }
                })
        });

        match self.swing {
            Swing::Forward(angle) => {
                ui.horizontal(|ui| {
                    ui.label("Limit:");
                    InspectAngle::new(angle).range_degrees(0.0..=180.0).show(ui);
                });
            }
            Swing::Backward(angle) => {
                ui.horizontal(|ui| {
                    ui.label("Limit:");
                    InspectAngle::new(angle).range_degrees(0.0..=180.0).show(ui);
                });
            }
            Swing::Both { forward, backward } => {
                ui.horizontal(|ui| {
                    ui.label("Forward Limit: ");
                    InspectAngle::new(forward)
                        .range_degrees(0.0..=180.0)
                        .show(ui);
                });
                ui.horizontal(|ui| {
                    ui.label("Backward Limit: ");
                    InspectAngle::new(backward)
                        .range_degrees(0.0..=180.0)
                        .show(ui);
                });
            }
        }
    }
}
