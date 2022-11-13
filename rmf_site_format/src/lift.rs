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
use bevy::{
    math::{Vec2, Vec3, Vec3A},
    prelude::{Bundle, Component, Deref, DerefMut, Entity, Query, With, Without},
    render::primitives::Aabb,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_CABIN_WALL_THICKNESS: f32 = 0.1;
pub const DEFAULT_CABIN_DOOR_THICKNESS: f32 = 0.05;
pub const DEFAULT_CABIN_GAP: f32 = 0.01;
pub const DEFAULT_CABIN_WIDTH: f32 = 1.5;
pub const DEFAULT_CABIN_DEPTH: f32 = 1.65;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lift<T: RefTrait> {
    /// The cabin doors that the lift cabin has
    pub cabin_doors: BTreeMap<T, LiftCabinDoor<T>>,
    /// Properties that define the lift
    pub properties: LiftProperties<T>,
    /// Anchors that are inside the cabin of the lift and exist in the map of
    /// the cabin's interior.
    pub cabin_anchors: BTreeMap<T, Anchor>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct LiftCabinDoor<T: RefTrait> {
    /// What kind of door is this
    pub kind: DoorType,
    /// Anchors that define the level door positioning for level doors.
    /// The key of this map is the cabin door ID and the value is a pair of
    /// anchor IDs associated with that cabin door, used to mark the location of
    /// where a level door is (or would be) located.
    pub reference_anchors: Edge<T>,
    #[serde(skip)]
    pub marker: LiftCabinDoorMarker,
}

#[cfg(feature = "bevy")]
impl LiftCabinDoor<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LiftCabinDoor<Entity> {
        LiftCabinDoor {
            kind: self.kind.clone(),
            reference_anchors: self.reference_anchors.to_ecs(id_to_entity),
            marker: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct LiftCabinDoorMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct LiftProperties<T: RefTrait> {
    /// Name of this lift. This must be unique within the site.
    pub name: NameInSite,
    /// These anchors define the canonical reference frame of the lift. Both
    /// anchors must be site-wide anchors.
    pub reference_anchors: Edge<T>,
    /// Description of the cabin for the lift.
    pub cabin: LiftCabin<T>,
    /// Descriptions of the doors used at each level
    pub level_doors: LevelDoors<T>,
    /// When this is true, the lift is only for decoration and will not be
    /// responsive during a simulation.
    pub is_static: IsStatic,
    /// What is the initial level for this lift. If nothing is specified, the
    /// lift will start on the lowest level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_level: InitialLevel<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct InitialLevel<T: RefTrait>(pub Option<T>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct LevelDoors<T: RefTrait> {
    /// A map from the ID of a level that this lift can visit to the door(s) that
    /// the lift opens on that level. key: level, value: door. The lift can only
    /// visit levels that are included in this map.
    pub visit: BTreeMap<T, BTreeSet<T>>,
}

impl<T: RefTrait> Default for LevelDoors<T> {
    fn default() -> Self {
        Self {
            visit: Default::default(),
        }
    }
}

#[cfg(feature="bevy")]
impl LevelDoors<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LevelDoors<Entity> {
        LevelDoors {
            visit: self.visit.iter().map(|(level, doors)| {
                (
                    *id_to_entity.get(level).unwrap(),
                    doors.iter().map(|door| id_to_entity.get(door).unwrap()).copied().collect()
                )
            }).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum LiftCabin<T: RefTrait> {
    /// The lift cabin is defined by some parameters.
    Rect(RectangularLiftCabin<T>),
    // TODO(MXG): Support Models as lift cabins
    // The model pose is relative to the center point of the two Lift anchors,
    // with the y-axis facing the left anchor. The lift doors should open along
    // the +/- y-axis, and agents should exit the lift along the positive x-axis.
    // Model(Model),
}

impl<T: RefTrait> Default for LiftCabin<T> {
    fn default() -> Self {
        LiftCabin::Rect(Default::default())
    }
}

impl<T: RefTrait> LiftCabin<T> {
    pub fn remove_door(&mut self, door: T) {
        match self {
            Self::Rect(params) => {
                for face in RectFace::iter_all() {
                    let placement = params.door_mut(face);
                    if placement.filter(|p| p.door == door).is_some() {
                        *placement = None;
                        break;
                    }
                }
            }
        }
    }

    pub fn level_door_anchors(&self, door: T) -> Option<[Anchor; 2]> {
        match self {
            Self::Rect(params) => {
                for (face, placement) in &params.doors() {
                    if placement.filter(|p| p.door == door).is_some() {
                        return params.level_door_anchors(*face);
                    }
                }
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallLiftCabin<T: RefTrait> {
    pub rect_doors: [Option<LiftCabinDoorPlacement<T>>; 4],
}

impl<T: RefTrait> Default for RecallLiftCabin<T> {
    fn default() -> Self {
        Self { rect_doors: Default::default() }
    }
}

impl<T: RefTrait> Recall for RecallLiftCabin<T> {
    type Source = LiftCabin<T>;

    fn remember(&mut self, source: &Self::Source) {
        match source {
            LiftCabin::Rect(params) => {
                for (face, door) in params.doors() {
                    if let Some(door) = door {
                        self.rect_doors[face as usize] = Some(*door);
                    }
                }
            }
        }
    }
}

impl<T: RefTrait> RecallLiftCabin<T> {
    pub fn rect_door(&self, face: RectFace) -> &Option<LiftCabinDoorPlacement<T>> {
        &self.rect_doors[face as usize]
    }
}

/// A lift cabin that is defined entirely by a standard set of parameters.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RectangularLiftCabin<T: RefTrait> {
    /// How wide is the interior of the cabin, along the axis formed by the
    /// anchor points.
    pub width: f32,
    /// How deep is the cabin, i.e. interior distance from the front wall to
    /// the back wall of the cabin.
    pub depth: f32,
    /// How thick are the walls of the cabin. Default is DEFAULT_CABIN_WALL_THICKNESS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wall_thickness: Option<f32>,
    /// How large is the gap between the line formed by the anchor points
    /// and the edge of the cabin that lines up with the door. Default is
    /// 0.01m.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f32>,
    /// Left (positive) / right (negative) shift of the cabin, off-center
    /// from the anchor points. Default is 0.0m.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift: Option<f32>,
    // NOTE(MXG): We explicitly list out the four doors instead of using an
    // array so the serialization looks nicer. Use doors() to get these fields
    // as an array.
    /// The placement of the cabin's front door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_door: Option<LiftCabinDoorPlacement<T>>,
    /// The placement of the cabin's back door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub back_door: Option<LiftCabinDoorPlacement<T>>,
    /// The placement of the cabin's left door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_door: Option<LiftCabinDoorPlacement<T>>,
    /// The placement of the cabin's right door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_door: Option<LiftCabinDoorPlacement<T>>,
}

impl<T: RefTrait> Default for RectangularLiftCabin<T> {
    fn default() -> Self {
        Self {
            width: DEFAULT_CABIN_WIDTH,
            depth: DEFAULT_CABIN_DEPTH,
            wall_thickness: None,
            gap: None,
            shift: None,
            front_door: None,
            back_door: None,
            left_door: None,
            right_door: None,
        }
    }
}

impl<T: RefTrait> RectangularLiftCabin<T> {
    pub fn thickness(&self) -> f32 {
        self.wall_thickness.unwrap_or(DEFAULT_CABIN_WALL_THICKNESS)
    }

    pub fn gap(&self) -> f32 {
        self.gap.unwrap_or(DEFAULT_CABIN_GAP)
    }

    pub fn shift(&self) -> f32 {
        self.shift.unwrap_or(0.0)
    }

    pub fn face_size(&self, face: RectFace) -> f32 {
        match face {
            RectFace::Front | RectFace::Back => self.width,
            RectFace::Left | RectFace::Right => self.depth,
        }
    }

    pub fn doors(&self) -> [(RectFace, &Option<LiftCabinDoorPlacement<T>>); 4] {
        [
            (RectFace::Front, &self.front_door),
            (RectFace::Back, &self.back_door),
            (RectFace::Left, &self.left_door),
            (RectFace::Right, &self.right_door),
        ]
    }

    pub fn door(&self, face: RectFace) -> &Option<LiftCabinDoorPlacement<T>> {
        match face {
            RectFace::Front => &self.front_door,
            RectFace::Back => &self.back_door,
            RectFace::Left => &self.left_door,
            RectFace::Right => &self.right_door,
        }
    }

    pub fn door_mut(&mut self, face: RectFace) -> &mut Option<LiftCabinDoorPlacement<T>> {
        match face {
            RectFace::Front => &mut self.front_door,
            RectFace::Back => &mut self.back_door,
            RectFace::Left => &mut self.left_door,
            RectFace::Right => &mut self.right_door,
        }
    }

    pub fn cabin_wall_coordinates(&self) -> Vec<[Vec3; 2]> {
        let n = Vec3::new(
            self.depth/2.0 + self.thickness()/2.0,
            self.width/2.0 + self.thickness()/2.0,
            0.0,
        );
        self.doors().into_iter().flat_map(
            |(face, params)| {
                let (u, v) = face.uv();
                let du = n.dot(u).abs();
                let dv = n.dot(v).abs() + self.thickness()/2.0;
                let start = u*du + v*dv;
                let end = u*du - v*dv;
                if let Some(params) = params {
                    let door_left = u*du + params.left_coordinate()*v;
                    let door_right = u*du + params.right_coordinate()*v;
                    vec![[start, door_left], [door_right, end]]
                } else {
                    vec![[start, end]]
                }
            }
        ).collect()
    }

    pub fn level_door_anchors(&self, face: RectFace) -> Option<[Anchor; 2]> {
        let door = self.door(face).as_ref()?;
        let (u, v) = face.uv2();
        let n = Vec2::new(self.depth/2.0, self.width/2.0);
        let delta = self.thickness() + door.custom_gap.unwrap_or(self.gap()) + door.thickness();
        let base = (n.dot(u).abs() + delta) * u;
        let left = base + door.left_coordinate() * v;
        let right = base + door.right_coordinate() * v;
        let d_floor = door.thickness() * u;
        Some([
            Anchor::CategorizedTranslate2D(
                Categorized::new(left.into())
                .with_category(Category::Floor, (left - d_floor).into())
            ),
            Anchor::CategorizedTranslate2D(
                Categorized::new(right.into())
                .with_category(Category::Floor, (right - d_floor).into())
            )
        ])
    }
}

#[cfg(feature = "bevy")]
impl<T: RefTrait> RectangularLiftCabin<T> {
    pub fn aabb(&self) -> Aabb {
        Aabb {
            center: Vec3A::new(
                -self.depth / 2.0 - self.thickness() - self.gap(),
                self.shift(),
                DEFAULT_LEVEL_HEIGHT / 2.0,
            ),
            half_extents: Vec3A::new(
                self.depth / 2.0,
                self.width / 2.0,
                DEFAULT_LEVEL_HEIGHT / 2.0,
            ),
        }
    }

    /// This gives a set of "placemats" that can be laid in front of each lift
    /// cabin door to be used as visual cues.
    pub fn level_door_placemats(
        &self,
        length: f32,
        recall: Option<&RecallLiftCabin<T>>,
    ) -> [(RectFace, Option<T>, Aabb); 4] {
        let n = Vec3::new(
            self.depth/2.0 + 1.5*self.thickness() + length/2.0,
            self.width/2.0 + 1.5*self.thickness() + length/2.0,
            0.0,
        );
        self.doors().map(|(face, params)| {
            let params = params.as_ref().or(recall.map(|r| r.rect_door(face).as_ref()).flatten());
            let (u, v) = face.uv();
            let gap = params.map(|p| p.custom_gap).flatten().unwrap_or(self.gap());
            let du = n.dot(u).abs() + gap;
            let shift = params.map(|p| p.shifted).flatten().unwrap_or(0.0);
            let width = params.map(|p| p.width).unwrap_or(self.width/2.0);
            let aabb = Aabb {
                center: (u*du + shift*v).into(),
                half_extents: (length*u/2.0 + width*v/2.0).into(),
            };
            (face, params.map(|p| p.door), aabb)
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct LiftCabinDoorPlacement<T: RefTrait> {
    /// Reference to the actual door entity
    pub door: T,
    /// How wide is the lift cabin door
    pub width: f32,
    /// Set the thickness of the door. If set to None, 10cm will be used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thickness: Option<f32>,
    /// Shift the door off-center to the left (positive) or right (negative)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shifted: Option<f32>,
    /// Use a different gap than the one for the parent LiftCabin
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_gap: Option<f32>,
}

#[cfg(feature = "bevy")]
impl LiftProperties<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LiftProperties<Entity> {
        LiftProperties {
            name: self.name.clone(),
            reference_anchors: self.reference_anchors.to_ecs(id_to_entity),
            cabin: self.cabin.to_ecs(id_to_entity),
            level_doors: self.level_doors.to_ecs(id_to_entity),
            is_static: self.is_static,
            initial_level: InitialLevel(self.initial_level.map(
                |id| id_to_entity.get(&id).unwrap()
            ).copied()),
        }
    }
}

impl<T: RefTrait> From<Edge<T>> for LiftProperties<T> {
    fn from(edge: Edge<T>) -> Self {
        LiftProperties {
            name: Default::default(),
            reference_anchors: edge,
            cabin: LiftCabin::default(),
            level_doors: Default::default(),
            is_static: Default::default(),
            initial_level: InitialLevel(None),
        }
    }
}

#[cfg(feature = "bevy")]
impl LiftCabin<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LiftCabin<Entity> {
        match self {
            LiftCabin::Rect(cabin) => LiftCabin::Rect(cabin.to_ecs(id_to_entity)),
        }
    }
}

#[cfg(feature = "bevy")]
pub type QueryLiftDoor<'w, 's> = Query<'w, 's, (
    &'static SiteID,
    &'static DoorType,
    &'static Edge<Entity>,
    Option<&'static Original<Edge<Entity>>>
), (With<LiftCabinDoorMarker>, Without<Pending>)>;

#[cfg(feature="bevy")]
impl LiftCabin<Entity> {
    pub fn to_u32(
        &self,
        doors: &QueryLiftDoor,
    ) -> LiftCabin<u32> {
        match self {
            LiftCabin::Rect(cabin) => LiftCabin::Rect(cabin.to_u32(doors)),
        }
    }
}

#[cfg(feature = "bevy")]
impl RectangularLiftCabin<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> RectangularLiftCabin<Entity> {
        RectangularLiftCabin {
            width: self.width,
            depth: self.depth,
            wall_thickness: self.wall_thickness,
            gap: self.gap,
            shift: self.shift,
            front_door: self.front_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
            back_door: self.back_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
            left_door: self.left_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
            right_door: self.right_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
        }
    }
}

#[cfg(feature = "bevy")]
impl RectangularLiftCabin<Entity> {
    pub fn to_u32(
        &self,
        doors: &QueryLiftDoor,
    ) -> RectangularLiftCabin<u32> {
        RectangularLiftCabin {
            width: self.width,
            depth: self.depth,
            wall_thickness: self.wall_thickness,
            gap: self.gap,
            shift: self.shift,
            front_door: self.front_door.as_ref().map(|d| d.to_u32(doors)),
            back_door: self.back_door.as_ref().map(|d| d.to_u32(doors)),
            left_door: self.left_door.as_ref().map(|d| d.to_u32(doors)),
            right_door: self.right_door.as_ref().map(|d| d.to_u32(doors)),
        }
    }
}

#[cfg(feature = "bevy")]
impl LiftCabinDoorPlacement<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LiftCabinDoorPlacement<Entity> {
        LiftCabinDoorPlacement {
            door: *id_to_entity.get(&self.door).unwrap(),
            width: self.width,
            thickness: self.thickness,
            shifted: self.shifted,
            custom_gap: self.custom_gap,
        }
    }
}

#[cfg(feature = "bevy")]
impl LiftCabinDoorPlacement<Entity> {
    pub fn to_u32(
        &self,
        doors: &QueryLiftDoor,
    ) -> LiftCabinDoorPlacement<u32> {
        LiftCabinDoorPlacement {
            door: doors.get(self.door).unwrap().0.0,
            width: self.width,
            thickness: self.thickness,
            shifted: self.shifted,
            custom_gap: self.custom_gap,
        }
    }
}

impl<T: RefTrait> LiftCabinDoorPlacement<T> {
    pub fn new(door: T, width: f32) -> Self {
        LiftCabinDoorPlacement {
            door,
            width,
            thickness: None,
            shifted: None,
            custom_gap: None
        }
    }

    pub fn left_coordinate(&self) -> f32 {
        self.width/2.0 + self.shifted.unwrap_or(0.0)
    }

    pub fn right_coordinate(&self) -> f32 {
        -self.width/2.0 + self.shifted.unwrap_or(0.0)
    }

    pub fn thickness(&self) -> f32 {
        self.thickness.unwrap_or(DEFAULT_CABIN_DOOR_THICKNESS)
    }
}
