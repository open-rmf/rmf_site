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
    site::{Original, Previously},
    widgets::{
        inspector::{InspectEdgeWidget, InspectAnchorParams, AppEvents},
    }
};
use rmf_site_format::{
    Edge, Motion, ReverseLane, LaneMarker,
};
use bevy::prelude::*;
use bevy_egui::egui::Ui;

pub type LaneQuery<'w, 's> = Query<'w, 's, (
    &'static Edge<Entity>,
    Option<&'static Original<Edge<Entity>>>,
    &'static Motion,
    Option<&'static Previously<Motion>>,
    &'static ReverseLane,
    Option<&'static Previously<ReverseLane>>,
), With<LaneMarker>>;

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

    }
}

pub struct InspectMotionWidget<'a> {
    pub motion: &'a Motion,
    pub previous: Option<&'a Motion>,
    pub disabled: bool,
}

impl<'a> InspectMotionWidget<'a> {

    pub fn new(motion: &'a Motion, previous: Option<&'a Motion>) -> Self {
        Self{motion, previous, disabled: false}
    }

    pub fn disable(self) -> Self {
        Self{disabled: true, ..self}
    }

    pub fn show(self, ui: &mut Ui) -> Option<Motion> {

    }
}
