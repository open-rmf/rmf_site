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
use librmf_site_editor::interaction::DragPlaneBundle;
use librmf_site_editor::site::Model as SiteModel;
use librmf_site_editor::site::Pose as SitePose;
use librmf_site_editor::site::{
    AssetSource, IsStatic, ModelLoader, ModelMarker, NameInSite, Rotation, Scale, VisualMeshMarker,
};
use librmf_site_editor::site_asset_io::MemoryDir;
use rclrs::*;
use rviz_interfaces::srv::*;
use std::{collections::VecDeque, future::Future, path::Path, sync::Arc};
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
    mut commands: Commands,
    mut model_loader: ModelLoader,
    children: Query<&Children>,
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
        let mut promise = client
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
    mem_dir: ResMut<MemoryDir>,
) {
    mem_dir
        .dir
        .insert_asset(Path::new(&marker.mesh_resource), data.clone());

    let name = marker.ns;
    let pos = marker.pose.position;
    let ori = marker.pose.orientation;
    let scale = marker.scale;
    let asset_source = AssetSource::Ros(marker.mesh_resource.to_string());

    let mesh_entity = commands
        .spawn(SiteModel {
            name: NameInSite(name.to_owned()),
            source: asset_source.clone(),
            pose: SitePose {
                trans: [pos.x as f32, pos.y as f32, pos.z as f32],
                rot: Rotation::Quat([ori.x as f32, ori.y as f32, ori.z as f32, ori.w as f32]),
            },
            is_static: IsStatic(false),
            scale: Scale(Vec3::new(scale.x as f32, scale.y as f32, scale.z as f32)),
            marker: ModelMarker,
        })
        .insert(ChildOf(scene_root))
        .id();

    let interaction = DragPlaneBundle::new(scene_root, Vec3::Z).globally();

    println!("Loading model for {}...", name);
    model_loader
        .update_asset_source_impulse(mesh_entity, asset_source, Some(interaction.clone()))
        .detach();
}
