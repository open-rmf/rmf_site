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

use crate::widgets::inspector::{InspectAngle, InspectOptionF32};
use bevy_egui::egui::{ComboBox, RichText, Ui};
use rmf_site_format::{
    Angle, Dock, Motion, OrientationConstraint, RecallMotion, RecallReverseLane,
    ReverseLane,
};

pub struct InspectMotionWidget<'a> {
    pub motion: &'a Motion,
    pub recall: &'a RecallMotion,
}

impl<'a> InspectMotionWidget<'a> {
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
                ComboBox::from_id_source("Orientation Constraint")
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
            "Speed Limit".to_string(),
            self.motion.speed_limit,
            self.recall.speed_limit.unwrap_or(1.0),
        )
        .clamp_range(0.0..=100.0)
        .min_decimals(2)
        .max_decimals(2)
        .speed(0.01)
        .suffix(" m/s".to_string())
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
                "Duration".to_string(),
                dock.duration,
                self.recall.dock_duration.unwrap_or(30.0),
            )
            .clamp_range(0.0..=std::f32::INFINITY)
            .min_decimals(0)
            .max_decimals(1)
            .speed(1.0)
            .suffix(" s".to_string())
            .tooltip("How long does the docking take?".to_string())
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

pub struct InspectReverseWidget<'a> {
    pub reverse: &'a ReverseLane,
    pub recall: &'a RecallReverseLane,
}

impl<'a> InspectReverseWidget<'a> {
    pub fn new(reverse: &'a ReverseLane, previous: &'a RecallReverseLane) -> Self {
        Self {
            reverse,
            recall: previous,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Option<ReverseLane> {
        let assumed_motion = self
            .reverse
            .different_motion()
            .cloned()
            .unwrap_or(self.recall.motion.clone().unwrap_or(Motion::default()));

        let mut new_reverse = self.reverse.clone();
        ui.label(RichText::new("Reverse Motion").size(18.0));
        ComboBox::from_id_source("Reverse Lane")
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

        match &mut new_reverse {
            ReverseLane::Different(motion) => {
                ui.add_space(10.0);
                if let Some(new_motion) =
                    InspectMotionWidget::new(motion, &self.recall.previous).show(ui)
                {
                    new_reverse = ReverseLane::Different(new_motion);
                }
            }
            _ => {
                // Do nothing
            }
        }

        if new_reverse != *self.reverse {
            Some(new_reverse)
        } else {
            None
        }
    }
}
