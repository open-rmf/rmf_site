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

use crate::misc::Rotation;
use crate::*;
#[cfg(feature = "bevy")]
use bevy::ecs::system::EntityCommands;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity, SpatialBundle};
#[cfg(feature = "bevy")]
use bevy::reflect::TypeUuid;
use glam::{EulerRot, Vec3};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

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
#[cfg(feature = "bevy")]
#[derive(Component, Deref, DerefMut, Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
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
    pub center: Pose,
    pub mass: Mass,
    pub inertia: Inertia,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum JointType {
    Fixed,
    Revolute,
    Prismatic,
    Continuous,
}

impl JointType {
    pub fn label(&self) -> String {
        match &self {
            JointType::Fixed => "Fixed",
            JointType::Revolute => "Revolute",
            JointType::Prismatic => "Prismatic",
            JointType::Continuous => "Continuous",
        }
        .to_string()
    }
}

impl From<&JointType> for urdf_rs::JointType {
    fn from(joint_type: &JointType) -> Self {
        match joint_type {
            JointType::Fixed => urdf_rs::JointType::Fixed,
            JointType::Revolute => urdf_rs::JointType::Revolute,
            JointType::Prismatic => urdf_rs::JointType::Prismatic,
            JointType::Continuous => urdf_rs::JointType::Continuous,
        }
    }
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

impl From<&JointLimit> for urdf_rs::JointLimit {
    fn from(limit: &JointLimit) -> Self {
        Self {
            lower: limit.lower as f64,
            upper: limit.upper as f64,
            effort: limit.effort as f64,
            velocity: limit.velocity as f64,
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

impl From<&JointAxis> for urdf_rs::Axis {
    fn from(axis: &JointAxis) -> Self {
        Self {
            xyz: urdf_rs::Vec3(axis.0.map(|v| v as f64)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Joint {
    pub name: NameInWorkcell,
    pub joint_type: JointType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<JointLimit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<JointAxis>,
}

// TODO(luca) should commands implementation be in rmf_site_editor instead of rmf_site_format?
/// Custom spawning implementation since bundles don't allow options
#[cfg(feature = "bevy")]
impl Joint {
    pub fn add_bevy_components(&self, mut commands: EntityCommands) {
        commands.insert((
            SpatialBundle::VISIBLE_IDENTITY,
            Category::Joint,
            self.name.clone(),
            self.joint_type.clone(),
        ));
        if let Some(limit) = &self.limit {
            commands.insert(limit.clone());
        }
        if let Some(axis) = &self.axis {
            commands.insert(axis.clone());
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Geometry {
    //#[serde(flatten)]
    Primitive(PrimitiveShape),
    Mesh {
        source: AssetSource,
        #[serde(default, skip_serializing_if = "is_default")]
        scale: Option<Vec3>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum PrimitiveShape {
    Box { size: [f32; 3] },
    Cylinder { radius: f32, length: f32 },
    Capsule { radius: f32, length: f32 },
    Sphere { radius: f32 },
}

impl PrimitiveShape {
    pub fn label(&self) -> String {
        match &self {
            PrimitiveShape::Box { .. } => "Box",
            PrimitiveShape::Cylinder { .. } => "Cylinder",
            PrimitiveShape::Capsule { .. } => "Capsule",
            PrimitiveShape::Sphere { .. } => "Sphere",
        }
        .to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallPrimitiveShape {
    pub box_size: Option<[f32; 3]>,
    pub cylinder_radius: Option<f32>,
    pub cylinder_length: Option<f32>,
    pub capsule_radius: Option<f32>,
    pub capsule_length: Option<f32>,
    pub sphere_radius: Option<f32>,
}

impl Recall for RecallPrimitiveShape {
    type Source = PrimitiveShape;

    fn remember(&mut self, source: &PrimitiveShape) {
        match source {
            PrimitiveShape::Box { size } => {
                self.box_size = Some(*size);
            }
            PrimitiveShape::Cylinder { radius, length } => {
                self.cylinder_radius = Some(*radius);
                self.cylinder_length = Some(*length);
            }
            PrimitiveShape::Capsule { radius, length } => {
                self.capsule_radius = Some(*radius);
                self.capsule_length = Some(*length);
            }
            PrimitiveShape::Sphere { radius } => {
                self.sphere_radius = Some(*radius);
            }
        }
    }
}

impl RecallPrimitiveShape {
    pub fn assume_box(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Box { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Box {
                size: self.box_size.unwrap_or_default(),
            }
        }
    }

    pub fn assume_cylinder(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Cylinder { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Cylinder {
                radius: self.cylinder_radius.unwrap_or_default(),
                length: self.cylinder_length.unwrap_or_default(),
            }
        }
    }

    pub fn assume_capsule(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Capsule { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Capsule {
                radius: self.capsule_radius.unwrap_or_default(),
                length: self.capsule_length.unwrap_or_default(),
            }
        }
    }

    pub fn assume_sphere(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Sphere { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Sphere {
                radius: self.sphere_radius.unwrap_or_default(),
            }
        }
    }
}

impl Default for Geometry {
    fn default() -> Self {
        Geometry::Primitive(PrimitiveShape::Box { size: [0.0; 3] })
    }
}

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
            Geometry::Mesh { source, scale } => {
                let scale = Scale(scale.unwrap_or(Vec3::ONE));
                // TODO(luca) Make a bundle for workcell models to avoid manual insertion here
                commands.insert((
                    NameInWorkcell(self.name.clone()),
                    source.clone(),
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

#[derive(Debug, ThisError)]
pub enum UrdfImportError {
    #[error("a joint refers to a non existing link [{0}]")]
    BrokenJointReference(String),
    // TODO(luca) Add urdf_rs::JointType to this error, it doesn't implement Display
    #[error("unsupported joint type found")]
    UnsupportedJointType,
}

impl From<Pose> for urdf_rs::Pose {
    fn from(pose: Pose) -> Self {
        urdf_rs::Pose {
            rpy: match pose.rot {
                Rotation::EulerExtrinsicXYZ(arr) => urdf_rs::Vec3(arr.map(|v| v.radians().into())),
                Rotation::Yaw(v) => urdf_rs::Vec3([0.0, 0.0, v.radians().into()]),
                Rotation::Quat([x, y, z, w]) => {
                    let (z, y, x) = glam::quat(x, y, z, w).to_euler(EulerRot::ZYX);
                    urdf_rs::Vec3([x as f64, y as f64, z as f64])
                }
            },
            xyz: urdf_rs::Vec3(pose.trans.map(|v| v as f64)),
        }
    }
}

impl From<&urdf_rs::Pose> for Pose {
    fn from(pose: &urdf_rs::Pose) -> Self {
        Pose {
            trans: pose.xyz.map(|t| t as f32),
            rot: Rotation::EulerExtrinsicXYZ(pose.rpy.map(|t| Angle::Rad(t as f32))),
        }
    }
}

impl From<Geometry> for urdf_rs::Geometry {
    fn from(geometry: Geometry) -> Self {
        match geometry {
            Geometry::Mesh { source, scale } => urdf_rs::Geometry::Mesh {
                filename: (&source).into(),
                scale: scale.map(|v| urdf_rs::Vec3([v.x as f64, v.y as f64, v.z as f64])),
            },
            Geometry::Primitive(PrimitiveShape::Box { size: [x, y, z] }) => {
                urdf_rs::Geometry::Box {
                    size: urdf_rs::Vec3([x as f64, y as f64, z as f64]),
                }
            }
            Geometry::Primitive(PrimitiveShape::Cylinder { radius, length }) => {
                urdf_rs::Geometry::Cylinder {
                    radius: radius as f64,
                    length: length as f64,
                }
            }
            Geometry::Primitive(PrimitiveShape::Capsule { radius, length }) => {
                urdf_rs::Geometry::Capsule {
                    radius: radius as f64,
                    length: length as f64,
                }
            }
            Geometry::Primitive(PrimitiveShape::Sphere { radius }) => urdf_rs::Geometry::Sphere {
                radius: radius as f64,
            },
        }
    }
}

#[derive(Debug, ThisError)]
pub enum WorkcellToUrdfError {
    #[error("Invalid anchor type {0:?}")]
    InvalidAnchorType(Anchor),
    #[error("Urdf error: {0}")]
    WriteToStringError(#[from] urdf_rs::UrdfError),
    #[error("Broken reference: {0}")]
    BrokenReference(u32),
    #[error("Frame {0} referred by joint {1} has no name, this is not allowed in URDF")]
    MissingJointFrameName(u32, u32),
}

impl Workcell {
    pub fn from_urdf(urdf: &urdf_rs::Robot) -> Result<Self, UrdfImportError> {
        let mut frame_name_to_id = HashMap::new();
        let root_id = 0_u32;
        let mut cur_id = 1u32..;
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
            let parent = frame_name_to_id.get(&joint.parent.link).ok_or(
                UrdfImportError::BrokenJointReference(joint.parent.link.clone()),
            )?;
            let child = frame_name_to_id.get(&joint.child.link).ok_or(
                UrdfImportError::BrokenJointReference(joint.child.link.clone()),
            )?;
            let joint_bundle = match joint.joint_type {
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
                    joint_type: JointType::Fixed,
                    limit: None,
                    axis: None,
                },
                urdf_rs::JointType::Continuous => Joint {
                    name: NameInWorkcell(joint.name.clone()),
                    joint_type: JointType::Continuous,
                    limit: None,
                    axis: None,
                },
                _ => {
                    return Err(UrdfImportError::UnsupportedJointType);
                }
            };
            let joint_id = cur_id.next().unwrap();
            // Reassign the child parenthood and pose to the joint
            // If the frame didn't exist we would have returned an error when populating child
            // hence this is safe.
            let child_frame = frames.get_mut(child).unwrap();
            child_frame.parent = joint_id;
            // In urdf, joint origin represents the coordinates of the joint in the
            // parent frame. The child is always in the origin of the joint
            child_frame.bundle.anchor = Anchor::Pose3D((&joint.origin).into());
            joints.insert(
                joint_id,
                Parented {
                    parent: *parent,
                    bundle: joint_bundle,
                },
            );
        }

        Ok(Workcell {
            properties: WorkcellProperties {
                name: urdf.name.clone(),
            },
            id: root_id,
            frames,
            visuals,
            collisions,
            inertials,
            joints,
        })
    }
    pub fn to_writer<W: io::Write>(&self, writer: W) -> serde_json::Result<()> {
        serde_json::ser::to_writer_pretty(writer, self)
    }

    pub fn to_string(&self) -> serde_json::Result<String> {
        serde_json::ser::to_string_pretty(self)
    }

    pub fn to_urdf(&self) -> Result<urdf_rs::Robot, WorkcellToUrdfError> {
        let mut parent_to_visuals = HashMap::new();
        for (_, visual) in self.visuals.iter() {
            let parent = visual.parent;
            let visual = &visual.bundle;
            let visual = urdf_rs::Visual {
                name: Some(visual.name.clone()),
                origin: visual.pose.into(),
                geometry: visual.geometry.clone().into(),
                material: None,
            };
            parent_to_visuals
                .entry(parent)
                .or_insert_with(Vec::new)
                .push(visual);
        }

        let mut parent_to_collisions = HashMap::new();
        for (_, collision) in self.collisions.iter() {
            let parent = collision.parent;
            let collision = &collision.bundle;
            let collision = urdf_rs::Collision {
                name: Some(collision.name.clone()),
                origin: collision.pose.into(),
                geometry: collision.geometry.clone().into(),
            };
            parent_to_collisions
                .entry(parent)
                .or_insert_with(Vec::new)
                .push(collision);
        }

        // If the workcell has a single frame child we can use the child as the base link.
        // Otherwise, we will need to spawn a new base link to contain all the workcell children
        let workcell_child_frames = self
            .frames
            .iter()
            .filter(|(_, frame)| frame.parent == self.id);
        let num_children = workcell_child_frames.clone().count();
        let frames = if num_children != 1 {
            // TODO(luca) remove hardcoding of base link name, it might in some cases create
            // duplicates
            let mut frames = self.frames.clone();
            let dummy_frame = Frame {
                anchor: Anchor::Pose3D(Pose {
                    rot: Rotation::Quat([0.0, 0.0, 0.0, 0.0]),
                    trans: [0.0, 0.0, 0.0],
                }),
                name: Some(NameInWorkcell(String::from("world"))),
                mesh_constraint: None,
                marker: FrameMarker,
            };
            frames.insert(
                self.id,
                Parented {
                    // Root has no parent, use placeholder of max u32
                    parent: u32::MAX,
                    bundle: dummy_frame,
                },
            );
            frames
        } else {
            // Flatten the hierarchy by making the only child the new workcell base link
            self.frames.clone()
        };

        let mut parent_to_inertials = HashMap::new();
        for (_, inertial) in self.inertials.iter() {
            let parent = inertial.parent;
            let inertial = &inertial.bundle;
            let inertial = urdf_rs::Inertial::from(inertial);
            parent_to_inertials.insert(parent, inertial);
        }

        // TODO(luca) combine multiple frames without a joint inbetween into a single link.
        // For now as soon as a joint is missing the hierarchy will be broken
        let links = frames
            .iter()
            .map(|(frame_id, parented_frame)| {
                let name = match &parented_frame.bundle.name {
                    Some(name) => name.0.clone(),
                    None => format!("frame_{}", &frame_id),
                };

                let inertial = parent_to_inertials.remove(&frame_id).unwrap_or_default();
                let collision = parent_to_collisions.remove(&frame_id).unwrap_or_default();
                let visual = parent_to_visuals.remove(&frame_id).unwrap_or_default();

                urdf_rs::Link {
                    name,
                    inertial,
                    collision,
                    visual,
                }
            })
            .collect::<Vec<_>>();

        let joints = self.joints
            .iter()
            .map(|(joint_id, parented_joint)| {
                let joint_parent = parented_joint.parent;
                let joint = &parented_joint.bundle;
                // The pose of the joint is the pose of the frame that has it as its parent
                let parent_frame = self.frames.get(&joint_parent).ok_or(WorkcellToUrdfError::BrokenReference(joint_parent))?;
                let (child_frame_id, child_frame) = self.frames.iter().find(|(_, frame)| frame.parent == *joint_id).ok_or(WorkcellToUrdfError::BrokenReference(*joint_id))?;
                let parent_name = parent_frame.bundle.name.clone().ok_or(WorkcellToUrdfError::MissingJointFrameName(joint_parent, *joint_id))?;
                let child_name = child_frame.bundle.name.clone().ok_or(WorkcellToUrdfError::MissingJointFrameName(*child_frame_id, *joint_id))?;
                let Anchor::Pose3D(pose) = child_frame.bundle.anchor else {
                    return Err(WorkcellToUrdfError::InvalidAnchorType(child_frame.bundle.anchor.clone()));
                };
                Ok(urdf_rs::Joint {
                    name: joint.name.0.clone(),
                    joint_type: (&joint.joint_type).into(),
                    origin: pose.into(),
                    parent: urdf_rs::LinkName {
                        link: parent_name.0
                    },
                    child: urdf_rs::LinkName {
                        link: child_name.0
                    },
                    axis: joint.axis.as_ref().map(|axis| urdf_rs::Axis::from(axis)).unwrap_or_default(),
                    limit: joint.limit.as_ref().map(|limit| urdf_rs::JointLimit::from(limit)).unwrap_or_default(),
                    dynamics: None,
                    mimic: None,
                    safety_controller: None,
                })
            })
            .collect::<Result<Vec<_>, WorkcellToUrdfError>>()?;

        // TODO(luca) implement materials
        let robot = urdf_rs::Robot {
            name: self.properties.name.clone(),
            links,
            joints,
            materials: vec![],
        };
        Ok(robot)
    }

    pub fn to_urdf_string(&self) -> Result<String, WorkcellToUrdfError> {
        let urdf = self.to_urdf()?;
        urdf_rs::write_to_string(&urdf).map_err(|e| WorkcellToUrdfError::WriteToStringError(e))
    }

    pub fn to_urdf_writer(&self, mut writer: impl io::Write) -> Result<(), std::io::Error> {
        let urdf = self
            .to_urdf_string()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writer.write_all(urdf.as_bytes())
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
pub struct UrdfRoot(pub urdf_rs::Robot);

// TODO(luca) feature gate urdf support
impl From<&urdf_rs::Geometry> for Geometry {
    fn from(geom: &urdf_rs::Geometry) -> Self {
        match geom {
            urdf_rs::Geometry::Box { size } => Geometry::Primitive(PrimitiveShape::Box {
                size: (**size).map(|f| f as f32),
            }),
            urdf_rs::Geometry::Cylinder { radius, length } => {
                Geometry::Primitive(PrimitiveShape::Cylinder {
                    radius: *radius as f32,
                    length: *length as f32,
                })
            }
            urdf_rs::Geometry::Capsule { radius, length } => {
                Geometry::Primitive(PrimitiveShape::Capsule {
                    radius: *radius as f32,
                    length: *length as f32,
                })
            }
            urdf_rs::Geometry::Sphere { radius } => Geometry::Primitive(PrimitiveShape::Sphere {
                radius: *radius as f32,
            }),
            urdf_rs::Geometry::Mesh { filename, scale } => {
                let scale = scale
                    .clone()
                    .and_then(|s| Some(Vec3::from_array(s.map(|v| v as f32))));
                Geometry::Mesh {
                    source: (&**filename).into(),
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
            center: (&inertial.origin).into(),
            mass: Mass(inertial.mass.value as f32),
            inertia: (&inertial.inertia).into(),
        }
    }
}

impl From<&Inertial> for urdf_rs::Inertial {
    fn from(inertial: &Inertial) -> Self {
        Self {
            origin: inertial.center.into(),
            mass: urdf_rs::Mass {
                value: inertial.mass.0 as f64,
            },
            inertia: urdf_rs::Inertia {
                ixx: inertial.inertia.ixx as f64,
                ixy: inertial.inertia.ixy as f64,
                ixz: inertial.inertia.ixz as f64,
                iyy: inertial.inertia.iyy as f64,
                iyz: inertial.inertia.iyz as f64,
                izz: inertial.inertia.izz as f64,
            },
        }
    }
}

impl WorkcellModel {
    fn from_urdf_data(
        pose: &urdf_rs::Pose,
        name: &Option<String>,
        geometry: &urdf_rs::Geometry,
    ) -> Self {
        WorkcellModel {
            name: name.clone().unwrap_or_default(),
            geometry: geometry.into(),
            pose: pose.into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use float_eq::{assert_float_eq, float_eq};

    fn frame_by_name(
        frames: &BTreeMap<u32, Parented<u32, Frame>>,
        name: &str,
    ) -> Option<(u32, Parented<u32, Frame>)> {
        frames
            .iter()
            .find(|(_, parented_frame)| {
                parented_frame.bundle.name == Some(NameInWorkcell(name.to_string()))
            })
            .map(|(id, f)| (*id, f.clone()))
    }

    fn element_by_parent<T: Clone>(
        models: &BTreeMap<u32, Parented<u32, T>>,
        parent: u32,
    ) -> Option<(u32, Parented<u32, T>)> {
        models
            .iter()
            .find(|(_, parented_element)| parented_element.parent == parent)
            .map(|(id, e)| (*id, e.clone()))
    }

    fn is_pose_eq(p1: &Pose, p2: &Pose) -> bool {
        if !p1
            .trans
            .iter()
            .zip(p2.trans.iter())
            .map(|(t1, t2)| float_eq!(t1, t2, abs <= 1e-6))
            .all(|eq| eq)
        {
            return false;
        }
        match (
            p1.rot.as_euler_extrinsic_xyz(),
            p2.rot.as_euler_extrinsic_xyz(),
        ) {
            (Rotation::EulerExtrinsicXYZ(r1), Rotation::EulerExtrinsicXYZ(r2)) => r1
                .iter()
                .zip(r2.iter())
                .map(|(a1, a2)| float_eq!(a1.radians(), a2.radians(), abs <= 1e-6))
                .all(|eq| eq),
            _ => false,
        }
    }

    fn is_inertial_eq(i1: &Inertial, i2: &Inertial) -> bool {
        is_pose_eq(&i1.origin, &i2.origin)
            && float_eq!(i1.mass.0, i2.mass.0, abs <= 1e6)
            && float_eq!(i1.inertia.ixx, i2.inertia.ixx, abs <= 1e6)
            && float_eq!(i1.inertia.ixy, i2.inertia.ixy, abs <= 1e6)
            && float_eq!(i1.inertia.ixz, i2.inertia.ixz, abs <= 1e6)
            && float_eq!(i1.inertia.iyy, i2.inertia.iyy, abs <= 1e6)
            && float_eq!(i1.inertia.iyz, i2.inertia.iyz, abs <= 1e6)
            && float_eq!(i1.inertia.izz, i2.inertia.izz, abs <= 1e6)
    }

    #[test]
    fn urdf_roundtrip() {
        let urdf = urdf_rs::read_file("test/07-physics.urdf").unwrap();
        let workcell = Workcell::from_urdf(&urdf).unwrap();
        assert_eq!(workcell.visuals.len(), 16);
        assert_eq!(workcell.collisions.len(), 16);
        assert_eq!(workcell.frames.len(), 16);
        assert_eq!(workcell.joints.len(), 15);
        assert_eq!(workcell.properties.name, "physics");
        // Test that we convert poses from joints to frames
        let (right_leg_id, right_leg) = frame_by_name(&workcell.frames, "right_leg").unwrap();
        let target_right_leg_pose = Pose {
            trans: [0.0, -0.22, 0.25],
            rot: Default::default(),
        };
        assert!(right_leg
            .bundle
            .anchor
            .is_close(&Anchor::Pose3D(target_right_leg_pose), 1e-6));
        // Test that we can parse parenthood and properties of visuals and collisions correctly
        let (_, right_leg_visual) = element_by_parent(&workcell.visuals, right_leg_id).unwrap();
        let target_right_leg_model_pose = Pose {
            trans: [0.0, 0.0, -0.3],
            rot: Rotation::EulerExtrinsicXYZ([
                Angle::Rad(0.0),
                Angle::Rad(1.57075),
                Angle::Rad(0.0),
            ]),
        };
        assert!(is_pose_eq(
            &right_leg_visual.bundle.pose,
            &target_right_leg_model_pose
        ));
        assert!(matches!(
            right_leg_visual.bundle.geometry,
            Geometry::Primitive(PrimitiveShape::Box { .. })
        ));
        let (_, right_leg_collision) =
            element_by_parent(&workcell.collisions, right_leg_id).unwrap();
        assert!(is_pose_eq(
            &right_leg_collision.bundle.pose,
            &target_right_leg_model_pose
        ));
        assert!(matches!(
            right_leg_collision.bundle.geometry,
            Geometry::Primitive(PrimitiveShape::Box { .. })
        ));
        // Test inertial parenthood and parsing
        let (_, right_leg_inertial) = element_by_parent(&workcell.inertials, right_leg_id).unwrap();
        assert_float_eq!(right_leg_inertial.bundle.mass.0, 10.0, abs <= 1e6);
        let target_right_leg_inertial = Inertial {
            origin: Pose::default(),
            mass: Mass(10.0),
            inertia: Inertia {
                ixx: 1.0,
                ixy: 0.0,
                ixz: 0.0,
                iyy: 1.0,
                iyz: 0.0,
                izz: 1.0,
            },
        };
        assert!(is_inertial_eq(
            &right_leg_inertial.bundle,
            &target_right_leg_inertial
        ));
        // Test joint parenthood and parsing
        let (_, right_leg_joint) = element_by_parent(&workcell.joints, right_leg_id).unwrap();
        assert_eq!(right_leg_joint.bundle.joint_type, JointType::Fixed);
        assert_eq!(
            right_leg_joint.bundle.name,
            NameInWorkcell("right_base_joint".to_string())
        );
        // Test that the new urdf contains the same data
        let new_urdf = workcell.to_urdf().unwrap();
        assert_eq!(new_urdf.name, "physics");
        assert_eq!(new_urdf.links.len(), 16);
        assert_eq!(new_urdf.joints.len(), 15);
        // Check that link information is preserved
        let right_leg_link = new_urdf
            .links
            .iter()
            .find(|l| l.name == "right_leg")
            .unwrap();
        assert!(is_inertial_eq(
            &(&right_leg_link.inertial).into(),
            &target_right_leg_inertial
        ));
        assert_eq!(right_leg_link.visual.len(), 1);
        assert_eq!(right_leg_link.collision.len(), 1);
        let right_leg_visual = right_leg_link.visual.get(0).unwrap();
        let right_leg_collision = right_leg_link.collision.get(0).unwrap();
        assert!(is_pose_eq(
            &(&right_leg_visual.origin).into(),
            &target_right_leg_model_pose
        ));
        assert!(is_pose_eq(
            &(&right_leg_collision.origin).into(),
            &target_right_leg_model_pose
        ));
        assert!(matches!(
            right_leg_visual.geometry,
            urdf_rs::Geometry::Box { .. }
        ));
        assert!(matches!(
            right_leg_collision.geometry,
            urdf_rs::Geometry::Box { .. }
        ));
        // Check that joint origin is preserved
        let right_leg_joint = new_urdf
            .joints
            .iter()
            .find(|l| l.name == "base_to_right_leg")
            .unwrap();
        assert!(is_pose_eq(
            &(&right_leg_joint.origin).into(),
            &target_right_leg_pose
        ));
        assert_eq!(right_leg_joint.joint_type, urdf_rs::JointType::Fixed);
    }
}
