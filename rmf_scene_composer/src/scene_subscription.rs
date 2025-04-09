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
use librmf_site_editor::site::ModelLoader;

#[derive(SystemParam)]
pub struct SceneSubscriber<'w, 's> {
    commands: Commands<'w, 's>,
    workflow: Res<'w, SceneSubscriptionWorkflow>,
    interaction_assets: Res<'w, InteractionAssets>,
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

        // Make an initial set of axes to visualize the scene while we wait for
        // the data to arrive.
        let axes = self.interaction_assets.make_orientation_cue_meshes(
            &mut self.commands,
            scene_root,
            1.0,
        );
        // Allow the axes to be selected and dragged to move the scene around
        for axis in axes {
            self.commands.entity(axis).insert((
                DragPlaneBundle::new(scene_root, Vec3::Z).globally(),
                OutlineVisualization::default(),
                VisualCue::outline(),
            ));
        }

        let subscription = self.commands.request(
            SceneSubscriptionRequest {
                topic_name: topic_name.clone(),
                root: scene_root,
            },
            self.workflow.service,
        ).take_response();

        self.commands.entity(scene_root).insert(SceneSubscription {
            topic_name,
            subscription,
        });

        scene_root
    }
}

#[derive(Component)]
pub struct SceneSubscription {
    topic_name: String,
    #[allow(unused)]
    subscription: Promise<()>,
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

            let on_error = error_node.input;
            builder.connect(error_node.output, scope.terminate);

            let subscription_node = scope
                .input
                .chain(builder)
                .fork_clone((
                    |chain: Chain<_>| chain.output(),
                    |chain: Chain<_>| {
                        chain
                            .trigger()
                            .then(get_session)
                            .fork_result(
                                |ok| ok.output(),
                                |err| err.connect(on_error),
                            )
                            .0
                    }
                ))
                .join(builder)
                .map_node(|input: AsyncMap<(SceneSubscriptionRequest, Arc<Session>), StreamOf<SceneUpdate>>| {
                    async move {
                        let (request, session) = input.request;
                        let SceneSubscriptionRequest { topic_name, root } = request;
                        let subscriber = session.declare_subscriber(&topic_name).await?;
                        loop {
                            match subscriber.recv_async().await {
                                Ok(sample) => {
                                    match Scene::decode(&*sample.payload().to_bytes()) {
                                        Ok(scene) => {
                                            input.streams.send(StreamOf(SceneUpdate { root, scene }));
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
                .then(generate_scene_sys.into_blocking_callback())
                .unused();
        });

        app
            .init_resource::<ZenohSession>()
            .insert_resource(SceneSubscriptionWorkflow { service });
    }
}

// TODO(@mxgrey): Replace this with a direct call to generate_scene
fn generate_scene_sys(
    In(SceneUpdate { root, scene }): In<SceneUpdate>,
    mut commands: Commands,
    mut model_loader: ModelLoader,
    children: Query<&Children>,
) {
    // Despawn any old children to clear space for the new scene
    if let Ok(children) = children.get(root) {
        for child in children {
            if let Some(e) = commands.get_entity(*child) {
                e.despawn_recursive();
            }
        }
    }

    generate_scene(root, scene, &mut commands, &mut model_loader);
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

#[derive(Clone, Debug)]
struct SceneSubscriptionRequest {
    topic_name: String,
    root: Entity,
}

struct SceneUpdate {
    root: Entity,
    scene: Scene,
}
