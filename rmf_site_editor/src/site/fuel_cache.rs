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

use crate::site::{AssetSource, ModelMarker, ModelSceneRoot, TentativeModelFormat};
use crate::site_asset_io::FUEL_API_KEY;
use crate::widgets::AssetGalleryStatus;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use crossbeam_channel::{Receiver, Sender};
use gz_fuel::{FuelClient as GzFuelClient, FuelModel};

#[derive(Resource, Clone, Default, Deref, DerefMut)]
pub struct FuelClient(GzFuelClient);

/// Event used to request an update to the fuel cache
pub struct UpdateFuelCache;

#[derive(Deref, DerefMut)]
pub struct FuelCacheUpdated(Option<Vec<FuelModel>>);

/// Event used to set the fuel API key from the UI. Will also trigger a reload for failed assets
#[derive(Deref, DerefMut)]
pub struct SetFuelApiKey(pub String);

/// Using channels instead of events to allow usage in wasm since, unlike event writers, they can
/// be cloned and moved into async functions therefore don't have lifetime issues
#[derive(Debug, Resource)]
pub struct UpdateFuelCacheChannels {
    pub sender: Sender<FuelCacheUpdated>,
    pub receiver: Receiver<FuelCacheUpdated>,
}

impl Default for UpdateFuelCacheChannels {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

pub fn handle_update_fuel_cache_requests(
    mut events: EventReader<UpdateFuelCache>,
    mut gallery_status: ResMut<AssetGalleryStatus>,
    fuel_client: Res<FuelClient>,
    channels: Res<UpdateFuelCacheChannels>,
) {
    if events.iter().last().is_some() {
        info!("Updating fuel cache");
        gallery_status.fetching_cache = true;
        let mut fuel_client = fuel_client.clone();
        let sender = channels.sender.clone();
        IoTaskPool::get()
            .spawn(async move {
                // Only write to cache in non wasm, no file system in web
                #[cfg(target_arch = "wasm32")]
                let write_to_disk = false;
                #[cfg(not(target_arch = "wasm32"))]
                let write_to_disk = true;
                // Send client if update was successful
                let res = fuel_client.update_cache(write_to_disk).await;
                sender
                    .send(FuelCacheUpdated(res))
                    .expect("Failed sending fuel cache update event");
            })
            .detach();
    }
}

pub fn read_update_fuel_cache_results(
    channels: Res<UpdateFuelCacheChannels>,
    mut fuel_client: ResMut<FuelClient>,
    mut gallery_status: ResMut<AssetGalleryStatus>,
) {
    if let Ok(result) = channels.receiver.try_recv() {
        match result.0 {
            Some(models) => fuel_client.models = Some(models),
            None => error!("Failed updating fuel cache"),
        }
        gallery_status.fetching_cache = false;
    }
}

pub fn reload_failed_models_with_new_api_key(
    mut commands: Commands,
    mut api_key_events: EventReader<SetFuelApiKey>,
    failed_models: Query<Entity, (With<ModelMarker>, Without<ModelSceneRoot>)>,
) {
    if let Some(key) = api_key_events.iter().last() {
        info!("New API Key set, attempting to re-download failed models");
        *FUEL_API_KEY.lock().unwrap() = Some((**key).clone());
        for e in &failed_models {
            commands.entity(e).insert(TentativeModelFormat::default());
        }
    }
}
