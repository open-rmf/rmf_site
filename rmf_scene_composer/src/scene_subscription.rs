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

use crate::{
    protos::gz::msgs::Scene,
    generate_scene,
};

use rmf_site_format::*;

use librmf_site_editor::interaction::{
    InteractionAssets, DragPlaneBundle, OutlineVisualization, VisualCue,
};

use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};

use bevy_impulse::*;
use std::{
    borrow::Cow,
    error::Error,
    sync::Arc,
};
use thiserror::Error as ThisError;
use zenoh::Session;
use prost::Message;

use futures::channel::oneshot::{channel, Sender, Receiver};

#[derive(SystemParam)]
pub struct SceneSubscriber<'w, 's> {
    commands: Commands<'w, 's>,
    workflow: Res<'w, SceneSubscriptionWorkflow>,
    subscriptions: Query<'w, 's, &'static mut SceneSubscription>,
}

impl<'w, 's> SceneSubscriber<'w, 's> {
    pub fn spawn_subscriber(
        &mut self,
        topic_name: String,
    ) -> Entity {
        let scene_root = self.commands.spawn((
            SpatialBundle::INHERITED_IDENTITY,
            Category::Custom(Cow::Borrowed("Scene")),
            OutlineVisualization::default(),
            VisualCue::outline(),
        )).id();

        self.commands.entity(scene_root).insert(
            DragPlaneBundle::new(scene_root, Vec3::Z).globally()
        );

        let (drop_last_subscription, subscription_dropped) = channel();

        let subscription = self.commands.request(
            SceneSubscriptionRequest {
                topic_name: topic_name.clone(),
                scene_root,
                subscription_dropped,
            },
            self.workflow.service,
        ).take_response();

        self.commands.entity(scene_root).insert(SceneSubscription {
            topic_name,
            subscription,
            drop_last_subscription: Some(drop_last_subscription),
        });

        scene_root
    }

    pub fn change_subscription(
        &mut self,
        scene_root: Entity,
        new_topic_name: String,
    ) {
        if let Ok(mut scene) = self.subscriptions.get_mut(scene_root) {
            if scene.topic_name == new_topic_name {
                // Topic name has not changed, so there is no need to do anything
                return;
            }

            scene.drop_last_subscription.take().map(|s| s.send(()));

            let (drop_last_subscription, subscription_dropped) = channel();

            let new_subscription = self.commands.request(
                SceneSubscriptionRequest {
                    topic_name: new_topic_name.clone(),
                    scene_root,
                    subscription_dropped,
                },
                self.workflow.service,
            ).take_response();

            scene.topic_name = new_topic_name;
            scene.subscription = new_subscription;
            scene.drop_last_subscription = Some(drop_last_subscription);
        } else {
            let (drop_last_subscription, subscription_dropped) = channel();

            // Somehow this entity wasn't already a scene... this is suspicious,
            // but we'll just spawn a new subscription for it.
            let subscription = self.commands.request(
                SceneSubscriptionRequest {
                    topic_name: new_topic_name.clone(),
                    scene_root,
                    subscription_dropped,
                },
                self.workflow.service,
            ).take_response();

            self.commands.entity(scene_root).insert(SceneSubscription {
                topic_name: new_topic_name,
                subscription,
                drop_last_subscription: Some(drop_last_subscription),
            });
        }
    }

    pub fn get_subscription(&self, scene_root: Entity) -> Option<&SceneSubscription> {
        self.subscriptions.get(scene_root).ok()
    }
}

#[derive(Component)]
pub struct SceneSubscription {
    topic_name: String,
    #[allow(unused)]
    subscription: Promise<()>,
    // TODO(@mxgrey): This should not be necessary
    // when https://github.com/open-rmf/bevy_impulse/issues/64 is resolved
    drop_last_subscription: Option<Sender<()>>,
}

impl SceneSubscription {
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }
}

#[derive(Default)]
pub(crate) struct SceneSubscribingPlugin {}

type ArcError = Arc<dyn Error + Send + Sync + 'static>;

