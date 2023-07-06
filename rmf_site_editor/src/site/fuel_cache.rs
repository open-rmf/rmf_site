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

use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
// TODO(luca) move FuelClient here
use crate::widgets::{AssetGalleryStatus, FuelClient};
use crossbeam_channel::{Receiver, Sender};

/// Event used to request an update to the fuel cache
pub struct UpdateFuelCache;

#[derive(Deref, DerefMut)]
pub struct FuelCacheUpdated(Option<FuelClient>);

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
    mut fuel_client: ResMut<FuelClient>,
    mut channels: ResMut<UpdateFuelCacheChannels>,
) {
    if let Some(e) = events.iter().last() {
        println!("Updating fuel cache");
        gallery_status.fetching_cache = true;
        let mut fuel_client = fuel_client.clone();
        let sender = channels.sender.clone();
        AsyncComputeTaskPool::get()
            .spawn(async move {
                // Send client if update was successful
                let res = fuel_client
                    .update_cache()
                    .await
                    .and_then(|()| Some(fuel_client));
                sender.send(FuelCacheUpdated(res));
            })
            .detach();
    }
}

pub fn read_update_fuel_cache_results(
    mut channels: ResMut<UpdateFuelCacheChannels>,
    mut fuel_client: ResMut<FuelClient>,
    mut gallery_status: ResMut<AssetGalleryStatus>,
) {
    if let Ok(result) = channels.receiver.try_recv() {
        println!("Fuel cache updated");
        if let Some(client) = result.0 {
            *fuel_client = client;
        }
        gallery_status.fetching_cache = false;
    }
}
