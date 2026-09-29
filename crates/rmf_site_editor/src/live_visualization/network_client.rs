/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use rmf_site_msgs::rmf_prototype_msgs::msg::ParticipantList;
use roslibrust::rosbridge::{ClientHandle, Subscriber};
use roslibrust::RosMessageType;

const TIMEOUT_SECONDS: u64 = 2;

#[derive(Resource)]
pub struct VisualizationStreamChannel<T> {
    _sender: UnboundedSender<T>,
    pub receiver: UnboundedReceiver<T>,
}

pub trait LiveStreamHandler: Send + Sync + 'static {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connection_requested: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) where
        Self: Sized;

    fn cleanup(_world: &mut World) {}
}

#[derive(Resource, Clone, Default)]
pub struct StreamRegistry {
    spawners:
        Vec<Arc<dyn Fn(String, ClientHandle, Arc<AtomicBool>, Arc<AtomicBool>) + Send + Sync>>,
}

pub struct StreamPlugin<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for StreamPlugin<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: LiveStreamHandler> Plugin for StreamPlugin<T> {
    fn build(&self, app: &mut App) {
        let (tx, rx) = unbounded_channel();
        app.insert_resource(VisualizationStreamChannel::<T> {
            _sender: tx.clone(),
            receiver: rx,
        });

        if !app.world().contains_resource::<StreamRegistry>() {
            app.insert_resource(StreamRegistry::default());
        }

        let tx_clone = tx.clone();
        app.world_mut()
            .resource_mut::<StreamRegistry>()
            .spawners
            .push(Arc::new(
                move |robot_name, client, connection_requested, connection_active| {
                    T::spawn_stream(
                        robot_name,
                        client,
                        tx_clone.clone(),
                        connection_requested,
                        connection_active,
                    );
                },
            ));

        app.add_systems(OnEnter(crate::AppState::MainMenu), T::cleanup)
            .add_systems(
                PreUpdate,
                (|world: &mut World| {
                    let is_active = world
                        .get_resource::<super::live_state::LiveStreamState>()
                        .is_some_and(|s| s.connection_active.load(Ordering::Relaxed));
                    if !is_active {
                        T::cleanup(world);
                    }
                })
                .run_if(in_state(crate::AppState::SiteEditor)),
            );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_network_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    // Use OnceLock to lazily initialize a single shared Tokio runtime across all network tasks
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(future);
    } else {
        let rt = RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to initialize Tokio runtime")
        });
        rt.spawn(future);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_network_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;

    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
}

// On losing connection, this function kills all zombie tasks
async fn wait_until_inactive(connection_active: &Arc<AtomicBool>) {
    while connection_active.load(Ordering::Relaxed) {
        sleep(Duration::from_secs(TIMEOUT_SECONDS)).await;
    }
}

pub async fn run_subscription_loop<M, E, F>(
    sub: Subscriber<M>,
    sender: UnboundedSender<E>,
    connection_requested: Arc<AtomicBool>,
    connection_active: Arc<AtomicBool>,
    mut map_fn: F,
) where
    M: RosMessageType,
    F: FnMut(M) -> E,
{
    loop {
        let msg = tokio::select! {
            msg = sub.next() => msg,
            _ = wait_until_inactive(&connection_active) => break,
        };

        if !connection_requested.load(Ordering::Relaxed)
            || !connection_active.load(Ordering::Relaxed)
        {
            break;
        }

        if let Err(e) = sender.send(map_fn(msg)) {
            error!("Failed to send event: {}", e);
            break;
        }
    }
}

pub fn start_rosbridge_subscriber(
    ws_url: &str,
    registry: StreamRegistry,
    connection_requested: Arc<AtomicBool>,
    connection_active: Arc<AtomicBool>,
) {
    let url = ws_url.to_string();
    spawn_network_task(run_rosbridge_loop(
        url,
        registry,
        connection_requested,
        connection_active,
    ));
}

async fn run_rosbridge_loop(
    url: String,
    registry: StreamRegistry,
    connection_requested: Arc<AtomicBool>,
    connection_active: Arc<AtomicBool>,
) {
    while connection_requested.load(Ordering::Relaxed) {
        // Add timeout for initial connection
        let opts = roslibrust::rosbridge::ClientHandleOptions::new(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECONDS));

        if let Ok(client) = ClientHandle::new_with_options(opts).await {
            info!("Connected via roslibrust to {}", url);
            connection_active.store(true, Ordering::Relaxed);

            {
                let health_client = client.clone();
                let connection_active = connection_active.clone();
                let connection_requested = connection_requested.clone();

                // Async task to periodically check if the rosbridge client has disconnected
                spawn_network_task(async move {
                    loop {
                        sleep(Duration::from_secs(TIMEOUT_SECONDS)).await;

                        if !connection_requested.load(Ordering::Relaxed)
                            || !connection_active.load(Ordering::Relaxed)
                        {
                            break;
                        }

                        if health_client.is_disconnected() {
                            warn!("Rosbridge connection lost.");
                            connection_active.store(false, Ordering::Relaxed);
                            break;
                        }
                    }
                });
            }

            if let Ok(discovery_sub) = client
                .subscribe_transient_local::<ParticipantList>("/destination/discovery")
                .await
            {
                let mut subscribed_robots = HashSet::new();

                loop {
                    let msg = tokio::select! {
                        msg = discovery_sub.next() => msg,
                        _ = wait_until_inactive(&connection_active) => break,
                    };

                    if !connection_requested.load(Ordering::Relaxed)
                        || !connection_active.load(Ordering::Relaxed)
                    {
                        info!("Disconnecting from rosbridge discovery stream.");
                        break;
                    }

                    for p in msg.participants {
                        if !subscribed_robots.insert(p.name.clone()) {
                            continue;
                        }

                        info!("Subscribing to: {}", p.name);

                        for spawner in &registry.spawners {
                            spawner(
                                p.name.clone(),
                                client.clone(),
                                connection_requested.clone(),
                                connection_active.clone(),
                            );
                        }
                    }
                }
            }

            connection_active.store(false, Ordering::Relaxed);
            warn!("Connection to server lost. Attempting to reconnect...");
        } else {
            connection_active.store(false, Ordering::Relaxed);
        }

        if connection_requested.load(Ordering::Relaxed) {
            sleep(Duration::from_secs(TIMEOUT_SECONDS)).await;
        }
    }

    info!("User disconnected. Shutting down network thread.");
    connection_active.store(false, Ordering::Relaxed);
}
