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
    site::{Category, PreventDeletion},
};
use bevy::{prelude::*};
use bevy::utils::HashMap;
use rmf_site_format::{Drawing, DrawingMarker, DrawingSource, Pose};
use rmf_site_format::{Rotation, Angle};

// We need to keep track of the drawing data until the image is loaded
// since we will need to scale the mesh according to the size of the image
// TODO Loading textures might need similar behavior if they are not square
#[derive(Default)]
pub struct LoadingDrawings(pub HashMap<Handle<Image>, (Entity, Pose)>);

pub fn add_drawing_visuals(
    mut commands: Commands,
    new_drawings: Query<(Entity, &DrawingSource, &Pose), Added<DrawingMarker>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut loading_drawings: ResMut<LoadingDrawings>,
) {
    for (e, source, pose) in &new_drawings {
        let texture_path = match source {
            DrawingSource::Filename(name) => name
        };
        // TODO texture path local to map file
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
            if let Some((entity, pose)) = (*loading_drawings).0.get(handle) {
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
                .insert(Selectable::new(*entity))
                .insert(Category("Drawing".to_string()));
            }
        }
    }
}
