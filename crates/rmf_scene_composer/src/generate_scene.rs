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

use crate::RosMesh;

use bevy::prelude::*;
use bevy_impulse::*;
use librmf_site_editor::{
    interaction::DragPlaneBundle,
    site::{
        Affiliation, AssetSource, Category, Group, IsStatic, Model as SiteModel,
        ModelDescriptionBundle, ModelLoader, ModelMarker, ModelProperty, NameInSite,
        Pose as SitePose, Rotation, Scale, VisualMeshMarker,
    },
    site_asset_io::MemoryDir,
    workspace::CurrentWorkspace,
};
use rviz_interfaces::srv::*;
use std::{future::Future, path::Path};
use thiserror::Error;
use visualization_msgs::msg::Marker;

#[derive(Error, Debug)]
pub enum SceneLoadingError {
    #[error("Unable to retrieve AssetSource from Mesh filename: {0}")]
    MeshFilenameNotFound(String),
    #[error("No geometry found: {0}")]
    GeometryNotFound(String),
}

pub fn retrieve_mesh_data(
    In(AsyncService {
        request, streams, ..
    }): AsyncServiceInput<RosMesh, StreamOf<RosMeshData>>,
) -> impl Future<Output = ()> {
    let RosMesh {
        scene_root,
        marker,
        node,
        client,
    } = request;
    let marker = marker.clone();
    async move {
        let srv_request = GetResource_Request {
            path: marker.mesh_resource.clone(),
            etag: String::new(),
        };

        // TODO(@xiyuoh) We only have a fraction of response back, update query queue on world bridge
        let _ = client
            .call_then(&srv_request, move |srv_response: GetResource_Response| {
                println!(
                    "Receiving GetResource response, status: {}, len: [{}]",
                    srv_response.status_code,
                    srv_response.body.len()
                );
                if srv_response.status_code == 1 {
                    streams.send(StreamOf(RosMeshData {
                        data: srv_response.body,
                        marker,
                        scene_root,
                    }));
                }
            })
            .unwrap();
    }
}

pub struct RosMeshData {
    pub data: Vec<u8>,
    pub marker: Marker,
    pub scene_root: Entity,
}

pub fn generate_mesh(
    In(RosMeshData {
        data,
        marker,
        scene_root,
    }): In<RosMeshData>,
    mut commands: Commands,
    mut model_loader: ModelLoader,
    current_workspace: Res<CurrentWorkspace>,
    asset_server: Res<AssetServer>,
    mem_dir: ResMut<MemoryDir>,
    model_descriptions: Query<(Entity, &NameInSite), (With<ModelMarker>, With<Group>)>,
) {
    let mut mesh_resource_path = marker.ns.clone();
    mesh_resource_path.push_str(".glb");

    let name = marker.ns;
    let pos = marker.pose.position;
    let ori = marker.pose.orientation;
    let scale = {
        let sc = marker.scale;
        Scale(Vec3::new(sc.x as f32, sc.y as f32, sc.z as f32))
    };
    let asset_source = AssetSource::Ros(mesh_resource_path.clone());

    let mut description_entity: Option<Entity> = None;
    for (e, model_description_name) in model_descriptions.iter() {
        if name == model_description_name.0 {
            description_entity = Some(e);
            break;
        }
    }
    if description_entity.is_none() {
        let Some(site_entity) = current_workspace.root else {
            return;
        };
        let description = ModelDescriptionBundle {
            name: NameInSite(name.clone()),
            source: ModelProperty(asset_source.clone()),
            is_static: ModelProperty(IsStatic::default()),
            scale: ModelProperty(scale.clone()),
            ..Default::default()
        };

        description_entity = Some(
            commands
                .spawn(description)
                .insert(Category::ModelDescription)
                .insert(ChildOf(site_entity))
                .id(),
        );

        mem_dir
            .dir
            .insert_asset(Path::new(&mesh_resource_path), data.clone());
    }

    let mesh_entity = commands
        .spawn(SiteModel {
            name: NameInSite(name.to_owned()),
            source: asset_source.clone(),
            pose: SitePose {
                trans: [pos.x as f32, pos.y as f32, pos.z as f32],
                rot: Rotation::Quat([ori.x as f32, ori.y as f32, ori.z as f32, ori.w as f32]),
            },
            is_static: IsStatic(false),
            scale,
            marker: ModelMarker,
        })
        .insert(ChildOf(scene_root))
        .insert(Affiliation(description_entity))
        .id();

    let interaction = DragPlaneBundle::new(scene_root, Vec3::Z).globally();
    println!("Loading model for {}...", mesh_resource_path);
    model_loader
        .update_asset_source_impulse(mesh_entity, asset_source, Some(interaction.clone()))
        .detach();
}
