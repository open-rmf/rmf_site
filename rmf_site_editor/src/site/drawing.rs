/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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
    interaction::Selectable,
    shapes::{make_flat_square_mesh, make_flat_rectangle_mesh},
    site::{Category, CurrentSite, DefaultFile},
};
use bevy::{prelude::*};
use bevy::utils::HashMap;
use rmf_site_format::{Drawing, DrawingMarker, AssetSource, Pose};

use std::path::PathBuf;

// We need to keep track of the drawing data until the image is loaded
// since we will need to scale the mesh according to the size of the image
// TODO Loading textures might need similar behavior if they are not square
#[derive(Default)]
pub struct LoadingDrawings(pub HashMap<Handle<Image>, (Entity, Pose)>);

fn get_current_site_path(
    current_site: Res<CurrentSite>,
    site_files: Query<(Entity, &DefaultFile)>,
) -> Option<PathBuf> {
    let site_entity = (*current_site).0.unwrap();
    let site_file = site_files.iter().find(| &el | el.0 == site_entity);
    match site_file {
        Some((_, file_path)) => Some(file_path.0.clone()),
        None => None
    }
}

pub fn add_drawing_visuals(
    new_drawings: Query<(Entity, &AssetSource, &Pose), Added<DrawingMarker>>,
    asset_server: Res<AssetServer>,
    mut loading_drawings: ResMut<LoadingDrawings>,
    current_site: Res<CurrentSite>,
    site_files: Query<(Entity, &DefaultFile)>,
) {
    let file_path = get_current_site_path(current_site, site_files);
    if file_path.is_none() {
        return;
    }
    for (e, source, pose) in &new_drawings {
        let texture_path = match source {
            AssetSource::Filename(name) => {
                file_path.as_ref().unwrap().with_file_name(name)
            }
        };
        let texture_handle: Handle<Image> = asset_server.load(texture_path);
        (*loading_drawings).0.insert(texture_handle, (e, pose.clone()));
    }
}

// Asset event handler for loaded drawings
pub fn handle_loaded_drawing(mut commands: Commands,
    mut ev_asset: EventReader<AssetEvent<Image>>,
    mut assets: ResMut<Assets<Image>>,
    mut loading_drawings: ResMut<LoadingDrawings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for ev in ev_asset.iter() {
        if let AssetEvent::Created { handle } = ev {
            if let Some((entity, pose)) = (*loading_drawings).0.remove(handle) {
                let img = assets.get(handle).unwrap();
                let width = img.texture_descriptor.size.width as f32;
                let height = img.texture_descriptor.size.height as f32;
                let aspect_ratio = width / height;
                // TODO pixel per meter conversion to set scale
                let mut mesh = Mesh::from(make_flat_rectangle_mesh(10.0, 10.0 * aspect_ratio));
                let uvs: Vec<[f32; 2]> = [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]].into_iter().cycle().take(8).collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                commands.entity(entity.clone()).insert_bundle(PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials.add(StandardMaterial {
                            base_color_texture: Some(handle.clone()),
                            ..default()
                        }),
                    // TODO Set Z to avoid z fighting on ground plane
                    transform: pose.transform(),
                    ..default()
                })
                .insert(Selectable::new(entity))
                .insert(Category("Drawing".to_string()));
            }
        }
    }
}

pub fn update_drawing_visuals(
    changed_drawings: Query<(Entity, &AssetSource, &Pose), Changed<AssetSource>>,
    asset_server: Res<AssetServer>,
    mut loading_drawings: ResMut<LoadingDrawings>,
    current_site: Res<CurrentSite>,
    site_files: Query<(Entity, &DefaultFile)>,
) {
    let file_path = get_current_site_path(current_site, site_files);
    if file_path.is_none() {
        return;
    }
    // If the file source was updated through the UI it will be an absolute path
    // hence it can be loaded straightaway
    for (e, source, pose) in &changed_drawings {
        let texture_path = match source {
            AssetSource::Filename(name) => name
        };
        let texture_handle: Handle<Image> = asset_server.load(texture_path);
        (*loading_drawings).0.insert(texture_handle, (e, pose.clone()));
    }
}
