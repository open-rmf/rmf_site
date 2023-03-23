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
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity};
use serde::{Deserialize, Serialize, Serializer};

/// Helper structure to serialize / deserialize entities with parents
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Parented<P: RefTrait, T> {
    pub parent: Option<P>,
    #[serde(flatten)]
    pub bundle: T,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct FrameMarker;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Frame {
    #[serde(flatten)]
    pub anchor: Anchor,
    #[serde(skip)]
    pub marker: FrameMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct MeshConstraint<T: RefTrait> {
    pub entity: T,
    pub element: MeshElement,
    pub relative_pose: Pose,
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
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct NameInWorkcell(String);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Link {
    pub name: NameInWorkcell,
}

impl Link {
    pub fn new(name: String) -> Self {
        Self {
            name: NameInWorkcell(name),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Joint {
    pub name: NameInWorkcell,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum MeshPrimitive {
    Box{size: [f32; 3]},
    Cylinder{radius: f32, length: f32},
    Capsule{radius: f32, length: f32},
    Sphere{radius: f32},
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
    /// Models, key is their id, used for hierarchy
    pub models: BTreeMap<u32, Parented<u32, Model>>,
    /// Mesh constraints, key is their id, matches an anchor id
    // TODO(luca) merge with frames? Not immediate since optional components are not allowed in
    // bundles
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mesh_constraints: BTreeMap<u32, MeshConstraint<u32>>,
}

impl Workcell {
    pub fn to_writer<W: io::Write>(&self, writer: W) -> serde_json::Result<()> {
        serde_json::ser::to_writer_pretty(writer, self)
    }

    pub fn to_string(&self) -> serde_json::Result<String> {
        serde_json::ser::to_string_pretty(self)
    }

    pub fn from_reader<R: io::Read>(reader: R) -> serde_json::Result<Self> {
        // TODO(MXG): Validate the parsed data, e.g. make sure anchor pairs
        // belong to the same level.
        serde_json::de::from_reader(reader)
    }

    pub fn from_str<'a>(s: &'a str) -> serde_json::Result<Self> {
        serde_json::de::from_str(s)
    }

    pub fn from_bytes<'a>(s: &'a [u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(s)
    }
}
