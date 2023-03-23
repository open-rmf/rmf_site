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
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum AssetSource {
    Local(String),
    Remote(String),
    Search(String),
    Bundled(String),
    Package(String),
}

impl AssetSource {
    pub fn label(&self) -> &str {
        match self {
            Self::Local(_) => "Local",
            Self::Remote(_) => "Remote",
            Self::Search(_) => "Search",
            Self::Bundled(_) => "Bundled",
            Self::Package(_) => "Package",
        }
    }
}

impl Default for AssetSource {
    fn default() -> Self {
        AssetSource::Local(String::new()).into()
    }
}

// Utility functions to add / strip prefixes for using AssetSource in AssetIo objects
impl From<&Path> for AssetSource {
    fn from(path: &Path) -> Self {
        if let Some(path) = path.to_str().and_then(|p| Some(String::from(p))) {
            AssetSource::from(&path)
        } else {
            AssetSource::default()
        }
    }
}

// Utility functions to add / strip prefixes for using AssetSource in AssetIo objects
impl From<&String> for AssetSource {
    fn from(path: &String) -> Self {
        // TODO(luca) pattern matching here would make sure unimplemented variants are a compile error
        if let Some(path) = path.strip_prefix("rmf-server://").and_then(|p| Some(p.to_string())) {
            return AssetSource::Remote(path);
        } else if let Some(path) = path.strip_prefix("file://").and_then(|p| Some(p.to_string())) {
            return AssetSource::Local(path);
        } else if let Some(path) = path.strip_prefix("search://").and_then(|p| Some(p.to_string())) {
            return AssetSource::Search(path);
        } else if let Some(path) = path.strip_prefix("bundled://").and_then(|p| Some(p.to_string())) {
            return AssetSource::Bundled(path);
        } else if let Some(path) = path.strip_prefix("package://").and_then(|p| Some(p.to_string())) {
            return AssetSource::Package(path);
        }
        AssetSource::default()
    }
}

impl From<&AssetSource> for String {
    fn from(asset_source: &AssetSource) -> String {
        match asset_source {
            AssetSource::Remote(uri) => String::from("rmf-server://") + &uri,
            AssetSource::Local(filename) => String::from("file://") + &filename,
            AssetSource::Search(name) => String::from("search://") + &name,
            AssetSource::Bundled(name) => String::from("bundled://") + &name,
            AssetSource::Package(path) => String::from("package://") + &path,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallAssetSource {
    pub filename: Option<String>,
    pub remote_uri: Option<String>,
    pub search_name: Option<String>,
    pub bundled_name: Option<String>,
    pub package_path: Option<String>,
}

impl Recall for RecallAssetSource {
    type Source = AssetSource;

    fn remember(&mut self, source: &AssetSource) {
        match source {
            AssetSource::Local(name) => {
                self.filename = Some(name.clone());
            }
            AssetSource::Remote(uri) => {
                self.remote_uri = Some(uri.clone());
            }
            AssetSource::Search(name) => {
                self.search_name = Some(name.clone());
            }
            AssetSource::Bundled(name) => {
                self.bundled_name = Some(name.clone());
            }
            AssetSource::Package(path) => {
                self.package_path = Some(path.clone());
            }
        }
    }
}
