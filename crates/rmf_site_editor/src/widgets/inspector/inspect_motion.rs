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
        inspector::{InspectAngle, InspectOptionF32},
        prelude::*,
        Inspect,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, RichText, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::{
    Angle, Dock, Motion, OrientationConstraint, RecallMotion, RecallReverseLane, ReverseLane,
};

#[derive(SystemParam)]
pub struct InspectMotion<'w, 's> {
    forward: InspectForwardMotion<'w, 's>,
    reverse: InspectReverseMotion<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectMotion<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.forward.show_widget(selection, ui);
        params.reverse.show_widget(selection, ui);
    }
}

#[derive(SystemParam)]
pub struct InspectForwardMotion<'w, 's> {
    commands: Commands<'w, 's>,
    motions: Query<'w, 's, (&'static Motion, &'static RecallMotion)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectForwardMotion<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(selection, ui);
    }
}

impl<'w, 's> InspectForwardMotion<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((motion, recall)) = self.motions.get(id) else {
            return;
        };

        if let Some(new_motion) = InspectMotionComponent::new(motion, recall).show(ui) {
            self.commands.trigger(Change::new(new_motion, id));
        }
        ui.add_space(10.0);
    }
}

#[derive(SystemParam)]
pub struct InspectReverseMotion<'w, 's> {
    commands: Commands<'w, 's>,
    reverse_motions: Query<'w, 's, (&'static ReverseLane, &'static RecallReverseLane)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectReverseMotion<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(selection, ui);
    }
}

impl<'w, 's> InspectReverseMotion<'w, 's> {
    fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((reverse, recall)) = self.reverse_motions.get(id) else {
            return;
        };

        let assumed_motion = reverse
            .different_motion()
            .cloned()
            .unwrap_or(recall.motion.clone().unwrap_or(Motion::default()));

        let mut new_reverse = reverse.clone();
        ui.label(RichText::new("Reverse Motion").size(18.0));
        ComboBox::from_id_salt("Reverse Lane")
            .selected_text(new_reverse.label())
            .show_ui(ui, |ui| {
                for variant in &[
                    ReverseLane::Same,
                    ReverseLane::Disable,
                    ReverseLane::Different(assumed_motion),
                ] {
                    ui.selectable_value(&mut new_reverse, variant.clone(), variant.label());
                }
            });

        ui.push_id("reverse_motion", |ui| {
            match &mut new_reverse {
                ReverseLane::Different(motion) => {
                    ui.add_space(10.0);
                    if let Some(new_motion) =
                        InspectMotionComponent::new(motion, &recall.previous).show(ui)
                    {
                        new_reverse = ReverseLane::Different(new_motion);
                    }
                }
                _ => {
                    // Do nothing
                }
            }
        });

        if new_reverse != *reverse {
            self.commands.trigger(Change::new(new_reverse, id));
        }
        ui.add_space(10.0);
    }
}

pub struct InspectMotionComponent<'a> {
    pub motion: &'a Motion,
    pub recall: &'a RecallMotion,
}

impl<'a> InspectMotionComponent<'a> {
    pub fn new(motion: &'a Motion, recall: &'a RecallMotion) -> Self {
        Self { motion, recall }
    }

    pub fn show(self, ui: &mut Ui) -> Option<Motion> {
        let new_orientation = ui
            .vertical(|ui| {
                let assumed_relative_yaw = self
                    .motion
                    .orientation_constraint
                    .relative_yaw()
                    .unwrap_or(self.recall.relative_yaw.unwrap_or(Angle::Deg(0.0)));

                let assumed_absolute_yaw = self
                    .motion
                    .orientation_constraint
                    .absolute_yaw()
                    .unwrap_or(self.recall.absolute_yaw.unwrap_or(Angle::Deg(0.0)));

                ui.label("Orientation Constraint");
                let mut orientation = self.motion.orientation_constraint.clone();
                ComboBox::from_id_salt("Orientation Constraint")
                    .selected_text(orientation.label())
                    .show_ui(ui, |ui| {
                        for variant in &[
                            OrientationConstraint::None,
                            OrientationConstraint::Forwards,
                            OrientationConstraint::Backwards,
                            OrientationConstraint::RelativeYaw(assumed_relative_yaw),
                            OrientationConstraint::AbsoluteYaw(assumed_absolute_yaw),
                        ] {
                            ui.selectable_value(&mut orientation, *variant, variant.label());
                        }
                    });

                match &mut orientation {
                    OrientationConstraint::RelativeYaw(value) => {
                        InspectAngle::new(value).show(ui);
                    }
                    OrientationConstraint::AbsoluteYaw(value) => {
                        InspectAngle::new(value).show(ui);
                    }
                    _ => {
                        // Do nothing
                    }
                }

                if orientation != self.motion.orientation_constraint {
                    return Some(orientation);
                }

                return None;
            })
            .inner;

        ui.add_space(10.0);
        let new_speed = InspectOptionF32::new(
            "Speed Limit",
            self.motion.speed_limit,
            self.recall.speed_limit.unwrap_or(1.0),
        )
        .clamp_range(0.0..=100.0)
        .min_decimals(2)
        .max_decimals(2)
        .speed(0.01)
        .suffix(" m/s")
        .show(ui);

        ui.add_space(10.0);
        let mut has_dock = self.motion.dock.is_some();
        ui.checkbox(&mut has_dock, "Dock");
        let new_dock = if has_dock {
            let mut dock =
                self.motion
                    .dock
                    .clone()
                    .unwrap_or(self.recall.dock.clone().unwrap_or_else(|| {
                        Dock {
                            name: self
                                .recall
                                .dock_name
                                .clone()
                                .unwrap_or("<Unnamed>".to_string()),
                            duration: self.recall.dock_duration,
                        }
                    }));

            ui.horizontal(|ui| {
                ui.label("name");
                ui.text_edit_singleline(&mut dock.name);
            });

            let new_duration = InspectOptionF32::new(
                "Duration",
                dock.duration,
                self.recall.dock_duration.unwrap_or(30.0),
            )
            .clamp_range(0.0..=std::f32::INFINITY)
            .min_decimals(0)
            .max_decimals(1)
            .speed(1.0)
            .suffix(" s")
            .tooltip("How long does the docking take?")
            .show(ui);

            if let Some(new_duration) = new_duration {
                dock.duration = new_duration;
            }

            if Some(&dock) != self.motion.dock.as_ref() {
                Some(Some(dock))
            } else {
                None
            }
        } else {
            if self.motion.dock.is_some() {
                Some(None)
            } else {
                None
            }
        };

        if new_orientation.is_some() || new_speed.is_some() || new_dock.is_some() {
            let mut new_motion = self.motion.clone();
            if let Some(new_orientation) = new_orientation {
                new_motion.orientation_constraint = new_orientation;
            }

            if let Some(new_speed) = new_speed {
                new_motion.speed_limit = new_speed;
            }

            if let Some(new_dock) = new_dock {
                new_motion.dock = new_dock;
            }

            return Some(new_motion);
        }

        return None;
    }
}
