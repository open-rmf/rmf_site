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
    prelude::{Component, Entity},
    render::primitives::Aabb,
    math::Vec3A,
};

pub const DEFAULT_CABIN_WALL_THICKNESS: f32 = 0.05;
pub const DEFAULT_CABIN_GAP: f32 = 0.01;
pub const DEFAULT_CABIN_WIDTH: f32 = 1.5;
pub const DEFAULT_CABIN_DEPTH: f32 = 1.65;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Lift<SiteID: Ord> {
    /// Name of this lift. This must be unique within the site.
    pub name: String,
    /// These anchors define the canonical reference frame of the lift. Both
    /// anchors must belong to the same level.
    pub reference_anchors: (SiteID, SiteID),
    /// Description of the cabin for the lift.
    pub cabin: LiftCabin,
    /// Anchors that are inside the cabin of the lift and exist in the map of
    /// the cabin's interior.
    pub cabin_anchors: BTreeMap<SiteID, (f32, f32)>,
    /// A map from the ID of a level that this lift can visit to the door that
    /// the lift opens on that level. key: level, value: door. The lift can only
    /// visit levels that are included in this map.
    pub level_doors: BTreeMap<SiteID, SiteID>,
    /// For each level (key of the map, given as its ID in the [`Site`]::levels
    /// map), specify two anchors that correct the positioning of this lift on
    /// that level. These will act like [`Fiducial`] when the site levels are
    /// being aligned.
    ///
    /// Finalized site files should not have this field because it should become
    /// unnecessary after levels have been scaled and aligned.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub corrections: BTreeMap<SiteID, (SiteID, SiteID)>,
    /// When this is true, the lift is only for decoration and will not be
    /// responsive during a simulation.
    pub is_static: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LiftCabin {
    /// The lift cabin is defined by some parameters.
    Params(ParameterizedLiftCabin),
    /// The model pose is relative to the center point of the two Lift anchors,
    /// with the y-axis facing the left anchor. The lift doors should open along
    /// the +/- y-axis, and agents should exit the lift along the positive x-axis.
    Model(Model),
}

/// A lift cabin that is defined entirely by a standard set of parameters.
#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[cfg(feature="bevy")]
impl ParameterizedLiftCabin {
    pub fn aabb(&self) -> Aabb {
        let thick = self.wall_thickness.unwrap_or(DEFAULT_CABIN_WALL_THICKNESS);
        let gap = self.gap.unwrap_or(DEFAULT_CABIN_GAP);
        Aabb{
            center: Vec3A::new(
                -self.depth/2.0 - thick - gap,
                self.shift.unwrap_or(0.),
                DEFAULT_LEVEL_HEIGHT,
            ),
            half_extents: Vec3A::new(
                self.depth/2.0,
                self.width/2.0,
                DEFAULT_LEVEL_HEIGHT/2.0,
            )
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiftCabinDoor {
    /// How wide is the lift cabin door
    pub width: f32,
    /// What kind of door is this
    pub kind: DoorType,
    /// Shift the door off-center to the left (positive) or right (negative)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shifted: Option<f32>,
}

#[cfg(feature="bevy")]
impl Lift<Entity> {
    pub fn to_u32(
        &self,
        reference_anchors: (u32, u32),
        cabin_anchors: BTreeMap<u32, (f32, f32)>,
        level_doors: BTreeMap<u32, u32>,
        corrections: BTreeMap<u32, (u32, u32)>,
    ) -> Lift<u32> {
        Lift{
            reference_anchors,
            cabin_anchors,
            level_doors,
            corrections,
            name: self.name.clone(),
            cabin: self.cabin.clone(),
            is_static: self.is_static,
        }
    }
}

#[cfg(feature="bevy")]
impl Lift<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Lift<Entity> {
        Lift{
            reference_anchors: (
                *id_to_entity.get(&self.reference_anchors.0).unwrap(),
                *id_to_entity.get(&self.reference_anchors.1).unwrap(),
            ),
            corrections: self.corrections.iter().map(|(level, (a0, a1))| {
                (
                    *id_to_entity.get(level).unwrap(),
                    (
                        *id_to_entity.get(a0).unwrap(),
                        *id_to_entity.get(a1).unwrap(),
                    )
                )
            }).collect(),
            level_doors: self.level_doors.iter().map(|(level, door)| {
                (
                    *id_to_entity.get(level).unwrap(),
                    *id_to_entity.get(door).unwrap(),
                )
            }).collect(),
            // These fields will be loaded as child entities so we can leave them
            // blank here
            cabin_anchors: Default::default(),
            name: self.name.clone(),
            cabin: self.cabin.clone(),
            is_static: self.is_static,
        }
    }
}

impl<SiteID: Copy + Ord> Edge<SiteID> for Lift<SiteID> {
    fn endpoints(&self) -> (SiteID, SiteID) {
        self.reference_anchors
    }

    fn endpoints_mut(&mut self) -> (&mut SiteID, &mut SiteID) {
        (&mut self.reference_anchors.0, &mut self.reference_anchors.1)
    }

    fn new(reference_anchors: (SiteID, SiteID)) -> Self {
        Lift{
            name: "<Unnamed>".to_string(),
            reference_anchors,
            cabin: LiftCabin::Params(ParameterizedLiftCabin{
                width: DEFAULT_CABIN_WIDTH,
                depth: DEFAULT_CABIN_DEPTH,
                door: LiftCabinDoor{
                    width: 0.75*DEFAULT_CABIN_WIDTH,
                    kind: DoorType::DoubleSliding{left_right_ratio: 0.5},
                    shifted: None,
                },
                wall_thickness: None,
                gap: None,
                shift: None
            }),
            cabin_anchors: BTreeMap::new(),
            level_doors: BTreeMap::new(),
            corrections: BTreeMap::new(),
            is_static: false,
        }
    }
}
