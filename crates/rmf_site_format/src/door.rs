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
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Entity, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_DOOR_THICKNESS: f32 = 0.05;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Door {
    /// (left_anchor, right_anchor)
    pub anchors: Edge,
    /// Name of the door. RMF requires each door name to be unique among all
    /// doors in the site.
    pub name: NameInSite,
    /// What kind of door is it.
    pub kind: DoorType,
    #[serde(skip)]
    pub marker: DoorMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum DoorType {
    SingleSliding(SingleSlidingDoor),
    DoubleSliding(DoubleSlidingDoor),
    SingleSwing(SingleSwingDoor),
    DoubleSwing(DoubleSwingDoor),
    /// A custom model for the door. The reference frame for the model will be
    /// the center point of the two anchor points with the y axis facing the
    /// left anchor point and the z axis pointing upwards. The model must have
    /// its own door control plugin.
    Model(Model),
}

impl DoorType {
    pub fn single_sliding(&self) -> Option<&SingleSlidingDoor> {
        match self {
            Self::SingleSliding(v) => Some(v),
            _ => None,
        }
    }

    pub fn double_sliding(&self) -> Option<&DoubleSlidingDoor> {
        match self {
            Self::DoubleSliding(v) => Some(v),
            _ => None,
        }
    }

    pub fn single_swing(&self) -> Option<&SingleSwingDoor> {
        match self {
            Self::SingleSwing(v) => Some(v),
            _ => None,
        }
    }

    pub fn double_swing(&self) -> Option<&DoubleSwingDoor> {
        match self {
            Self::DoubleSwing(v) => Some(v),
            _ => None,
        }
    }

    pub fn model(&self) -> Option<&Model> {
        match self {
            Self::Model(v) => Some(v),
            _ => None,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::SingleSliding(_) => "Single Sliding",
            Self::DoubleSliding(_) => "Double Sliding",
            Self::SingleSwing(_) => "Single Swing",
            Self::DoubleSwing(_) => "Double Swing",
            Self::Model(_) => "Model",
        }
    }
}

impl Default for DoorType {
    fn default() -> Self {
        SingleSlidingDoor::default().into()
    }
}

/// A single door that slides to one side
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct SingleSlidingDoor {
    /// Which side the door slides towards
    pub towards: Side,
}

impl Default for SingleSlidingDoor {
    fn default() -> Self {
        Self {
            towards: Side::Left,
        }
    }
}

impl From<SingleSlidingDoor> for DoorType {
    fn from(v: SingleSlidingDoor) -> Self {
        Self::SingleSliding(v)
    }
}

/// Two doors that each slide towards their own side
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct DoubleSlidingDoor {
    /// Length of the left door divided by the length of the right door
    pub left_right_ratio: f32,
}

impl DoubleSlidingDoor {
    /// Get the offset from the door center of the point where the doors
    /// separate. A value of 0.0 means the doors are even. A negative value
    /// means the left door is smaller while a positive value means the right
    /// door is smaller.
    pub fn compute_offset(&self, door_width: f32) -> f32 {
        let l = self.left_right_ratio * door_width / (self.left_right_ratio + 1.0);
        return door_width / 2.0 - l;
    }
}

impl Default for DoubleSlidingDoor {
    fn default() -> Self {
        Self {
            left_right_ratio: 1.0,
        }
    }
}

impl From<DoubleSlidingDoor> for DoorType {
    fn from(v: DoubleSlidingDoor) -> Self {
        Self::DoubleSliding(v)
    }
}

/// A single door that swings along a pivot on the left or right side
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct SingleSwingDoor {
    /// Which anchor (Left or Right) does the door pivot on
    pub pivot_on: Side,
    /// How does the door swing
    pub swing: Swing,
}

impl Default for SingleSwingDoor {
    fn default() -> Self {
        Self {
            pivot_on: Side::Left,
            swing: Swing::Forward(Angle::Deg(90.0)),
        }
    }
}

impl From<SingleSwingDoor> for DoorType {
    fn from(v: SingleSwingDoor) -> Self {
        Self::SingleSwing(v)
    }
}

/// Two doors, one left and one right, that each swing on their own pivot.
/// It is assumed their swinging parameters are symmetrical.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct DoubleSwingDoor {
    pub swing: Swing,
    /// Length of the left door divided by the length of the right door
    pub left_right_ratio: f32,
}

impl DoubleSwingDoor {
    /// Get the offset from the door center of the point where the doors
    /// separate. A value of 0.0 means the doors are even. A negative value
    /// means the left door is smaller while a positive value means the right
    /// door is smaller.
    pub fn compute_offset(&self, door_width: f32) -> f32 {
        let l = self.left_right_ratio * door_width / (self.left_right_ratio + 1.0);
        return door_width / 2.0 - l;
    }
}

impl Default for DoubleSwingDoor {
    fn default() -> Self {
        Self {
            swing: Swing::Forward(Angle::Deg(90.0)),
            left_right_ratio: 1.0,
        }
    }
}

impl From<DoubleSwingDoor> for DoorType {
    fn from(v: DoubleSwingDoor) -> Self {
        Self::DoubleSwing(v)
    }
}

