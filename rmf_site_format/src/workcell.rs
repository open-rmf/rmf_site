/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use std::collections::{BTreeMap, HashSet};
use std::io;

use glam::Vec3;
use crate::*;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity};
#[cfg(feature = "bevy")]
use bevy::reflect::TypeUuid;
#[cfg(feature = "bevy")]
use bevy::ecs::system::EntityCommands;
use serde::{Deserialize, Serialize, Serializer};
use urdf_rs::Robot;

/// Helper structure to serialize / deserialize entities with parents
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Parented<P: RefTrait, T> {
    pub parent: Option<P>,
    #[serde(flatten)]
    pub bundle: T,
}

/// Joint data, parent frame will be encoded in the frame that contains this structure, child frame
/// will be encoded in the Parented<> RefTrait at the root level
pub struct Joint {
    #[serde(rename = "type")]
    pub joint_type: JointType,
    pub axis: JointAxis,
    pub limits: JointLimits,
}

pub enum FrameType {
    /// Just an empty frame, used to mark locations
    Empty,
    /// The frame is a joint, it will contain the joint that connects it to another frame
    Joint(Joint),
    /// The frame is a link, it will contain a link with inertial, visual and collision data
    Link(Link),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Frame {
    #[serde(flatten)]
    pub anchor: Anchor,
    pub mesh_constraint: Option<MeshConstraint<u32>>,
    pub name: NameInWorkcell,
    #[serde(flatten)]
    pub frame_type: FrameType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct MeshConstraint<T: RefTrait> {
    pub entity: T,
    pub element: MeshElement,
    pub relative_pose: Pose,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Inertial {
    /// Pose of the inertial element relataive to the origin of the link
    pub origin: Pose,
    pub mass: Mass,
    pub inertia: Inertia,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Link {
    pub name: NameInWorkcell,
    pub inertial: Inertial,
    pub visuals: Vec<WorkcellVisual>,
    pub collisions: Vec<WorkcellCollision>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MeshElement {
    Vertex(u32),
    // TODO(luca) edge and vertices
}

/// Attached to Model entities to keep track of constraints attached to them,
/// for change detection and hierarchy propagation
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct ConstraintDependents(pub HashSet<Entity>);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct WorkcellProperties {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct NameInWorkcell(pub String);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct Mass(f32);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Inertia {
    ixx: f32,
    ixy: f32,
    ixz: f32,
    iyy: f32,
    iyz: f32,
    izz: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Geometry {
    //#[serde(flatten)]
    Primitive(MeshPrimitive),
    Mesh{filename: String},
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum MeshPrimitive {
    Box{size: [f32; 3]},
    Cylinder{radius: f32, length: f32},
    Capsule{radius: f32, length: f32},
    Sphere{radius: f32},
}

impl Default for Geometry {
    fn default() -> Self {
        Geometry::Primitive(MeshPrimitive::Box{size: [0.0; 3]})
    }
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct WorkcellVisualMarker;

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct WorkcellCollisionMarker;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct WorkcellModel {
    pub name: String,
    pub id: u32,
    pub geometry: Geometry,
    pub pose: Pose,
}

#[cfg(feature = "bevy")]
impl WorkcellModel {
    // TODO(luca) An API to mark it as a visual or collision, inserting the correct marker
    // component maybe making WorkcellModel generic?
    pub fn add_bevy_components(&self, mut commands: EntityCommands) {
        match &self.geometry {
            Geometry::Primitive(primitive) => {
                commands.insert((primitive.clone(), self.pose.clone(), NameInSite(self.name.clone())));
            },
            Geometry::Mesh{filename} => {
                println!("Setting pose of {:?} to {:?}", filename, self.pose);
                commands.insert(Model {
                    // TODO(luca) move away from NameInSite and using NameInWorkcell? Also will
                    // mean moving away from Model bundle
                    name: NameInSite(self.name.clone()),
                    source: AssetSource::from(filename),
                    pose: self.pose.clone(),
                    // TODO*luca) parametrize is_static, default false for visuals and true for
                    // collisions
                    is_static: misc::IsStatic(false),
                    constraints: ConstraintDependents::default(),
                    marker: ModelMarker,
                });
            },
        }
    }
}

// TODO(luca) we might need a different bundle to denote a workcell included in site
// editor mode to deal with serde of workcells there (that would only have an asset source?)
/// Container for serialization / deserialization of workcells
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Workcell {
    /// Workcell specific properties
    #[serde(flatten)]
    pub properties: WorkcellProperties,
    /// Workcell ID, used for entities to set their parent to the root workcell
    pub id: u32,
    /// Frames, key is their id
    pub frames: BTreeMap<u32, Parented<u32, Frame>>,
}

impl Workcell {
    pub fn to_writer<W: io::Write>(&self, writer: W) -> serde_json::Result<()> {
        serde_json::ser::to_writer_pretty(writer, self)
    }

    pub fn to_string(&self) -> serde_json::Result<String> {
        serde_json::ser::to_string_pretty(self)
    }

    pub fn from_reader<R: io::Read>(reader: R) -> serde_json::Result<Self> {
        serde_json::de::from_reader(reader)
    }

    pub fn from_str<'a>(s: &'a str) -> serde_json::Result<Self> {
        serde_json::de::from_str(s)
    }

    pub fn from_bytes<'a>(s: &'a [u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(s)
    }
}

#[cfg_attr(feature = "bevy", derive(Component, Clone, Debug, Deref, DerefMut, TypeUuid))]
#[cfg_attr(feature = "bevy", uuid = "fe707f9e-c6f3-11ed-afa1-0242ac120002")]
pub struct UrdfRoot(pub Robot);

// TODO(luca) feature gate urdf support
impl From::<&urdf_rs::Geometry> for Geometry {
    fn from(geom: &urdf_rs::Geometry) -> Self {
        match geom {
            urdf_rs::Geometry::Box(urdf_rs::BoxGeometry {size}) => Geometry::Primitive(MeshPrimitive::Box{size: (**size).map(|f| f as f32)}),
            urdf_rs::Geometry::Cylinder(urdf_rs::CylinderGeometry {radius, length}) => Geometry::Primitive(MeshPrimitive::Cylinder{radius: *radius as f32, length: *length as f32}),
            urdf_rs::Geometry::Capsule(urdf_rs::CapsuleGeometry {radius, length}) => Geometry::Primitive(MeshPrimitive::Capsule{radius: *radius as f32, length: *length as f32}),
            urdf_rs::Geometry::Sphere(urdf_rs::SphereGeometry {radius}) => Geometry::Primitive(MeshPrimitive::Sphere{radius: *radius as f32}),
            // TODO(luca) mesh scale support
            urdf_rs::Geometry::Mesh(urdf_rs::MeshGeometry {filename, ..}) => Geometry::Mesh{filename: filename.clone()},
        }
    }
}

impl From::<&urdf_rs::Link> for Link {
    fn from(link: &urdf_rs::Link) -> Self {
        Self {
            name: NameInWorkcell(link.name.clone()),
            inertial: Inertial {
                origin: Pose {
                    trans: link.inertial.origin.xyz.0.map(|v| v as f32),
                    rot: Rotation::EulerExtrinsicXYZ(link.inertial.origin.rpy.map(|v| Angle::Rad(v as f32))),
                },
                mass: Mass(link.inertial.mass.value as f32),
                inertia: Inertia::default(),
            },
        }
    }
}
