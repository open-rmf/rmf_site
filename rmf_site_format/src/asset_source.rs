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
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    /// Returns true if the asset source is a local file with a relative path.
    pub fn is_local_relative(&self) -> bool {
        if let Self::Local(asset_path) = self {
            return Path::new(asset_path).is_relative();
        }

        return false;
    }

    /// If the AssetSource contains a relative local path, this will migrate its
    /// relativity from `old_path` to `new_path`. For any other path type, this
    /// function simply returns Ok. It will return Err if diff_paths was not
    /// able to calculate how to make the asset path relative to the new path or
    /// if the result could not be converted back to a string.
    pub fn migrate_relative_path(
        &mut self,
        old_reference_path: &PathBuf,
        new_reference_path: &PathBuf,
    ) -> Result<(), ()> {
        if let Self::Local(asset_path) = self {
            if Path::new(asset_path).is_relative() {
                println!("Changing path for [{asset_path:?}]");
                let new_path = diff_paths(
                    &old_reference_path.with_file_name(asset_path.clone()),
                    new_reference_path,
                )
                .ok_or(())?;
                *asset_path = new_path.to_str().ok_or(())?.to_owned();
            }
        }

        Ok(())
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
        if let Some(path) = path.strip_prefix("rmf-server://").map(|p| p.to_string()) {
            return AssetSource::Remote(path);
        } else if let Some(path) = path.strip_prefix("file://").map(|p| p.to_string()) {
            return AssetSource::Local(path);
        } else if let Some(path) = path.strip_prefix("search://").map(|p| p.to_string()) {
            return AssetSource::Search(path);
        } else if let Some(path) = path.strip_prefix("bundled://").map(|p| p.to_string()) {
            return AssetSource::Bundled(path);
        } else if let Some(path) = path.strip_prefix("package://").map(|p| p.to_string()) {
            return AssetSource::Package(path);
        }
        AssetSource::default()
    }
}

impl From<&AssetSource> for String {
    fn from(asset_source: &AssetSource) -> String {
        match asset_source {
            AssetSource::Remote(uri) => String::from("rmf-server://") + uri,
            AssetSource::Local(filename) => String::from("file://") + filename,
            AssetSource::Search(name) => String::from("search://") + name,
            AssetSource::Bundled(name) => String::from("bundled://") + name,
            AssetSource::Package(path) => String::from("package://") + path,
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
