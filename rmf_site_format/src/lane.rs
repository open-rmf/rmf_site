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

use crate::*;
use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::prelude::{Component, Entity, Bundle};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature="bevy", derive(Bundle))]
pub struct Lane<T: RefTrait> {
    /// The endpoints of the lane (start, end)
    pub anchors: Edge<T>,
    /// The properties of the lane when traveling forwards
    pub forward: Motion,
    /// The properties of the lane when traveling in reverse
    pub reverse: ReverseLane,
    /// Marker that tells bevy the entity is a Lane-type
    #[serde(skip)]
    pub marker: LaneMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct LaneMarker;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Motion {
    #[serde(skip_serializing_if="OrientationConstraint::is_none")]
    pub orientation_constraint: OrientationConstraint,
    #[serde(skip_serializing_if="Option::is_none")]
    pub speed_limit: Option<f32>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub dock: Option<Dock>,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct RecallMotion {
    pub relative_yaw: Option<Angle>,
    pub absolute_yaw: Option<Angle>,
    pub speed_limit: Option<f32>,
    pub dock: Option<Dock>,
    pub dock_name: Option<String>,
    pub dock_duration: Option<f32>,
}

impl Recall for RecallMotion {
    type Source = Motion;

    fn remember(&mut self, source: &Motion) {
        match source.orientation_constraint {
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

        if let Some(s) = source.speed_limit {
            self.speed_limit = Some(s);
        }

        if let Some(dock) = &source.dock {
            self.dock = Some(dock.clone());
            self.dock_name = Some(dock.name.clone());
            if let Some(duration) = dock.duration {
                self.dock_duration = Some(duration);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum OrientationConstraint {
    None,
    Forwards,
    Backwards,
    RelativeYaw(Angle),
    AbsoluteYaw(Angle),
}

impl OrientationConstraint {

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn relative_yaw(&self) -> Option<Angle> {
        match self {
            Self::RelativeYaw(yaw) => Some(*yaw),
            _ => None,
        }
    }

    pub fn absolute_yaw(&self) -> Option<Angle> {
        match self {
            Self::AbsoluteYaw(yaw) => Some(*yaw),
            _ => None,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::None => "None",
            Self::Forwards => "Forwards",
            Self::Backwards => "Backwards",
            Self::RelativeYaw(_) => "Relative Yaw",
            Self::AbsoluteYaw(_) => "Absolute Yaw",
        }
    }
}

impl Default for OrientationConstraint {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature="bevy", derive(Component))]
pub enum ReverseLane {
    Same,
    Disable,
    Different(Motion),
}

impl ReverseLane {
    pub fn label(&self) -> &str {
        match self {
            Self::Same => "Same",
            Self::Disable => "Disabled",
            Self::Different(_) => "Different",
        }
    }

    pub fn different_motion(&self) -> Option<&Motion> {
        match self {
            Self::Different(motion) => Some(motion),
            _ => None,
        }
    }
}

impl Default for ReverseLane {
    fn default() -> Self {
        ReverseLane::Same
    }
}


#[derive(Clone, Debug, Default)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct RecallReverseLane {
    pub motion: Option<Motion>,
    pub previous: RecallMotion,
}

impl Recall for RecallReverseLane {
    type Source = ReverseLane;
    fn remember(&mut self, from_reverse: &ReverseLane) {
        match from_reverse {
            ReverseLane::Different(from_motion) => {
                self.motion = Some(from_motion.clone());
                self.previous.remember(from_motion);
            },
            _ => {
                // Do nothing
            }
        }
    }
}


#[cfg(feature="bevy")]
impl Lane<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Lane<Entity> {
        Lane{
            anchors: self.anchors.to_ecs(id_to_entity),
            forward: self.forward.clone(),
            reverse: self.reverse.clone(),
            marker: Default::default(),
        }
    }
}

impl<T: RefTrait> From<Edge<T>> for Lane<T> {
    fn from(edge: Edge<T>) -> Self {
        Lane{
            anchors: edge,
            forward: Default::default(),
            reverse: Default::default(),
            marker: Default::default(),
        }
    }
}
