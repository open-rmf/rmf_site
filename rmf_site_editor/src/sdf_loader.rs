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
use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::BoxedFuture;

use thiserror::Error;

use sdformat_rs::SdfModel;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<SdfLoader>().add_asset::<SdfRoot>();
    }
}

#[derive(Component, Default, Debug, TypeUuid, TypePath, Clone)]
#[uuid = "fe707f9e-c6f3-11ed-afa2-0242ac120002"]
pub struct SdfRoot {
    pub model: SdfModel,
    // TODO(make this a AssetSource)
    pub path: String,
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
    #[error("No <model> tag found in model.sdf file")]
    MissingModelTag,
}

async fn load_model<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
) -> Result<(), SdfError> {
    let sdf_str = std::str::from_utf8(bytes).unwrap();
    let root = sdformat_rs::from_str::<sdformat_rs::SdfRoot>(sdf_str);
    match root {
        Ok(root) => {
            if let Some(model) = root.model {
                let path = load_context.path().to_str().unwrap().to_string();
                let sdf_root = SdfRoot { model, path };
                load_context.set_default_asset(LoadedAsset::new(sdf_root));
                Ok(())
            } else {
                Err(SdfError::MissingModelTag)
            }
        }
        Err(err) => Err(SdfError::YaserdeError(err)),
    }
}
