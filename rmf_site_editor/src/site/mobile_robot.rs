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

use crate::{
    interaction::{DragPlaneBundle, Selectable, MODEL_PREVIEW_LAYER},
    site::{
        Affiliation, AssetSource, Category, CurrentLevel, Group, ModelMarker, ModelProperty,
        NameInSite, Pending, Pose, PreventDeletion, Scale, SiteAssets, SiteParent,
    },
    site_asset_io::MODEL_ENVIRONMENT_VARIABLE,
};
use bevy::{
    asset::{LoadState, LoadedUntypedAsset},
    gltf::Gltf,
    prelude::*,
    render::view::RenderLayers,
};
use bevy_mod_outline::OutlineMeshExt;
use smallvec::SmallVec;
use std::any::TypeId;

#[derive(Resource, Clone)]
pub struct PoseGoal {
    pub pose: Pose,
}
