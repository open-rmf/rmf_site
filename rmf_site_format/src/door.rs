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
use bevy::prelude::Component;

/// How the door swings relative to someone who is standing in the frame of door
/// with the left and right sides of their body aligned with the left and right
/// anchor points of the door.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Swing {
    /// Swing forwards up to this (positive) angle
    Forward(Angle),
    /// Swing backwards up to this (positive) angle
    Backward(Angle),
    /// Swing each direction by (forward, backward) positive degrees.
    Both(Angle, Angle),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DoorType {
    /// A single door that slides to one side
    SingleSliding(Side),
    /// Two doors that each slide towards their own side
    DoubleSliding{
        /// Length of the left door divided by the length of the right door
        left_right_ratio: f32
    },
    /// A single door that swings along a pivot on the left or right side
    SingleSwing{
        /// Which anchor (Left or Right) does the door pivot on
        pivot: Side,
        /// How does the door swing
        swing: Swing
    },
    /// Two doors, one left and one right, that each swing on their own pivot.
    /// It is assumed their swinging parameters are symmetrical.
    DoubleSwing(Swing),
    /// A custom model for the door. The reference frame for the model will be
    /// the center point of the two anchor points with the y axis facing the
    /// left anchor point and the z axis pointing upwards. The model must have
    /// its own door control plugin.
    Model(Model),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Door<SiteID> {
    /// (left_anchor, right_anchor)
    pub anchors: (SiteID, SiteID),
    /// Name of the door. RMF requires each door name to be unique among all
    /// doors in the site.
    pub name: String,
    /// What kind of door is it.
    pub kind: DoorType,
}
