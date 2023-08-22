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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;

use crate::*;
#[cfg(feature = "bevy")]
use bevy::ecs::system::EntityCommands;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity};
#[cfg(feature = "bevy")]
use bevy::reflect::TypeUuid;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use urdf_rs::Robot;

/// Helper structure to serialize / deserialize entities with parents
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Parented<P: RefTrait, T> {
    pub parent: P,
    #[serde(flatten)]
    pub bundle: T,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct FrameMarker;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Frame {
    #[serde(flatten)]
    pub anchor: Anchor,
    #[serde(default, skip_serializing_if = "is_default")]
    pub name: Option<NameInWorkcell>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mesh_constraint: Option<MeshConstraint<u32>>,
    #[serde(skip)]
    pub marker: FrameMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct MeshConstraint<T: RefTrait> {
    pub entity: T,
    pub element: MeshElement,
    pub relative_pose: Pose,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct NameInWorkcell(pub String);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct Mass(f32);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Inertia {
    pub ixx: f32,
    pub ixy: f32,
    pub ixz: f32,
    pub iyy: f32,
    pub iyz: f32,
    pub izz: f32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Inertial {
    pub origin: Pose,
    pub mass: Mass,
    pub inertia: Inertia,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum JointType {
    Fixed,
    Revolute,
    Prismatic,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct JointLimit {
    pub lower: f32,
    pub upper: f32,
    pub effort: f32,
    pub velocity: f32,
}

impl From<&urdf_rs::JointLimit> for JointLimit {
    fn from(limit: &urdf_rs::JointLimit) -> Self {
        Self {
            lower: limit.lower as f32,
            upper: limit.upper as f32,
            effort: limit.effort as f32,
            velocity: limit.velocity as f32,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct JointAxis([f32; 3]);

impl From<&urdf_rs::Axis> for JointAxis {
    fn from(axis: &urdf_rs::Axis) -> Self {
        Self(axis.xyz.map(|t| t as f32))
    }
}

// TODO(luca) create a to_bevy impl function to spawn the components
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Joint {
    pub name: NameInWorkcell,
    pub joint_type: JointType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<JointLimit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<JointAxis>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Geometry {
    //#[serde(flatten)]
    Primitive(MeshPrimitive),
    Mesh {
        filename: String,
        #[serde(default, skip_serializing_if = "is_default")]
        scale: Option<Vec3>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum MeshPrimitive {
    Box { size: [f32; 3] },
    Cylinder { radius: f32, length: f32 },
    Capsule { radius: f32, length: f32 },
    Sphere { radius: f32 },
}

impl MeshPrimitive {
    pub fn label(&self) -> String {
        match &self {
            MeshPrimitive::Box { .. } => "Box",
            MeshPrimitive::Cylinder { .. } => "Cylinder",
            MeshPrimitive::Capsule { .. } => "Capsule",
            MeshPrimitive::Sphere { .. } => "Sphere",
        }
        .to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallMeshPrimitive {
    pub box_size: Option<[f32; 3]>,
    pub cylinder_radius: Option<f32>,
    pub cylinder_length: Option<f32>,
    pub capsule_radius: Option<f32>,
    pub capsule_length: Option<f32>,
    pub sphere_radius: Option<f32>,
}

impl Recall for RecallMeshPrimitive {
    type Source = MeshPrimitive;

    fn remember(&mut self, source: &MeshPrimitive) {
        match source {
            MeshPrimitive::Box { size } => {
                self.box_size = Some(*size);
            }
            MeshPrimitive::Cylinder { radius, length } => {
                self.cylinder_radius = Some(*radius);
                self.cylinder_length = Some(*length);
            }
            MeshPrimitive::Capsule { radius, length } => {
                self.capsule_radius = Some(*radius);
                self.capsule_length = Some(*length);
            }
            MeshPrimitive::Sphere { radius } => {
                self.sphere_radius = Some(*radius);
            }
        }
    }
}

impl RecallMeshPrimitive {
    pub fn assume_box(&self, current: &MeshPrimitive) -> MeshPrimitive {
        MeshPrimitive::Box {
            size: self.box_size.unwrap_or_default(),
        }
    }

    pub fn assume_cylinder(&self, current: &MeshPrimitive) -> MeshPrimitive {
        MeshPrimitive::Cylinder {
            radius: self.cylinder_radius.unwrap_or_default(),
            length: self.cylinder_length.unwrap_or_default(),
        }
    }

    pub fn assume_capsule(&self, current: &MeshPrimitive) -> MeshPrimitive {
        MeshPrimitive::Capsule {
            radius: self.capsule_radius.unwrap_or_default(),
            length: self.capsule_length.unwrap_or_default(),
        }
    }

    pub fn assume_sphere(&self, current: &MeshPrimitive) -> MeshPrimitive {
        MeshPrimitive::Sphere {
            radius: self.sphere_radius.unwrap_or_default(),
        }
    }
}

impl Default for Geometry {
    fn default() -> Self {
        Geometry::Primitive(MeshPrimitive::Box { size: [0.0; 3] })
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
    pub geometry: Geometry,
    pub pose: Pose,
}

#[cfg(feature = "bevy")]
impl WorkcellModel {
    pub fn add_bevy_components(&self, mut commands: EntityCommands) {
        match &self.geometry {
            Geometry::Primitive(primitive) => {
                commands.insert((
                    primitive.clone(),
                    self.pose.clone(),
                    NameInWorkcell(self.name.clone()),
                ));
            }
            Geometry::Mesh { filename, scale } => {
                println!("Setting pose of {:?} to {:?}", filename, self.pose);
                let scale = Scale(scale.unwrap_or_default());
                // TODO(luca) Make a bundle for workcell models to avoid manual insertion here
                commands.insert((
                    NameInWorkcell(self.name.clone()),
                    AssetSource::from(filename),
                    self.pose.clone(),
                    ConstraintDependents::default(),
                    scale,
                    ModelMarker,
                ));
            }
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
    /// Site ID, used for entities to set their parent to the root workcell
    pub id: u32,
    /// Frames, key is their id, used for hierarchy
    pub frames: BTreeMap<u32, Parented<u32, Frame>>,
    /// Visuals, key is their id, used for hierarchy
    pub visuals: BTreeMap<u32, Parented<u32, WorkcellModel>>,
    /// Collisions, key is their id, used for hierarchy
    pub collisions: BTreeMap<u32, Parented<u32, WorkcellModel>>,
    /// Inertials, key is their id, used for hierarchy
    pub inertials: BTreeMap<u32, Parented<u32, Inertial>>,
    /// Joints, key is their id, used for hierarchy. They must have a frame as a parent and a frame
    /// as a child
    pub joints: BTreeMap<u32, Parented<u32, Joint>>,
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

#[cfg_attr(
    feature = "bevy",
    derive(Component, Clone, Debug, Deref, DerefMut, TypeUuid)
)]
#[cfg_attr(feature = "bevy", uuid = "fe707f9e-c6f3-11ed-afa1-0242ac120002")]
pub struct UrdfRoot(pub Robot);

// TODO(luca) feature gate urdf support
impl From<&urdf_rs::Geometry> for Geometry {
    fn from(geom: &urdf_rs::Geometry) -> Self {
        match geom {
            urdf_rs::Geometry::Box { size } => Geometry::Primitive(MeshPrimitive::Box {
                size: (**size).map(|f| f as f32),
            }),
            urdf_rs::Geometry::Cylinder { radius, length } => {
                Geometry::Primitive(MeshPrimitive::Cylinder {
                    radius: *radius as f32,
                    length: *length as f32,
                })
            }
            urdf_rs::Geometry::Capsule { radius, length } => {
                Geometry::Primitive(MeshPrimitive::Capsule {
                    radius: *radius as f32,
                    length: *length as f32,
                })
            }
            urdf_rs::Geometry::Sphere { radius } => Geometry::Primitive(MeshPrimitive::Sphere {
                radius: *radius as f32,
            }),
            urdf_rs::Geometry::Mesh { filename, scale } => {
                let scale = scale
                    .clone()
                    .and_then(|s| Some(Vec3::from_array(s.map(|v| v as f32))));
                Geometry::Mesh {
                    filename: filename.clone(),
                    scale,
                }
            }
        }
    }
}

impl From<&urdf_rs::Inertia> for Inertia {
    fn from(inertia: &urdf_rs::Inertia) -> Self {
        Self {
            ixx: inertia.ixx as f32,
            ixy: inertia.ixy as f32,
            ixz: inertia.ixz as f32,
            iyy: inertia.iyy as f32,
            iyz: inertia.iyz as f32,
            izz: inertia.izz as f32,
        }
    }
}

impl From<&urdf_rs::Inertial> for Inertial {
    fn from(inertial: &urdf_rs::Inertial) -> Self {
        Self {
            origin: Pose {
                trans: inertial.origin.xyz.0.map(|v| v as f32),
                rot: Rotation::EulerExtrinsicXYZ(inertial.origin.rpy.map(|v| Angle::Rad(v as f32))),
            },
            mass: Mass(inertial.mass.value as f32),
            inertia: (&inertial.inertia).into(),
        }
    }
}

impl WorkcellModel {
    fn from_urdf_data(
        pose: &urdf_rs::Pose,
        name: &Option<String>,
        geometry: &urdf_rs::Geometry,
    ) -> Self {
        let trans = pose.xyz.map(|t| t as f32);
        let rot = Rotation::EulerExtrinsicXYZ(pose.rpy.map(|t| Angle::Rad(t as f32)));
        WorkcellModel {
            name: name.clone().unwrap_or_default(),
            geometry: geometry.into(),
            pose: Pose { trans, rot },
        }
    }
}

impl From<&urdf_rs::Visual> for WorkcellModel {
    fn from(visual: &urdf_rs::Visual) -> Self {
        WorkcellModel::from_urdf_data(&visual.origin, &visual.name, &visual.geometry)
    }
}

impl From<&urdf_rs::Collision> for WorkcellModel {
    fn from(collision: &urdf_rs::Collision) -> Self {
        WorkcellModel::from_urdf_data(&collision.origin, &collision.name, &collision.geometry)
    }
}

impl From<&urdf_rs::Robot> for Workcell {
    fn from(urdf: &urdf_rs::Robot) -> Self {
        let mut frame_name_to_id = HashMap::new();
        let root_id = 0_u32;
        let mut cur_id = 1u32..;
        // Keep track of which frames have a parent, add the ones that don't as a root child
        let mut root_frames = HashSet::new();
        let mut frames = BTreeMap::new();
        let mut visuals = BTreeMap::new();
        let mut collisions = BTreeMap::new();
        let mut inertials = BTreeMap::new();
        let mut joints = BTreeMap::new();
        // Populate here
        for link in &urdf.links {
            let inertial = Inertial::from(&link.inertial);
            // Add a frame with the link's name, then the inertial data as a child
            let frame_id = cur_id.next().unwrap();
            let inertial_id = cur_id.next().unwrap();
            frame_name_to_id.insert(link.name.clone(), frame_id);
            root_frames.insert(frame_id);
            // Pose and parent will be overwritten by joints, if needed
            frames.insert(
                frame_id,
                Parented {
                    parent: root_id,
                    bundle: Frame {
                        anchor: Anchor::Pose3D(Pose::default()),
                        name: Some(NameInWorkcell(link.name.clone())),
                        mesh_constraint: Default::default(),
                        marker: Default::default(),
                    },
                },
            );
            inertials.insert(
                inertial_id,
                Parented {
                    parent: frame_id,
                    bundle: inertial,
                },
            );
            for visual in &link.visual {
                let model = WorkcellModel::from(visual);
                let visual_id = cur_id.next().unwrap();
                visuals.insert(
                    visual_id,
                    Parented {
                        parent: frame_id,
                        bundle: model,
                    },
                );
            }
            for collision in &link.collision {
                let model = WorkcellModel::from(collision);
                let collision_id = cur_id.next().unwrap();
                collisions.insert(
                    collision_id,
                    Parented {
                        parent: frame_id,
                        bundle: model,
                    },
                );
            }
        }
        for joint in &urdf.joints {
            // TODO(luca) should this if let failure return a broken reference error?
            if let Some(parent) = frame_name_to_id.get(&joint.parent.link) {
                if let Some(child) = frame_name_to_id.get(&joint.child.link) {
                    // In urdf, joint origin represents the coordinates of the joint in the
                    // parent frame. The child is always in the origin of the joint
                    let parent_pose = Pose {
                        trans: joint.origin.xyz.map(|t| t as f32),
                        rot: Rotation::EulerExtrinsicXYZ(
                            joint.origin.rpy.map(|t| Angle::Rad(t as f32)),
                        ),
                    };
                    let joint = match joint.joint_type {
                        urdf_rs::JointType::Revolute => Joint {
                            name: NameInWorkcell(joint.name.clone()),
                            joint_type: JointType::Revolute,
                            limit: Some((&joint.limit).into()),
                            axis: Some((&joint.axis).into()),
                        },
                        urdf_rs::JointType::Prismatic => Joint {
                            name: NameInWorkcell(joint.name.clone()),
                            joint_type: JointType::Prismatic,
                            limit: Some((&joint.limit).into()),
                            axis: Some((&joint.axis).into()),
                        },
                        urdf_rs::JointType::Fixed => Joint {
                            name: NameInWorkcell(joint.name.clone()),
                            joint_type: JointType::Prismatic,
                            limit: None,
                            axis: None,
                        },
                        _ => {
                            todo!("Unimplemented joint type {:?}", joint.joint_type);
                        }
                    };
                    let joint_id = cur_id.next().unwrap();
                    // Reassign the child parenthood and pose to the joint
                    let child_frame = frames.get_mut(child).unwrap();
                    child_frame.parent = joint_id;
                    child_frame.bundle.anchor = Anchor::Pose3D(parent_pose);
                    joints.insert(
                        joint_id,
                        Parented {
                            parent: *parent,
                            bundle: joint,
                        },
                    );
                }
            }
        }

        Workcell {
            properties: WorkcellProperties {
                name: urdf.name.clone(),
            },
            id: root_id,
            frames,
            visuals,
            collisions,
            inertials,
            joints,
        }
    }
}
