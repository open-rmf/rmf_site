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

use crate::*;
#[cfg(feature = "bevy")]
use bevy::ecs::system::EntityCommands;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity};
#[cfg(feature = "bevy")]
use bevy::reflect::TypeUuid;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use urdf_rs::{Robot, Visual};

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
pub struct Inertia {}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Inertial {
    pub origin: Pose,
    pub mass: Mass,
    pub inertia: Inertia,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Link {
    pub name: NameInWorkcell,
    pub inertial: Inertial,
    #[serde(skip)]
    pub marker: LinkMarker,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct LinkMarker;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Joint {
    pub name: NameInWorkcell,
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
    // TODO(luca) Joints
}

impl From<Pose> for urdf_rs::Pose {
    fn from(pose: Pose) -> Self {
        urdf_rs::Pose {
            rpy: match pose.rot {
                Rotation::EulerExtrinsicXYZ(arr) => urdf_rs::Vec3(arr.map(|v| match v {
                    Angle::Rad(v) => v as f64,
                    Angle::Deg(v) => v.to_radians() as f64,
                })),
                Rotation::Yaw(v) => match v {
                    Angle::Rad(v) => urdf_rs::Vec3([0.0, 0.0, v as f64]),
                    Angle::Deg(v) => urdf_rs::Vec3([0.0, 0.0, v.to_radians() as f64]),
                },
                _ => todo!("Unsupported rotation type for conversion to urdf pose"),
            },
            xyz: urdf_rs::Vec3(pose.trans.map(|v| v as f64)),
        }
    }
}

impl From<Geometry> for urdf_rs::Geometry {
    fn from(geometry: Geometry) -> Self {
        match geometry {
            Geometry::Mesh { filename, scale } => urdf_rs::Geometry::Mesh {
                filename,
                scale: scale.map(|v| urdf_rs::Vec3([v.x as f64, v.y as f64, v.z as f64])),
            },
            Geometry::Primitive(MeshPrimitive::Box { size: [x, y, z] }) => urdf_rs::Geometry::Box {
                size: urdf_rs::Vec3([x as f64, y as f64, z as f64]),
            },
            Geometry::Primitive(MeshPrimitive::Cylinder { radius, length }) => {
                urdf_rs::Geometry::Cylinder {
                    radius: radius as f64,
                    length: length as f64,
                }
            }
            Geometry::Primitive(MeshPrimitive::Capsule { radius, length }) => {
                urdf_rs::Geometry::Capsule {
                    radius: radius as f64,
                    length: length as f64,
                }
            }
            Geometry::Primitive(MeshPrimitive::Sphere { radius }) => urdf_rs::Geometry::Sphere {
                radius: radius as f64,
            },
            _ => todo!("Only meshes and primitives are supported for conversion to urdf geometry"),
        }
    }
}

impl From<Workcell> for urdf_rs::Robot {
    fn from(workcell: Workcell) -> Self {
        dbg!(&workcell);
        let visuals_and_parent: Vec<(urdf_rs::Visual, _)> = workcell
            .visuals
            .into_iter()
            .map(|v| {
                let visual = v.1.bundle;
                let visual = urdf_rs::Visual {
                    name: Some(visual.name),
                    origin: visual.pose.into(),
                    geometry: visual.geometry.into(),
                    material: None,
                };
                (visual, v.1.parent)
            })
            .collect();
        urdf_rs::Robot {
            name: workcell.properties.name,
            links: workcell
                .frames
                .into_iter()
                .map(|f| {
                    let frame = f.1.bundle;
                    let visual: Vec<Visual> = visuals_and_parent
                        .iter()
                        .filter(|(_, parent)| parent == &f.1.parent)
                        .map(|(visual, _)| visual.clone())
                        .collect();

                    dbg!(&frame);
                    let pose: urdf_rs::Pose = match frame.anchor {
                        Anchor::Pose3D(pose) => pose.into(),
                        _ => todo!(),
                    };
                    urdf_rs::Link {
                        name: match frame.name {
                            Some(name) => name.0,
                            None => format!("frame_{}", f.0),
                        },
                        inertial: urdf_rs::Inertial {
                            origin: pose,
                            inertia: {
                                urdf_rs::Inertia {
                                    ixx: 0.0,
                                    ixy: 0.0,
                                    ixz: 0.0,
                                    iyy: 0.0,
                                    iyz: 0.0,
                                    izz: 0.0,
                                }
                            },
                            mass: urdf_rs::Mass { value: 0.0 },
                        },
                        collision: vec![],
                        visual,
                    }
                })
                .collect(),
            joints: vec![],
            materials: vec![],
        }
    }
}

impl Workcell {
    pub fn to_writer<W: io::Write>(&self, writer: W) -> serde_json::Result<()> {
        serde_json::ser::to_writer_pretty(writer, self)
    }

    pub fn to_string(&self) -> serde_json::Result<String> {
        serde_json::ser::to_string_pretty(self)
    }

    pub fn to_urdf(&self) -> urdf_rs::Robot {
        self.clone().into()
    }

    pub fn to_urdf_string(&self) -> urdf_rs::Result<String> {
        let urdf = self.to_urdf();
        urdf_rs::write_to_string(&urdf)
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

impl From<&urdf_rs::Link> for Link {
    fn from(link: &urdf_rs::Link) -> Self {
        Self {
            name: NameInWorkcell(link.name.clone()),
            inertial: Inertial {
                origin: Pose {
                    trans: link.inertial.origin.xyz.0.map(|v| v as f32),
                    rot: Rotation::EulerExtrinsicXYZ(
                        link.inertial.origin.rpy.map(|v| Angle::Rad(v as f32)),
                    ),
                },
                mass: Mass(link.inertial.mass.value as f32),
                inertia: Inertia::default(),
            },
            marker: LinkMarker,
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
