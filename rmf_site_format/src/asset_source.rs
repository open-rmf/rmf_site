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
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum AssetSource {
    Local(String),
    Remote(String),
}

impl AssetSource {
    pub fn label(&self) -> &str {
        match self {
            Self::Local(_) => "Local",
            Self::Remote(_) => "Remote",
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
        if path.starts_with("rmf-server://") {
            let without_prefix = path.to_str().unwrap().strip_prefix("rmf-server://").unwrap();
            return AssetSource::Remote(String::from(without_prefix));
        }
        else if path.starts_with("file://") {
            println!("Prestripping {}", path.to_str().unwrap());
            let without_prefix = path.to_str().unwrap().strip_prefix("file://").unwrap();
            println!("Poststripping {}", without_prefix);
            return AssetSource::Local(String::from(without_prefix));
        }
        AssetSource::default()
    }
}

/*
impl From<AssetSource> for PathBuf {
    fn from(asset_source: AssetSource) -> PathBuf {
        match asset_source {
            AssetSource::Remote(uri) => { 
                let mut buf = PathBuf::new();
                buf.push("rmf-server://");
                buf.push(uri);
                println!("Buf is {}", buf.display().to_string());
                buf
            }
            AssetSource::Local(filename) => { 
                let mut buf = PathBuf::new();
                buf.push("file://");
                buf.push(filename);
                buf
            }
        }
    }
}
*/

impl From<AssetSource> for String {
    fn from(asset_source: AssetSource) -> String {
        match asset_source {
            AssetSource::Remote(uri) => { 
                String::from("rmf-server://") + &uri
            }
            AssetSource::Local(filename) => { 
                let ret = String::from("file://") + &filename;
                println!("Converted local path is {}", ret);
                ret
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallAssetSource {
    pub filename: Option<String>,
    pub remote_uri: Option<String>,
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
        }
    }
}