impl From<Model> for DoorType {
    fn from(v: Model) -> Self {
        Self::Model(v)
    }
}

/// How the door swings relative to someone who is standing in the frame of door
/// with the left and right sides of their body aligned with the left and right
/// anchor points of the door.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub enum Swing {
    /// Swing forwards up to this (positive) angle
    Forward(Angle),
    /// Swing backwards up to this (positive) angle
    Backward(Angle),
    /// Swing each direction up to the given (positive) angle.
    Both { forward: Angle, backward: Angle },
}

impl Swing {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Forward(_) => "Forward",
            Self::Backward(_) => "Backward",
            Self::Both { .. } => "Both",
        }
    }

    /// Given which side will be pivoted on, this function gives back
    /// 0. The initial angle of the swing
    /// 1. The angle that will be swept by the swing, relative to the initial angle
    /// For Both, the initial angle will be the backward angle and the sweep
    /// will reach the forward angle.
    pub fn swing_on_pivot(&self, pivot_on: Side) -> (Angle, Angle) {
        let pivot_sign = pivot_on.sign();
        let closed_angle = pivot_on.pivot_closed_angle();
        match self {
            Self::Forward(sweep) => (closed_angle, pivot_sign * *sweep),
            Self::Backward(sweep) => (closed_angle, -pivot_sign * *sweep),
            Self::Both { forward, backward } => (
                closed_angle - pivot_sign * *backward,
                pivot_sign * (*forward + *backward),
            ),
        }
    }

    pub fn assume_forward(&self) -> Self {
        match self {
            Self::Forward(angle) => Self::Forward(*angle),
            Self::Backward(angle) => Self::Forward(*angle),
            Self::Both { forward, .. } => Self::Forward(*forward),
        }
    }

    pub fn assume_backward(&self) -> Self {
        match self {
            Self::Forward(angle) => Self::Backward(*angle),
            Self::Backward(angle) => Self::Backward(*angle),
            Self::Both { backward, .. } => Self::Backward(*backward),
        }
    }

    pub fn assume_both(&self) -> Self {
        match self {
            Self::Forward(angle) => Self::Both {
                forward: *angle,
                backward: *angle,
            },
            Self::Backward(angle) => Self::Both {
                forward: *angle,
                backward: *angle,
            },
            Self::Both { forward, backward } => Self::Both {
                forward: *forward,
                backward: *backward,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct DoorMarker;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallDoorType {
    pub single_sliding: Option<DoorType>,
    pub double_sliding: Option<DoorType>,
    pub single_swing: Option<DoorType>,
    pub double_swing: Option<DoorType>,
    pub model: Option<DoorType>,
}

impl RecallDoorType {
    pub fn assume_single_sliding(&self, current: &DoorType) -> DoorType {
        current
            .single_sliding()
            .map(|x| x.clone().into())
            .unwrap_or(
                self.single_sliding
                    .as_ref()
                    .map(|x| x.clone())
                    .unwrap_or(SingleSlidingDoor::default().into()),
            )
    }

    pub fn assume_double_sliding(&self, current: &DoorType) -> DoorType {
        current
            .double_sliding()
            .map(|x| x.clone().into())
            .unwrap_or(
                self.double_sliding
                    .as_ref()
                    .map(|x| x.clone())
                    .unwrap_or(DoubleSlidingDoor::default().into()),
            )
    }

    pub fn assume_single_swing(&self, current: &DoorType) -> DoorType {
        current.single_swing().map(|x| x.clone().into()).unwrap_or(
            self.single_swing
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or(SingleSwingDoor::default().into()),
        )
    }

    pub fn assume_double_swing(&self, current: &DoorType) -> DoorType {
        current.double_swing().map(|x| x.clone().into()).unwrap_or(
            self.double_swing
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or(DoubleSwingDoor::default().into()),
        )
    }

    pub fn assume_model(&self, current: &DoorType) -> DoorType {
        current.model().map(|x| x.clone().into()).unwrap_or(
            self.model
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or(DoorType::Model(Model::default())),
        )
    }
}

impl Recall for RecallDoorType {
    type Source = DoorType;

    fn remember(&mut self, source: &Self::Source) {
        match source {
            DoorType::SingleSliding(_) => {
                self.single_sliding = Some(source.clone());
            }
            DoorType::DoubleSliding { .. } => {
                self.double_sliding = Some(source.clone());
            }
            DoorType::SingleSwing { .. } => {
                self.single_swing = Some(source.clone());
            }
            DoorType::DoubleSwing(_) => {
                self.double_swing = Some(source.clone());
            }
            DoorType::Model(_) => {
                self.model = Some(source.clone());
            }
        }
    }
}

impl Door {
    pub fn convert(&self, id_map: &HashMap<Entity, Entity>) -> Result<Door, Entity> {
        Ok(Door {
            anchors: self.anchors.convert(id_map)?,
            name: self.name.clone(),
            kind: self.kind.clone(),
            marker: Default::default(),
        })
    }
}

impl From<Edge> for Door {
    fn from(edge: Edge) -> Self {
        Door {
            anchors: edge,
            name: NameInSite("<Unnamed>".to_string()),
            kind: SingleSlidingDoor::default().into(),
            marker: Default::default(),
        }
    }
}
