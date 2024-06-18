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

use crate::site::{ModelMarker, ModelSceneRoot, TentativeModelFormat};
use crate::site_asset_io::FUEL_API_KEY;
use crate::widgets::AssetGalleryStatus;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use crossbeam_channel::{Receiver, Sender};
use gz_fuel::{FuelClient as GzFuelClient, FuelModel};

#[derive(Resource, Clone, Default, Deref, DerefMut)]
pub struct FuelClient(GzFuelClient);

/// Event used to request an update to the fuel cache
#[derive(Event)]
pub struct UpdateFuelCache;

#[derive(Deref, DerefMut)]
pub struct FuelCacheUpdated(Option<Vec<FuelModel>>);

/// Event used to set the fuel API key from the UI. Will also trigger a reload for failed assets
#[derive(Deref, DerefMut, Event)]
pub struct SetFuelApiKey(pub String);

/// Using channels instead of events to allow usage in wasm since, unlike event writers, they can
/// be cloned and moved into async functions therefore don't have lifetime issues
#[derive(Debug, Resource)]
pub struct FuelCacheUpdateChannel {
    pub sender: Sender<FuelCacheUpdated>,
    pub receiver: Receiver<FuelCacheUpdated>,
}

impl Default for FuelCacheUpdateChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

/// Channels that give incremental updates about what models have been fetched.
#[derive(Debug, Resource)]
pub struct FuelCacheProgressChannel {
    pub sender: Sender<FuelModel>,
    pub receiver: Receiver<FuelModel>,
}

impl Default for FuelCacheProgressChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

pub fn handle_update_fuel_cache_requests(
    mut events: EventReader<UpdateFuelCache>,
    mut gallery_status: ResMut<AssetGalleryStatus>,
    fuel_client: Res<FuelClient>,
    update_channel: Res<FuelCacheUpdateChannel>,
    progress_channel: Res<FuelCacheProgressChannel>,
) {
    if events.read().last().is_some() {
        info!("Updating fuel cache, this might take a few minutes");
        gallery_status.fetching_cache = true;
        let mut fuel_client = fuel_client.clone();
        let sender = update_channel.sender.clone();
        let progress = progress_channel.sender.clone();
        IoTaskPool::get()
            .spawn(async move {
                // Only write to cache in non wasm, no file system in web
                #[cfg(target_arch = "wasm32")]
                let write_to_disk = false;
                #[cfg(not(target_arch = "wasm32"))]
                let write_to_disk = true;
                // Send client if update was successful

                let res = fuel_client
                    .update_cache_with_progress(write_to_disk, Some(progress))
                    .await;
                sender
                    .send(FuelCacheUpdated(res))
                    .expect("Failed sending fuel cache update event");
            })
            .detach();
    }

    while let Ok(next_model) = progress_channel.receiver.try_recv() {
        info!(
            "Received model {} owned by {}",
            next_model.name, next_model.owner,
        );
    }
}

pub fn read_update_fuel_cache_results(
    channels: Res<FuelCacheUpdateChannel>,
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
    if let Some(key) = api_key_events.read().last() {
        info!("New API Key set, attempting to re-download failed models");
        let mut key_guard = match FUEL_API_KEY.lock() {
            Ok(key) => key,
            Err(poisoned) => poisoned.into_inner(),
        };
        *key_guard = Some((**key).clone());
        for e in &failed_models {
            commands.entity(e).insert(TentativeModelFormat::default());
        }
    }
}
