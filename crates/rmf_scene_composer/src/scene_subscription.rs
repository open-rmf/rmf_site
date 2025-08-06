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

use crate::generate_scene;

use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_impulse::{Promise, Service, *};
use example_interfaces::srv::*;
use futures::channel::oneshot::{channel, Receiver, Sender};
use librmf_site_editor::interaction::{DragPlaneBundle, InteractionAssets, OutlineVisualization};
use rclrs::*;
use rmf_site_format::*;
use rmf_site_picking::VisualCue;
use std::{borrow::Cow, error::Error, sync::Arc};
use visualization_msgs::msg::Marker;

#[derive(SystemParam)]
pub struct SceneSubscriber<'w, 's> {
    commands: Commands<'w, 's>,
    workflow: Res<'w, SceneSubscriptionWorkflow>,
    subscriptions: Query<'w, 's, &'static mut SceneSubscription>,
}

impl<'w, 's> SceneSubscriber<'w, 's> {
    pub fn spawn_scene(&mut self, topic_name: String) -> Entity {
        let scene_root = self
            .commands
            .spawn((
                Transform::IDENTITY,
                Visibility::Inherited,
                Category::Custom(Cow::Borrowed("Scene")),
                OutlineVisualization::default(),
                VisualCue::outline(),
            ))
            .id();

        self.commands
            .entity(scene_root)
            .insert(DragPlaneBundle::new(scene_root, Vec3::Z).globally());

        let (drop_last_subscription, subscription_dropped) = channel();

        let subscription = self
            .commands
            .request(
                SceneSubscriptionRequest {
                    topic_name: topic_name.clone(),
                    scene_root,
                    subscription_dropped,
                },
                self.workflow.service,
            )
            .take_response();

        self.commands.entity(scene_root).insert(SceneSubscription {
            topic_name,
            subscription,
            drop_last_subscription: Some(drop_last_subscription),
        });

        scene_root
    }

    pub fn change_subscription(&mut self, scene_root: Entity, new_topic_name: String) {
        if let Ok(mut scene) = self.subscriptions.get_mut(scene_root) {
            if scene.topic_name == new_topic_name {
                // Topic name has not changed, so there is no need to do anything
                return;
            }

            scene.drop_last_subscription.take().map(|s| s.send(()));

            let (drop_last_subscription, subscription_dropped) = channel();

            let new_subscription = self
                .commands
                .request(
                    SceneSubscriptionRequest {
                        topic_name: new_topic_name.clone(),
                        scene_root,
                        subscription_dropped,
                    },
                    self.workflow.service,
                )
                .take_response();

            scene.topic_name = new_topic_name;
            scene.subscription = new_subscription;
            scene.drop_last_subscription = Some(drop_last_subscription);
        } else {
            let (drop_last_subscription, subscription_dropped) = channel();

            // Somehow this entity wasn't already a scene... this is suspicious,
            // but we'll just spawn a new subscription for it.
            let subscription = self
                .commands
                .request(
                    SceneSubscriptionRequest {
                        topic_name: new_topic_name.clone(),
                        scene_root,
                        subscription_dropped,
                    },
                    self.workflow.service,
                )
                .take_response();

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
        let service = app.world_mut().spawn_io_workflow(move |scope, builder| {
            let error_node = builder.create_map_block(|error: ArcError| {
                error!("{error}");
            });

            let basic_scene_visual = builder
                .commands()
                .spawn_service(set_basic_scene_visual.into_blocking_service());

            let on_error = error_node.input;
            builder.connect(error_node.output, scope.terminate);

            let subscription_node = scope
                .input
                .chain(builder)
                .then(basic_scene_visual)
                .branch_for_err(|err: Chain<()>| err.connect(scope.terminate))
                .map_block(|r| (r, ()))
                .map_node(
                    |input: AsyncMap<(SceneSubscriptionRequest, ()), StreamOf<RosMesh>>| {
                        async move {
                            let (request, _) = input.request;
                            let SceneSubscriptionRequest {
                                topic_name,
                                scene_root,
                                subscription_dropped: mut drop_subscription,
                            } = request;

                            // TODO(@xiyuoh) Get rid of the ugly hack
                            // Currently we're creating a new executor everytime this workflow is triggered
                            // because the executor cannot be safely passed into .map_node()
                            // Using until_promise_resolved to drop "expired" node subscriptions, but all created
                            // nodes will stay. Find a way to have only one node up.

                            let context = Context::default_from_env().unwrap();
                            let mut executor = context.create_basic_executor();
                            let node_name = format!("marker_subscriber_{}", topic_name.clone());
                            let node = executor.create_node(&node_name).unwrap();

                            // Create client for resource retriever service
                            // TODO(@xiyuoh) Update srv used here
                            let client = node.create_client::<AddTwoInts>("add_two_ints").unwrap();

                            let logger = node.logger().clone();
                            log!(&logger, "Attempting to subscribe to topic: {}", topic_name);

                            let mut receiver = node
                                .create_subscription_receiver::<Marker>(topic_name.as_str())
                                .unwrap();
                            log!(
                                &logger,
                                "Waiting to receive markers from topic {}...",
                                topic_name
                            );
                            let promise = executor.commands().run(async move {
                                while let Some(msg) = receiver.recv().await {
                                    println!("# I heard: '{}'", msg.id);
                                    log!(&logger, "Received a msg: id={}, ns={}", msg.id, msg.ns);
                                    log!(&logger, "Received a mesh: {}", msg.mesh_resource);
                                    let mesh_resource = msg.mesh_resource.clone();

                                    // Send mesh resource to service
                                    let srv_request = AddTwoInts_Request { a: 41, b: 1 };

                                    let srv_response: AddTwoInts_Response =
                                        client.call(&srv_request).unwrap().await.unwrap();
                                    println!(
                                        "Result of {} + {} is: {}",
                                        srv_request.a, srv_request.b, srv_response.sum,
                                    );
                                    let sum = srv_response.sum;
                                    // Send mesh to streams
                                    input.streams.send(StreamOf(RosMesh { mesh_resource, sum }));
                                }
                            });

                            std::thread::spawn(move || {
                                executor.spin(
                                    SpinOptions::new().until_promise_resolved(drop_subscription),
                                )
                            });
                        }
                    },
                );

            // TODO(@xiyuoh) Set up service call to WorldBridge to retrieve GLTF mesh file

            subscription_node
                .streams
                .chain(builder)
                .inner()
                .then(generate_scene.into_blocking_callback())
                .unused();
        });

        app.register_type::<Name>() // TypeRegistrationPlugin no longer exists
            .insert_resource(SceneSubscriptionWorkflow { service });
    }
}

fn set_basic_scene_visual(
    In(request): In<SceneSubscriptionRequest>,
    world: &mut World,
) -> Result<SceneSubscriptionRequest, ()> {
    let scene_root = request.scene_root;

    // Clear any pre-existing children
    world
        .get_entity_mut(request.scene_root)
        .map_err(|_| ())?
        .despawn_related::<Children>();

    // Insert the basic axes
    world.resource_scope::<InteractionAssets, _>(|world, interaction_assets| {
        world.command(|mut commands| {
            // Make an initial set of axes to visualize the scene while we wait for
            // the data to arrive.
            let axes =
                interaction_assets.make_orientation_cue_meshes(&mut commands, scene_root, 1.0);
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

pub(crate) struct RosMesh {
    pub(crate) mesh_resource: String,
    pub(crate) sum: i64,
}
