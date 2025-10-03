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
    asset::{AssetPath, ParseAssetPathError},
    prelude::{Component, Reflect, ReflectComponent},
};
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum AssetSource {
    Local(String),
    Remote(String),
    Search(String),
    Package(String),
    Memory(String),
}

impl AssetSource {
    pub fn label(&self) -> &str {
        match self {
            Self::Local(_) => "Local",
            Self::Remote(_) => "Remote",
            Self::Search(_) => "Search",
            Self::Package(_) => "Package",
            Self::Memory(_) => "Memory",
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
                    old_reference_path.with_file_name(asset_path.clone()),
                    new_reference_path.parent().ok_or(())?,
                )
                .ok_or(())?;
                *asset_path = new_path.to_str().ok_or(())?.to_owned();
            }
        }

        Ok(())
    }

    /// Convert the asset source into an asset path without attempting to validate
    /// whether the asset path has valid syntax.
    pub unsafe fn as_unvalidated_asset_path(&self) -> String {
        match self {
            AssetSource::Remote(uri) => String::from("rmf-server://") + uri,
            AssetSource::Local(filename) => String::from("file://") + filename,
            AssetSource::Search(name) => String::from("search://") + name,
            AssetSource::Package(path) => String::from("package://") + path,
            AssetSource::Memory(path) => String::from("memory://") + path,
        }
    }

    /// Extracts the model name (the content of the source after its last '/').
    pub fn model_name(&self) -> String {
        let asset_path = unsafe { self.as_unvalidated_asset_path() };
        // Unwrap safe because split will always have at least one item
        asset_path.split("/").last().unwrap().into()
    }

    /// If the asset source is local, convert it to an absolute path name relative
    /// to the specified base folder.
    pub fn with_base_path(self, base_path: Option<&PathBuf>) -> Self {
        let Some(base_folder) = base_path else {
            return self;
        };

        match self {
            AssetSource::Local(filename) => {
                let path = PathBuf::from(filename);
                if path.is_relative() {
                    Self::Local(
                        base_folder
                            .with_file_name(path)
                            .to_string_lossy()
                            .into_owned(),
                    )
                } else {
                    Self::Local(path.to_string_lossy().into_owned())
                }
            }
            source => source,
        }
    }
}

impl Default for AssetSource {
    fn default() -> Self {
        AssetSource::Local(String::new()).into()
    }
}

#[cfg(feature = "bevy")]
impl TryFrom<&AssetSource> for String {
    type Error = ParseAssetPathError;
    fn try_from(asset_source: &AssetSource) -> Result<String, ParseAssetPathError> {
        // SAFETY: After we get this string, we immediately validate it before
        // returning it.
        let result = unsafe { asset_source.as_unvalidated_asset_path() };

        // Verify that the string can be parsed as an asset path before we
        // return it.
        AssetPath::try_parse(&result)
            .map(|_| ()) // drop the borrowing of result
            .map(|_| result)
    }
}

impl TryFrom<&str> for AssetSource {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if let Some(uri) = s.strip_prefix("rmf-server://") {
            Ok(AssetSource::Remote(uri.to_owned()))
        } else if let Some(uri) = s.strip_prefix("file://") {
            Ok(AssetSource::Local(uri.to_owned()))
        } else if let Some(uri) = s.strip_prefix("search://") {
            Ok(AssetSource::Search(uri.to_owned()))
        } else if let Some(uri) = s.strip_prefix("package://") {
            Ok(AssetSource::Package(uri.to_owned()))
        } else if let Some(uri) = s.strip_prefix("memory://") {
            Ok(AssetSource::Memory(uri.to_owned()))
        } else {
            Err(format!("Unsupported asset type: {}", s))
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallAssetSource {
    pub filename: Option<String>,
    pub remote_uri: Option<String>,
    pub search_name: Option<String>,
    pub bundled_name: Option<String>,
    pub package_path: Option<String>,
    pub ros_resource: Option<String>,
    pub map: Option<(i32, f32, f32)>,
}

//TODO(arjo) This is a slippery slope
impl Eq for RecallAssetSource {}

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
            AssetSource::Package(path) => {
                self.package_path = Some(path.clone());
            }
            AssetSource::Memory(path) => {
                self.ros_resource = Some(path.clone());
            }
        }
    }
}