impl Plugin for SceneSubscribingPlugin {
    fn build(&self, app: &mut App) {
        let get_session = app.spawn_continuous_service(
            PreUpdate,
            |
                In(r): ContinuousServiceInput<(), _>,
                mut requests: ContinuousQuery<(), Result<Arc<Session>, ArcError>>,
                mut session_res: ResMut<ZenohSession>,
            | {
                // TODO(@mxgrey): Investigate whether this continuous service
                // can be replaced with a single-producer, multiple-receiver
                // with latching. Perhaps that can be implemented in Promise
                // itself.
                let Some(mut queue) = requests.get_mut(&r.key) else {
                    return;
                };

                if queue.is_empty() {
                    return;
                }

                queue.for_each(|order| {
                    let session_peak = match session_res.session.peek() {
                        PromiseState::Available(session) => session,
                        PromiseState::Pending => {
                            return;
                        }
                        PromiseState::Cancelled(cancelled) => {
                            order.respond(Err(Arc::new(cancelled.clone())));
                            return;
                        }
                        PromiseState::Disposed => {
                            order.respond(Err(Arc::new(SessionPromiseError::Disposed)));
                            return;
                        }
                        PromiseState::Taken => {
                            order.respond(Err(Arc::new(SessionPromiseError::Taken)));
                            return;
                        }
                    };

                    order.respond(session_peak.clone());
                });
            }
        );

        let service = app.world.spawn_io_workflow(move |scope, builder| {
            let error_node = builder.create_map_block(|error: ArcError| {
                error!("{error}");
            });

            let basic_scene_visual = builder.commands().spawn_service(
                set_basic_scene_visual.into_blocking_service()
            );

            let on_error = error_node.input;
            builder.connect(error_node.output, scope.terminate);

            let subscription_node = scope
                .input
                .chain(builder)
                .then(basic_scene_visual)
                .branch_for_err(|err: Chain<()>| err.connect(scope.terminate))
                .map_block(|r| (r, ()))
                .fork_unzip(
                    (
                        |chain: Chain<_>| chain.output(),
                        |chain: Chain<_>| {
                            chain
                                .then(get_session)
                                .fork_result(
                                    |ok| ok.output(),
                                    |err| err.connect(on_error),
                                )
                                .0
                        }
                    )
                )
                .join(builder)
                .map_node(|input: AsyncMap<(SceneSubscriptionRequest, Arc<Session>), StreamOf<SceneUpdate>>| {
                    async move {
                        let (request, session) = input.request;
                        let SceneSubscriptionRequest { topic_name, scene_root, subscription_dropped: mut drop_subscription } = request;

                        let subscriber = session.declare_subscriber(&topic_name).await?;
                        loop {
                            let sample = match futures::future::select(
                                subscriber.recv_async(),
                                drop_subscription,
                            ).await {
                                futures::future::Either::Left((sample, d)) => {
                                    drop_subscription = d;
                                    sample
                                }
                                futures::future::Either::Right(_) => return Ok(()),
                            };

                            match sample {
                                Ok(sample) => {
                                    match Scene::decode(&*sample.payload().to_bytes()) {
                                        Ok(scene) => {
                                            input.streams.send(StreamOf(SceneUpdate { scene_root, scene }));
                                        }
                                        Err(err) => {
                                            error!("Error decoding incoming scene message on topic [{topic_name}]: {err}");
                                        }
                                    }
                                }
                                Err(err) => {
                                    return Err::<(), ArcError>(Arc::from(err));
                                }
                            }
                        }
                    }
                });

            subscription_node
                .output
                .chain(builder)
                .fork_result(
                    |ok| ok.connect(scope.terminate),
                    |err| err.connect(on_error),
                );

            subscription_node
                .streams
                .chain(builder)
                .inner()
                .then(generate_scene.into_blocking_callback())
                .unused();
        });

        app
            .init_resource::<ZenohSession>()
            .insert_resource(SceneSubscriptionWorkflow { service });
    }
}

fn set_basic_scene_visual(
    In(request): In<SceneSubscriptionRequest>,
    world: &mut World,
) -> Result<SceneSubscriptionRequest, ()> {
    let scene_root = request.scene_root;

    // Clear any pre-existing children
    world.get_entity_mut(request.scene_root).ok_or(())?.despawn_descendants();

    // Insert the basic axes
    world.resource_scope::<InteractionAssets, _>(|world, interaction_assets| {
        world.command(|mut commands| {
            // Make an initial set of axes to visualize the scene while we wait for
            // the data to arrive.
            let axes = interaction_assets.make_orientation_cue_meshes(
                &mut commands,
                scene_root,
                1.0,
            );
            // Allow the axes to be selected and dragged to move the scene around
            for axis in axes {
                commands.entity(axis).insert((
                    DragPlaneBundle::new(scene_root, Vec3::Z).globally(),
                    OutlineVisualization::default(),
                    VisualCue::outline(),
                ));
            }
        });
    });

    Ok(request)
}

#[derive(ThisError, Debug)]
enum SessionPromiseError {
    #[error("Zenoh session was disposed")]
    Disposed,
    #[error("Zenoh session was taken")]
    Taken,
}

#[derive(Resource)]
struct ZenohSession {
    session: Promise<Result<Arc<Session>, ArcError>>,
}

impl FromWorld for ZenohSession {
    fn from_world(world: &mut World) -> Self {
        let session = world.command(|commands| {
            commands
                .serve(async {
                    zenoh::open(zenoh::Config::default())
                        .await
                        .map(Arc::new)
                        .map_err(Arc::from)
                })
                .take_response()
        });

        Self { session }
    }
}

#[derive(Resource)]
pub(crate) struct SceneSubscriptionWorkflow {
    service: Service<SceneSubscriptionRequest, ()>,
}

#[derive(Debug)]
struct SceneSubscriptionRequest {
    topic_name: String,
    scene_root: Entity,
    subscription_dropped: Receiver<()>,
}

pub(crate) struct SceneUpdate {
    pub(crate) scene_root: Entity,
    pub(crate) scene: Scene,
}
