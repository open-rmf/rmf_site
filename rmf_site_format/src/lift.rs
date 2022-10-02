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
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::{
    prelude::{Component, Entity, Bundle, Deref, DerefMut},
    render::primitives::Aabb,
    math::Vec3A,
};

pub const DEFAULT_CABIN_WALL_THICKNESS: f32 = 0.05;
pub const DEFAULT_CABIN_GAP: f32 = 0.01;
pub const DEFAULT_CABIN_WIDTH: f32 = 1.5;
pub const DEFAULT_CABIN_DEPTH: f32 = 1.65;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lift<T: RefTrait> {
    pub properties: LiftProperties<T>,
    /// Anchors that are inside the cabin of the lift and exist in the map of
    /// the cabin's interior.
    pub cabin_anchors: BTreeMap<T, [f32; 2]>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature="bevy", derive(Bundle))]
pub struct LiftProperties<T: RefTrait> {
    /// Name of this lift. This must be unique within the site.
    pub name: NameInSite,
    /// These anchors define the canonical reference frame of the lift. Both
    /// anchors must belong to the same level.
    pub reference_anchors: Edge<T>,
    /// Description of the cabin for the lift.
    pub cabin: LiftCabin,
    /// A map from the ID of a level that this lift can visit to the door that
    /// the lift opens on that level. key: level, value: door. The lift can only
    /// visit levels that are included in this map.
    pub level_doors: LevelDoors<T>,
    /// For each level (key of the map, given as its ID in the [`Site`]::levels
    /// map), specify two anchors that correct the positioning of this lift on
    /// that level. These will act like [`Fiducial`] when the site levels are
    /// being aligned.
    ///
    /// Finalized site files should not have this field because it should become
    /// unnecessary after levels have been scaled and aligned.
    #[serde(skip_serializing_if = "Corrections::is_empty")]
    pub corrections: Corrections<T>,
    /// When this is true, the lift is only for decoration and will not be
    /// responsive during a simulation.
    pub is_static: IsStatic,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature="bevy", derive(Component))]
pub enum LiftCabin {
    /// The lift cabin is defined by some parameters.
    Params(ParameterizedLiftCabin),
    /// The model pose is relative to the center point of the two Lift anchors,
    /// with the y-axis facing the left anchor. The lift doors should open along
    /// the +/- y-axis, and agents should exit the lift along the positive x-axis.
    Model(Model),
}

impl Default for LiftCabin {
    fn default() -> Self {
        LiftCabin::Params(Default::default())
    }
}

/// A lift cabin that is defined entirely by a standard set of parameters.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ParameterizedLiftCabin {
    /// How wide is the interior of the cabin, along the axis formed by the
    /// anchor points.
    pub width: f32,
    /// How deep is the cabin, i.e. interior distance from the front wall to
    /// the back wall of the cabin.
    pub depth: f32,
    /// What type of door is attached to the cabin.
    pub door: LiftCabinDoor,
    /// How thick are the walls of the cabin. Default is 0.05m.
    #[serde(skip_serializing_if="Option::is_none")]
    pub wall_thickness: Option<f32>,
    /// How large is the gap between the line formed by the anchor points
    /// and the edge of the cabin that lines up with the door. Default is
    /// 0.01m.
    #[serde(skip_serializing_if="Option::is_none")]
    pub gap: Option<f32>,
    /// Left (positive) / right (negative) shift of the cabin, off-center
    /// from the anchor points. Default is 0.0m.
    #[serde(skip_serializing_if="Option::is_none")]
    pub shift: Option<f32>,
}

impl Default for ParameterizedLiftCabin {
    fn default() -> Self {
        Self{
            width: DEFAULT_CABIN_WIDTH,
            depth: DEFAULT_CABIN_DEPTH,
            door: LiftCabinDoor{
                width: 0.75*DEFAULT_CABIN_WIDTH,
                kind: DoorType::DoubleSliding{left_right_ratio: 0.5},
                shifted: None,
            },
            wall_thickness: None,
            gap: None,
            shift: None,
        }
    }
}

#[cfg(feature="bevy")]
impl ParameterizedLiftCabin {
    pub fn aabb(&self) -> Aabb {
        let thick = self.wall_thickness.unwrap_or(DEFAULT_CABIN_WALL_THICKNESS);
        let gap = self.gap.unwrap_or(DEFAULT_CABIN_GAP);
        Aabb{
            center: Vec3A::new(
                -self.depth/2.0 - thick - gap,
                self.shift.unwrap_or(0.),
                DEFAULT_LEVEL_HEIGHT/2.0,
            ),
            half_extents: Vec3A::new(
                self.depth/2.0,
                self.width/2.0,
                DEFAULT_LEVEL_HEIGHT/2.0,
            )
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LiftCabinDoor {
    /// How wide is the lift cabin door
    pub width: f32,
    /// What kind of door is this
    pub kind: DoorType,
    /// Shift the door off-center to the left (positive) or right (negative)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shifted: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component, Deref, DerefMut))]
pub struct LevelDoors<T: RefTrait>(pub BTreeMap<T, T>);
impl<T: RefTrait> Default for LevelDoors<T> {
    fn default() -> Self {
        LevelDoors(BTreeMap::new())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component, Deref, DerefMut))]
pub struct Corrections<T: RefTrait>(pub BTreeMap<T, Edge<T>>);
impl<T: RefTrait> Default for Corrections<T> {
    fn default() -> Self {
        Corrections(BTreeMap::new())
    }
}
impl<T: RefTrait> Corrections<T> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(feature="bevy")]
impl LiftProperties<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> LiftProperties<Entity> {
        LiftProperties{
            name: self.name.clone(),
            reference_anchors: self.reference_anchors.to_ecs(id_to_entity),
            cabin: self.cabin.clone(),
            level_doors: LevelDoors(self.level_doors.iter().map(|(level, door)| {
                (
                    *id_to_entity.get(level).unwrap(),
                    *id_to_entity.get(door).unwrap(),
                )
            }).collect()),
            corrections: Corrections(self.corrections.iter().map(|(level, edge)| {
                (
                    *id_to_entity.get(level).unwrap(),
                    edge.to_ecs(id_to_entity),
                )
            }).collect()),
            is_static: self.is_static,
        }
    }
}

impl<T: RefTrait> From<Edge<T>> for LiftProperties<T> {
    fn from(edge: Edge<T>) -> Self {
        LiftProperties{
            reference_anchors: edge,
            name: Default::default(),
            cabin: Default::default(),
            level_doors: Default::default(),
            corrections: Default::default(),
            is_static: Default::default(),
        }
    }
}
