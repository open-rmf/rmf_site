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
pub struct Lane<T: SiteID> {
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
    #[serde(skip_serializing_if="Option::is_none")]
    pub orientation_constraint: Option<OrientationConstraint>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub speed_limit: Option<f32>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub dock: Option<Dock>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum OrientationConstraint {
    Forward,
    Reverse,
    RelativeYaw(f32),
    AbsoluteYaw(f32),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature="bevy", derive(Component))]
pub enum ReverseLane {
    Same,
    Disable,
    Different(Motion),
}

impl Default for ReverseLane {
    fn default() -> Self {
        ReverseLane::Same
    }
}

#[cfg(feature="bevy")]
impl Lane<Entity> {
    pub fn to_u32(&self, anchors: Edge<u32>) -> Lane<u32> {
        Lane{
            anchors,
            forward: self.forward.clone(),
            reverse: self.reverse.clone(),
            marker: Default::default(),
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

impl<T: SiteID> From<Edge<T>> for Lane<T> {
    fn from(edge: Edge<T>) -> Self {
        Lane{
            anchors: edge,
            forward: Default::default(),
            reverse: Default::default(),
            marker: Default::default(),
        }
    }
}
