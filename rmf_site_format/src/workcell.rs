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

use std::collections::{BTreeMap, HashMap};

#[cfg(feature = "bevy")]
use std::collections::HashSet;

use std::io;

use crate::misc::Rotation;
use crate::*;
#[cfg(feature = "bevy")]
use bevy::ecs::system::EntityCommands;
#[cfg(feature = "bevy")]
use bevy::prelude::{
    Bundle, Component, Deref, DerefMut, Entity, Reflect, ReflectComponent, SpatialBundle,
};
#[cfg(feature = "bevy")]
use bevy::reflect::TypePath;
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct NameOfWorkcell(pub String);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct WorkcellProperties {
    pub name: NameOfWorkcell,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct NameInWorkcell(pub String);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct Mass(f32);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Moment {
    pub ixx: f32,
    pub ixy: f32,
    pub ixz: f32,
    pub iyy: f32,
    pub iyz: f32,
    pub izz: f32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Inertia {
    pub center: Pose,
    pub mass: Mass,
    pub moment: Moment,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
enum RangeLimits {
    None,
    Symmetric(f32),
    Asymmetric {
        lower: Option<f32>,
        upper: Option<f32>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JointLimits {
    position: RangeLimits,
    effort: RangeLimits,
    velocity: RangeLimits,
}

impl From<&urdf_rs::JointLimit> for JointLimits {
    fn from(limit: &urdf_rs::JointLimit) -> Self {
        Self {
            position: RangeLimits::Asymmetric {
                lower: Some(limit.lower as f32),
                upper: Some(limit.upper as f32),
            },
            effort: RangeLimits::Symmetric(limit.effort as f32),
            velocity: RangeLimits::Symmetric(limit.velocity as f32),
        }
    }
}

impl From<&JointLimits> for urdf_rs::JointLimit {
    fn from(limits: &JointLimits) -> Self {
        const DEFAULT_EFFORT_LIMIT: f64 = 1e3;
        const DEFAULT_VELOCITY_LIMIT: f64 = 10.0;
        fn min_or_default(slice: [Option<f32>; 2], default: f64) -> f64 {
            let mut vec = slice
                .iter()
                .filter_map(|v| v.map(|m| m as f64))
                .collect::<Vec<_>>();
            vec.sort_by(|a, b| a.total_cmp(b));
            vec.first().cloned().unwrap_or(default)
        }
        // 0.0 is a valid default in urdf for lower and upper limits
        let (lower, upper) = match limits.position {
            RangeLimits::None => (0.0, 0.0),
            RangeLimits::Symmetric(l) => (l as f64, l as f64),
            RangeLimits::Asymmetric { lower, upper } => (
                lower.map(|v| v as f64).unwrap_or_default(),
                upper.map(|v| v as f64).unwrap_or_default(),
            ),
        };
        let effort = match limits.effort {
            RangeLimits::None => {
                println!(
                    "No effort limit found when exporting to urdf, setting to {}",
                    DEFAULT_EFFORT_LIMIT
                );
                DEFAULT_EFFORT_LIMIT
            }
            RangeLimits::Symmetric(l) => l as f64,
            RangeLimits::Asymmetric { lower, upper } => {
                let limit = min_or_default([lower, upper], DEFAULT_EFFORT_LIMIT);
                println!(
                    "Asymmetric effort limit found when exporting to urdf, setting to {}",
                    limit
                );
                limit
            }
        };
        let velocity = match limits.velocity {
            RangeLimits::None => {
                println!(
                    "No velocity limit found when exporting to urdf, setting to {}",
                    DEFAULT_VELOCITY_LIMIT
                );
                DEFAULT_VELOCITY_LIMIT
            }
            RangeLimits::Symmetric(l) => l as f64,
            RangeLimits::Asymmetric { lower, upper } => {
                let limit = min_or_default([lower, upper], DEFAULT_VELOCITY_LIMIT);
                println!(
                    "Asymmetric velocity limit found when exporting to urdf, setting to {}",
                    limit
                );
                limit
            }
        };
        Self {
            lower,
            upper,
            effort,
            velocity,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Joint {
    pub name: NameInWorkcell,
    pub properties: JointProperties,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum JointProperties {
    Fixed,
    Prismatic(SingleDofJoint),
    Revolute(SingleDofJoint),
    Continuous(SingleDofJoint),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SingleDofJoint {
    pub limits: JointLimits,
    pub axis: JointAxis,
}

impl JointProperties {
    pub fn label(&self) -> String {
        match &self {
            JointProperties::Fixed => "Fixed",
            JointProperties::Revolute(_) => "Revolute",
            JointProperties::Prismatic(_) => "Prismatic",
            JointProperties::Continuous(_) => "Continuous",
        }
        .to_string()
    }
}

// TODO(luca) should commands implementation be in rmf_site_editor instead of rmf_site_format?
/// Custom spawning implementation since bundles don't allow options
#[cfg(feature = "bevy")]
impl Joint {
    pub fn add_bevy_components(&self, commands: &mut EntityCommands) {
        commands.insert((
            SpatialBundle::INHERITED_IDENTITY,
            Category::Joint,
            self.name.clone(),
            self.properties.clone(),
        ));
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
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum PrimitiveShape {
    Box { size: [f32; 3] },
    Cylinder { radius: f32, length: f32 },
    Capsule { radius: f32, length: f32 },
    Sphere { radius: f32 },
}

impl Default for PrimitiveShape {
    fn default() -> Self {
        Self::Box {
            size: [1.0, 1.0, 1.0],
        }
    }
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
    pub fn add_bevy_components(&self, commands: &mut EntityCommands) {
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
    /// Inertias, key is their id, used for hierarchy
    pub inertias: BTreeMap<u32, Parented<u32, Inertia>>,
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
                // SAFETY: We don't need to validate the syntax of the asset
                // path because that will be done later when we attempt to load
                // this as an asset.
                filename: unsafe { (&source).as_unvalidated_asset_path() },
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
        let mut inertias = BTreeMap::new();
        let mut joints = BTreeMap::new();
        // Populate here
        for link in &urdf.links {
            let inertia = Inertia::from(&link.inertial);
            // Add a frame with the link's name, then the inertia data as a child
            let frame_id = cur_id.next().unwrap();
            let inertia_id = cur_id.next().unwrap();
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
            inertias.insert(
                inertia_id,
                Parented {
                    parent: frame_id,
                    bundle: inertia,
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
            let properties = match joint.joint_type {
                urdf_rs::JointType::Revolute => JointProperties::Revolute(SingleDofJoint {
                    axis: (&joint.axis).into(),
                    limits: (&joint.limit).into(),
                }),
                urdf_rs::JointType::Prismatic => JointProperties::Prismatic(SingleDofJoint {
                    axis: (&joint.axis).into(),
                    limits: (&joint.limit).into(),
                }),
                urdf_rs::JointType::Fixed => JointProperties::Fixed,
                urdf_rs::JointType::Continuous => JointProperties::Continuous(SingleDofJoint {
                    axis: (&joint.axis).into(),
                    limits: (&joint.limit).into(),
                }),
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
                    bundle: Joint {
                        name: NameInWorkcell(joint.name.clone()),
                        properties,
                    },
                },
            );
        }

        Ok(Workcell {
            properties: WorkcellProperties {
                name: NameOfWorkcell(urdf.name.clone()),
            },
            id: root_id,
            frames,
            visuals,
            collisions,
            inertias,
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
                // As per Industrial Workcell Coordinate Conventions, the name of the workcell
                // datum link shall be "<workcell_name>_workcell_link".
                name: Some(NameInWorkcell(String::from(
                    self.properties.name.0.clone() + "_workcell_link",
                ))),
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
        for (_, inertia) in self.inertias.iter() {
            let parent = inertia.parent;
            let inertia = &inertia.bundle;
            let inertial = urdf_rs::Inertial::from(inertia);
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

        let joints = self
            .joints
            .iter()
            .map(|(joint_id, parented_joint)| {
                let joint_parent = parented_joint.parent;
                let joint = &parented_joint.bundle;
                // The pose of the joint is the pose of the frame that has it as its parent
                let parent_frame = self
                    .frames
                    .get(&joint_parent)
                    .ok_or(WorkcellToUrdfError::BrokenReference(joint_parent))?;
                let (child_frame_id, child_frame) = self
                    .frames
                    .iter()
                    .find(|(_, frame)| frame.parent == *joint_id)
                    .ok_or(WorkcellToUrdfError::BrokenReference(*joint_id))?;
                let parent_name = parent_frame.bundle.name.clone().ok_or(
                    WorkcellToUrdfError::MissingJointFrameName(joint_parent, *joint_id),
                )?;
                let child_name = child_frame.bundle.name.clone().ok_or(
                    WorkcellToUrdfError::MissingJointFrameName(*child_frame_id, *joint_id),
                )?;
                let Anchor::Pose3D(pose) = child_frame.bundle.anchor else {
                    return Err(WorkcellToUrdfError::InvalidAnchorType(
                        child_frame.bundle.anchor.clone(),
                    ));
                };
                let (joint_type, axis, limit) = match &joint.properties {
                    JointProperties::Fixed => (
                        urdf_rs::JointType::Fixed,
                        urdf_rs::Axis::default(),
                        urdf_rs::JointLimit::default(),
                    ),
                    JointProperties::Revolute(joint) => (
                        urdf_rs::JointType::Revolute,
                        (&joint.axis).into(),
                        (&joint.limits).into(),
                    ),
                    JointProperties::Prismatic(joint) => (
                        urdf_rs::JointType::Prismatic,
                        (&joint.axis).into(),
                        (&joint.limits).into(),
                    ),
                    JointProperties::Continuous(joint) => (
                        urdf_rs::JointType::Continuous,
                        (&joint.axis).into(),
                        (&joint.limits).into(),
                    ),
                };
                Ok(urdf_rs::Joint {
                    name: joint.name.0.clone(),
                    joint_type,
                    origin: pose.into(),
                    parent: urdf_rs::LinkName {
                        link: parent_name.0,
                    },
                    child: urdf_rs::LinkName { link: child_name.0 },
                    axis,
                    limit,
                    dynamics: None,
                    mimic: None,
                    safety_controller: None,
                })
            })
            .collect::<Result<Vec<_>, WorkcellToUrdfError>>()?;

        // TODO(luca) implement materials
        let robot = urdf_rs::Robot {
            name: self.properties.name.0.clone(),
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
    derive(Component, Clone, Debug, Deref, DerefMut, TypePath)
)]
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
                // Most (all?) Urdf files use package references, we fallback to local if that is
                // not the case
                let source = if let Some(path) = filename.strip_prefix("package://") {
                    AssetSource::Package(path.to_owned())
                } else {
                    AssetSource::Local(filename.clone())
                };
                Geometry::Mesh { source, scale }
            }
        }
    }
}

impl From<&urdf_rs::Inertia> for Moment {
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

impl From<&urdf_rs::Inertial> for Inertia {
    fn from(inertial: &urdf_rs::Inertial) -> Self {
        Self {
            center: (&inertial.origin).into(),
            mass: Mass(inertial.mass.value as f32),
            moment: (&inertial.inertia).into(),
        }
    }
}

impl From<&Inertia> for urdf_rs::Inertial {
    fn from(inertia: &Inertia) -> Self {
        Self {
            origin: inertia.center.into(),
            mass: urdf_rs::Mass {
                value: inertia.mass.0 as f64,
            },
            inertia: urdf_rs::Inertia {
                ixx: inertia.moment.ixx as f64,
                ixy: inertia.moment.ixy as f64,
                ixz: inertia.moment.ixz as f64,
                iyy: inertia.moment.iyy as f64,
                iyz: inertia.moment.iyz as f64,
                izz: inertia.moment.izz as f64,
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

    fn is_inertia_eq(i1: &Inertia, i2: &Inertia) -> bool {
        is_pose_eq(&i1.center, &i2.center)
            && float_eq!(i1.mass.0, i2.mass.0, abs <= 1e6)
            && float_eq!(i1.moment.ixx, i2.moment.ixx, abs <= 1e6)
            && float_eq!(i1.moment.ixy, i2.moment.ixy, abs <= 1e6)
            && float_eq!(i1.moment.ixz, i2.moment.ixz, abs <= 1e6)
            && float_eq!(i1.moment.iyy, i2.moment.iyy, abs <= 1e6)
            && float_eq!(i1.moment.iyz, i2.moment.iyz, abs <= 1e6)
            && float_eq!(i1.moment.izz, i2.moment.izz, abs <= 1e6)
    }

    #[test]
    fn urdf_roundtrip() {
        let urdf = urdf_rs::read_file("test/07-physics.urdf").unwrap();
        let workcell = Workcell::from_urdf(&urdf).unwrap();
        assert_eq!(workcell.visuals.len(), 16);
        assert_eq!(workcell.collisions.len(), 16);
        assert_eq!(workcell.frames.len(), 16);
        assert_eq!(workcell.joints.len(), 15);
        assert_eq!(workcell.properties.name.0, "physics");
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
        // Test inertia parenthood and parsing
        let (_, right_leg_inertia) = element_by_parent(&workcell.inertias, right_leg_id).unwrap();
        assert_float_eq!(right_leg_inertia.bundle.mass.0, 10.0, abs <= 1e6);
        let target_right_leg_inertia = Inertia {
            center: Pose::default(),
            mass: Mass(10.0),
            moment: Moment {
                ixx: 1.0,
                ixy: 0.0,
                ixz: 0.0,
                iyy: 1.0,
                iyz: 0.0,
                izz: 1.0,
            },
        };
        assert!(is_inertia_eq(
            &right_leg_inertia.bundle,
            &target_right_leg_inertia
        ));
        // Test joint parenthood and parsing
        let (_, right_leg_joint) = element_by_parent(&workcell.joints, right_leg_id).unwrap();
        assert!(matches!(
            right_leg_joint.bundle.properties,
            JointProperties::Fixed
        ));
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
        assert!(is_inertia_eq(
            &(&right_leg_link.inertial).into(),
            &target_right_leg_inertia
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
        assert!(matches!(
            right_leg_joint.joint_type,
            urdf_rs::JointType::Fixed
        ));
    }
}
