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

use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::utils::BoxedFuture;

use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;

use thiserror::Error;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<SdfLoader>().add_asset::<SdfRoot>();
    }
}

// TODO(luca) remove most of this when a crate for SDF Yaserde support is available
#[derive(Component, YaDeserialize, YaSerialize, Default, Debug, TypeUuid, Clone)]
#[uuid = "fe707f9e-c6f3-11ed-afa2-0242ac120002"]
#[yaserde(rename = "sdf")]
pub struct SdfRoot {
    pub model: SdfModel,
    #[yaserde(skip)]
    // TODO(make this a AssetSource)
    pub path: String,
}

#[derive(YaDeserialize, YaSerialize, Default, Debug, Clone)]
pub struct SdfModel {
    pub link: Vec<SdfLink>,
    // TODO(luca) scales, collisions
}

#[derive(YaDeserialize, YaSerialize, Default, Debug, Clone)]
pub struct SdfLink {
    pub visual: Vec<SdfVisual>,
}

#[derive(YaDeserialize, YaSerialize, Default, Debug, Clone)]
pub struct SdfVisual {
    pub geometry: SdfGeometry,
}

#[derive(YaDeserialize, YaSerialize, Debug, Clone)]
pub struct SdfMesh {
    pub uri: String,
    pub scale: Option<String>,
}

#[derive(YaDeserialize, YaSerialize, Debug, Clone)]
pub enum SdfGeometry {
    #[yaserde(rename = "mesh")]
    Mesh(SdfMesh),
}

impl Default for SdfGeometry {
    fn default() -> Self {
        SdfGeometry::Mesh(SdfMesh {
            uri: String::new(),
            scale: None,
        })
    }
}

#[derive(Default)]
struct SdfLoader;

impl AssetLoader for SdfLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move { Ok(load_model(bytes, load_context).await?) })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["sdf"];
        EXTENSIONS
    }
}

#[derive(Error, Debug)]
pub enum SdfError {
    #[error("Yaserde loading error: {0}")]
    YaserdeError(String),
}

async fn load_model<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
) -> Result<(), SdfError> {
    let sdf_str = std::str::from_utf8(bytes).unwrap();
    let mut sdf = yaserde::de::from_str::<SdfRoot>(sdf_str);
    match sdf {
        Ok(mut sdf) => {
            sdf.path = load_context.path().clone().to_str().unwrap().to_string();
            load_context.set_default_asset(LoadedAsset::new(sdf));
            Ok(())
        }
        Err(err) => Err(SdfError::YaserdeError(err)),
    }
}
