/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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
use bevy::prelude::{Component, Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extensions {
    /// Hook for downstream extensions to put their own serialized data into
    /// the site file.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<Arc<str>, serde_json::Value>,
}

/// Settings for how the site editor interacts with each extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionSettings {
    /// Always skip this extension while saving.
    ///
    /// This is false by default.
    #[serde(default, skip_serializing_if = "is_false")]
    pub skip_during_save: bool,

    /// When true true, saving will fail if this extension returns an error
    /// while saving. When false, a warning will be printed if this extension
    /// produces an error while saving.
    ///
    /// This is false by default.
    #[serde(default, skip_serializing_if = "is_false")]
    pub prevent_saving_on_error: bool,

    /// When true, if this extension is missing from the application that tries
    /// to load the site or if an error occurs in this extension while trying
    /// the site, then the site will not be allowed to load at all.
    ///
    /// If this is false, then the site will load as much as it can and only
    /// print a warning in situations where this extension is missing or produces
    /// an error while loading.
    ///
    /// This is false by default.
    #[serde(default, skip_serializing_if = "is_false")]
    pub prevent_loading_on_error: bool,
}

impl Default for ExtensionSettings {
    fn default() -> Self {
        Self {
            skip_during_save: false,
            prevent_saving_on_error: false,
            prevent_loading_on_error: false,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]

pub struct SiteExtensionSettings(BTreeMap<Arc<str>, ExtensionSettings>);
