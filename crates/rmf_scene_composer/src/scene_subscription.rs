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

use crate::generate_scene::*;

use bevy::{
    ecs::{schedule::ScheduleConfigs, system::ScheduleSystem, system::SystemParam},
    prelude::*,
};
use bevy_impulse::{Promise, Service, *};
use futures::channel::oneshot::{channel, Receiver, Sender};
use librmf_site_editor::interaction::{DragPlaneBundle, InteractionAssets, OutlineVisualization};
use librmf_site_editor::site::SiteUpdateSet;
use rclrs::*;
use rmf_site_format::*;
use rmf_site_picking::VisualCue;
use rviz_interfaces::srv::*;
use std::{borrow::Cow, error::Error, sync::Arc};
use visualization_msgs::msg::{Marker, MarkerArray};

#[derive(SystemParam)]
pub struct SceneSubscriber<'w, 's> {
    commands: Commands<'w, 's>,
    children: Query<'w, 's, &'static Children>,
    workflow: Res<'w, SceneSubscriptionWorkflow>,
    subscriptions: Query<'w, 's, &'static mut SceneSubscription>,
    subscription_node: Res<'w, SceneSubscriptionNode>,
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
        let node = self.subscription_node.clone();
        // Despawn any old children to clear space for the new scene
        if let Ok(children) = self.children.get(scene_root) {
            for child in children {
                if let Ok(mut e) = self.commands.get_entity(*child) {
                    e.despawn();
                }
            }
        }

        let subscription = self
            .commands
            .request(
                SceneSubscriptionRequest {
                    topic_name: topic_name.clone(),
                    scene_root,
                    subscription_dropped,
                    node,
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
            let node = self.subscription_node.clone();
            // Despawn any old children to clear space for the new scene
            if let Ok(children) = self.children.get(scene_root) {
                for child in children {
                    if let Ok(mut e) = self.commands.get_entity(*child) {
                        e.despawn();
                    }
                }
            }

            let new_subscription = self
                .commands
                .request(
                    SceneSubscriptionRequest {
                        topic_name: new_topic_name.clone(),
                        scene_root,
                        subscription_dropped,
                        node,
                    },
                    self.workflow.service,
                )
                .take_response();

            scene.topic_name = new_topic_name;
            scene.subscription = new_subscription;
            scene.drop_last_subscription = Some(drop_last_subscription);
        } else {
            let (drop_last_subscription, subscription_dropped) = channel();
            let node = self.subscription_node.clone();
            // Despawn any old children to clear space for the new scene
            if let Ok(children) = self.children.get(scene_root) {
                for child in children {
                    if let Ok(mut e) = self.commands.get_entity(*child) {
                        e.despawn();
                    }
                }
            }

            // Somehow this entity wasn't already a scene... this is suspicious,
            // but we'll just spawn a new subscription for it.
            let subscription = self
                .commands
                .request(
                    SceneSubscriptionRequest {
                        topic_name: new_topic_name.clone(),
                        scene_root,
                        subscription_dropped,
                        node,
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

#[derive(Resource, Deref)]
pub struct RclrsExecutorCommands(Arc<ExecutorCommands>);

#[derive(Default)]
pub(crate) struct RclrsPlugin {}

impl Plugin for RclrsPlugin {
    fn build(&self, app: &mut App) {
        let mut executor = Context::default_from_env().unwrap().create_basic_executor();
        app.insert_resource(RclrsExecutorCommands(Arc::clone(executor.commands())));

        std::thread::spawn(move || executor.spin(SpinOptions::default()));
    }
}

#[derive(Resource, Deref)]
pub struct SceneSubscriptionNode(Arc<NodeState>);

#[derive(Default)]
pub(crate) struct SceneSubscribingPlugin {}

type ArcError = Arc<dyn Error + Send + Sync + 'static>;

impl Plugin for SceneSubscribingPlugin {
    fn build(&self, app: &mut App) {
        let executor_commands = app.world().resource::<RclrsExecutorCommands>();
        let node = executor_commands.create_node("marker_subscriber").unwrap();
        app.insert_resource(SceneSubscriptionNode(node));

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
                        let (request, _) = input.request;
                        let SceneSubscriptionRequest {
                            topic_name,
                            scene_root,
                            subscription_dropped: mut drop_subscription,
                            node,
                        } = request;

                        // TODO(@xiyuoh) Get rid of the ugly hack
                        // Currently we're creating a new executor everytime this workflow is triggered
                        // because for some reason await gets stuck when we use a node created out of this scope.

                        let context = Context::default_from_env().unwrap();
                        let mut executor = context.create_basic_executor();
                        let node_name = format!("marker_subscriber_{}", topic_name.clone());
                        let node = executor.create_node(&node_name).unwrap();
                        std::thread::spawn(move || executor.spin(SpinOptions::default()));

                        let node = node.clone();
                        async move {
                            let logger = node.logger().clone();

                            let mut receiver = node
                                .create_subscription_receiver::<MarkerArray>(
                                    topic_name.as_str().transient_local(),
                                )
                                .unwrap();

                            let client = node
                                .create_client::<GetResource>("workcell_1/rviz_get_resource")
                                .unwrap();
                            client.notify_on_service_ready().await;
                            log!(&logger, "GetResource service is ready!");

                            loop {
                                let sample = match futures::future::select(
                                    Box::pin(receiver.recv()),
                                    drop_subscription,
                                )
                                .await
                                {
                                    futures::future::Either::Left((sample, d)) => {
                                        drop_subscription = d;
                                        sample
                                    }
                                    futures::future::Either::Right(_) => {
                                        return Ok(());
                                    }
                                };

                                match sample {
                                    Some(msg) => {
                                        for marker in msg.markers.iter() {
                                            let mut marker = marker.clone();
                                            input.streams.send(StreamOf(RosMesh {
                                                scene_root,
                                                marker,
                                                node: node.clone(),
                                                client: client.clone(),
                                            }));
                                        }
                                    }
                                    None => {
                                        return Err(());
                                    }
                                }
                            }
                        }
                    },
                );

            subscription_node.output.chain(builder).fork_result(
                // TODO(@xiyuoh) proper error handling
                |ok| ok.connect(scope.terminate),
                |err| err.connect(scope.terminate),
            );

            let retrieve_mesh_data = builder.commands().spawn_service(retrieve_mesh_data);

            let retrieval_node = subscription_node
                .streams
                .chain(builder)
                .inner()
                .then_node(retrieve_mesh_data);

            retrieval_node.output.chain(builder).unused();
            retrieval_node
                .streams
                .chain(builder)
                .inner()
                .then(generate_mesh.into_blocking_callback())
                .unused();
        });

        app.register_type::<Name>() // TypeRegistrationPlugin no longer exists
            .insert_resource(SceneSubscriptionWorkflow { service });
    }
}

fn strip_resource_prefix(resource: &String, prefix: Vec<&str>) -> String {
    let mut mesh_resource = resource.clone();
    for p in prefix.iter() {
        if let Some(stripped) = mesh_resource.strip_prefix(p) {
            mesh_resource = stripped.to_string();
            break;
        };
    }
    mesh_resource
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
    node: Arc<NodeState>,
}

#[derive(Clone)]
pub(crate) struct RosMesh {
    pub(crate) scene_root: Entity,
    pub(crate) marker: Marker,
    pub(crate) node: Arc<NodeState>,
    pub(crate) client: Arc<ClientState<GetResource>>,
}
