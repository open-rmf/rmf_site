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

use std::collections::BTreeMap;
use std::io;

use crate::*;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Entity};
use serde::{Deserialize, Serialize, Serializer};

/// Helper structure to serialize / deserialize entities with parents
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Parented<P: RefTrait, T> {
    pub parent: Option<P>,
    #[serde(flatten)]
    pub bundle: T,
}

// TODO(luca) we might need a different bundle to denote a workcell included in site
// editor mode to deal with serde of workcells there (that would only have an asset source?)
/// Bundle used to spawn and move whole workcells in site editor mode
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Workcell {
    /// Used in site editor to assign a unique name
    pub name: NameInSite,
    /// Anchors, key is their id, used for hierarchy
    pub anchors: BTreeMap<u32, Parented<u32, Anchor>>,
    /// Models, key is their id, used for hierarchy
    pub models: BTreeMap<u32, Parented<u32, Model>>,
    // TODO(luca) add source, might need asset loader specialization
    // since workcells will be saved as .workcell.ron files
    // pub source: AssetSource,
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

/*
/// Populated in workcell editor mode, in site editor a Workcell will have
/// a series of non mutable WorkcellElement child entities
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct WorkcellElement {
    /// Unique name to identify the element
    pub name: NameInSite,
    /// Workcell elements are normal meshes, point to where the mesh is stored
    pub source: AssetSource,
    /// Workcell element poses are defined relative to other entities
    pub pose: Pose,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct WorkcellAnchor {
    /// Anchor element
    pub anchor: Anchor,
    // TODO(luca) Add mesh constraint
    /// Pose are defined relative to other entities
    pub pose: Pose,
}
*/
