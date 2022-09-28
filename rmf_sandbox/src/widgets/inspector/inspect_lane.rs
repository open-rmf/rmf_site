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
    site::{Original, Change},
    widgets::{
        inspector::{
            InspectEdgeWidget, InspectAnchorParams, AppEvents, InspectAngle,
        },
    }
};
use rmf_site_format::{
    Edge, Motion, ReverseLane, LaneMarker, OrientationConstraint, Angle,
};
use bevy::prelude::*;
use bevy_egui::egui::{
    Ui, ComboBox, DragValue, Button,
};

pub type LaneQuery<'w, 's> = Query<'w, 's, (
    &'static Edge<Entity>,
    Option<&'static Original<Edge<Entity>>>,
    &'static Motion,
    &'static PreviousMotion,
    &'static ReverseLane,
    &'static PreviousReverse,
), With<LaneMarker>>;

#[derive(Clone, Debug, Default, Component)]
pub struct PreviousMotion {
    pub relative_yaw: Option<Angle>,
    pub absolute_yaw: Option<Angle>,
    pub speed_limit: Option<f32>,
    pub dock_name: Option<String>,
    pub dock_duration: Option<f32>,
}

impl PreviousMotion {
    pub fn absorb(&mut self, from_motion: &Motion) {
        match from_motion.orientation_constraint {
            OrientationConstraint::RelativeYaw(v) => {
                self.relative_yaw = Some(v);
            },
            OrientationConstraint::AbsoluteYaw(v) => {
                self.absolute_yaw = Some(v);
            },
            _ => {
                // Do nothing
            }
        }

        if let Some(s) = from_motion.speed_limit {
            self.speed_limit = Some(s);
        }

        if let Some(dock) = &from_motion.dock {
            self.dock_name = Some(dock.name.clone());
            if let Some(duration) = dock.duration {
                self.dock_duration = Some(duration);
            }
        }
    }
}

#[derive(Clone, Debug, Default, Component)]
pub struct PreviousReverse{
    pub motion: Option<Motion>,
    pub previous: PreviousMotion,
}

impl PreviousReverse {
    pub fn absorb(&mut self, from_reverse: &ReverseLane) {
        match from_reverse {
            ReverseLane::Different(from_motion) => {
                self.motion = Some(from_motion.clone());
                self.previous.absorb(from_motion);
            },
            _ => {
                // Do nothing
            }
        }
    }
}

pub struct InspectLaneWidget<'a, 'w1, 'w2, 'w3, 's1, 's2, 's3> {
    pub entity: Entity,
    pub lanes: &'a LaneQuery<'w1, 's1>,
    pub anchor_params: &'a mut InspectAnchorParams<'w2, 's2>,
    pub events: &'a mut AppEvents<'w3, 's3>,
}

impl<'a, 'w1, 'w2, 'w3, 's1, 's2, 's3> InspectLaneWidget<'a, 'w1, 'w2, 'w3, 's1, 's2, 's3> {

    pub fn new(
        entity: Entity,
        lanes: &'a LaneQuery<'w1, 's1>,
        anchor_params: &'a mut InspectAnchorParams<'w2, 's2>,
        events: &'a mut AppEvents<'w3, 's3>,
    ) -> Self {
        Self{entity, lanes, anchor_params, events}
    }

    pub fn show(self, ui: &mut Ui) {
        let (edge, original, forward, p_forward, reverse, p_reverse) = self.lanes.get(self.entity).unwrap();
        InspectEdgeWidget::new(
            self.entity, edge, original, self.anchor_params, self.events,
        ).show(ui);

        if let Some(new_motion) = InspectMotionWidget::new(forward, p_forward).show(ui) {
            self.events.change_motion.send(Change::new(new_motion, self.entity));
        }
    }
}

pub struct InspectMotionWidget<'a> {
    pub motion: &'a Motion,
    pub previous: &'a PreviousMotion,
    pub disabled: bool,
}

impl<'a> InspectMotionWidget<'a> {

    pub fn new(motion: &'a Motion, previous: &'a PreviousMotion) -> Self {
        Self{motion, previous, disabled: false}
    }

    pub fn disable(self) -> Self {
        Self{disabled: true, ..self}
    }

    pub fn show(self, ui: &mut Ui) -> Option<Motion> {
        ui.add_space(10.0);
        ui.label("Orientation Constraint");
        let new_orientation = ui.horizontal(|ui| {
            let assumed_relative_yaw =
                self.motion.orientation_constraint.relative_yaw().unwrap_or(
                    self.previous.relative_yaw.unwrap_or(Angle::Deg(0.0))
                );

            let assumed_absolute_yaw =
                self.motion.orientation_constraint.absolute_yaw().unwrap_or(
                    self.previous.absolute_yaw.unwrap_or(Angle::Deg(0.0))
                );

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
                },
                OrientationConstraint::AbsoluteYaw(value) => {
                    InspectAngle::new(value).show(ui);
                },
                _ => {
                    // Do nothing
                }
            }

            if orientation != self.motion.orientation_constraint {
                return Some(orientation);
            }

            return None;
        }).inner;

        ui.add_space(10.0);
        let new_speed = ui.horizontal(|ui| {
            let mut assumed_speed = self.motion.speed_limit.unwrap_or(
                self.previous.speed_limit.unwrap_or(0.0)
            );

            let mut has_speed_limit = self.motion.speed_limit.is_some();
            ui.checkbox(&mut has_speed_limit, "Speed Limit");
            if has_speed_limit {
                ui.add(
                    DragValue::new(&mut assumed_speed)
                    .clamp_range(0.0..=100.0)
                    .min_decimals(2)
                    .max_decimals(2)
                    .speed(0.01)
                    .suffix(" m/s")
                );
            }

            if has_speed_limit {
                if self.motion.speed_limit != Some(assumed_speed) {
                    return Some(Some(assumed_speed));
                }
            } else {
                if self.motion.speed_limit.is_some() {
                    return Some(None);
                }
            }

            return None;
        }).inner;

        if new_orientation.is_some() || new_speed.is_some() {
            let mut new_motion = self.motion.clone();
            if let Some(new_orientation) = new_orientation {
                new_motion.orientation_constraint = new_orientation;
            }

            if let Some(new_speed) = new_speed {
                new_motion.speed_limit = new_speed;
            }

            return Some(new_motion);
        }

        return None;
    }
}

pub fn add_previous_lane_trackers(
    mut commands: Commands,
    new_lanes: Query<(Entity, &Motion, &ReverseLane), Added<LaneMarker>>,
) {
    for (e, motion, reverse) in &new_lanes {
        let mut p_motion = PreviousMotion::default();
        p_motion.absorb(motion);
        commands.entity(e).insert(p_motion);

        let mut p_reverse = PreviousReverse::default();
        p_reverse.absorb(reverse);
        commands.entity(e).insert(p_reverse);
    }
}

pub fn update_previous_lane_trackers(
    mut changed_motions: Query<(&Motion, &mut PreviousMotion), Changed<Motion>>,
    mut changed_reverses: Query<(&ReverseLane, &mut PreviousReverse), Changed<ReverseLane>>,
) {
    for (motion, mut previous) in &mut changed_motions {
        previous.absorb(motion);
    }

    for (reverse, mut previous) in &mut changed_motions {
        previous.absorb(reverse);
    }
}
