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
    shapes::make_flat_rect_mesh,
    site::{Category, CurrentSite, DefaultFile},
};
use bevy::{math::Affine3A, prelude::*, utils::HashMap};
use rmf_site_format::{AssetSource, DrawingMarker, PixelsPerMeter, Pose};

use std::path::{Path, PathBuf};

// We need to keep track of the drawing data until the image is loaded
// since we will need to scale the mesh according to the size of the image
#[derive(Default)]
pub struct LoadingDrawings(pub HashMap<Handle<Image>, (Entity, Pose, PixelsPerMeter)>);

fn get_current_site_path(
    current_site: Res<CurrentSite>,
    site_files: Query<&DefaultFile>,
) -> Option<PathBuf> {
    let site_entity = (*current_site).0?;
    site_files.get(site_entity).map(|f| f.0.clone()).ok()
}

pub fn add_drawing_visuals(
    new_drawings: Query<(Entity, &AssetSource, &Pose, &PixelsPerMeter), Added<DrawingMarker>>,
    asset_server: Res<AssetServer>,
    mut loading_drawings: ResMut<LoadingDrawings>,
    current_site: Res<CurrentSite>,
    site_files: Query<&DefaultFile>,
) {
    // TODO support for remote sources
    let mut file_path = match get_current_site_path(current_site, site_files) {
        Some(file_path) => file_path,
        None => return,
    };
    for (e, source, pose, pixels_per_meter) in &new_drawings {
        // Append file name to path if it's a local file
        // TODO cleanup
        let asset_source = match source {
            AssetSource::Local(name) => AssetSource::Local(String::from(file_path.with_file_name(name).to_str().unwrap())),
            AssetSource::Remote(uri) => source.clone(),
        };
        let texture_path = String::from(asset_source.clone());
        println!("Loading texture path {}", &texture_path);
        let texture_handle: Handle<Image> = asset_server.load(&String::from(asset_source));
        loading_drawings
            .0
            .insert(texture_handle, (e, pose.clone(), pixels_per_meter.clone()));
    }
}

// Asset event handler for loaded drawings
pub fn handle_loaded_drawing(
    mut commands: Commands,
    mut ev_asset: EventReader<AssetEvent<Image>>,
    assets: Res<Assets<Image>>,
    mut loading_drawings: ResMut<LoadingDrawings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for ev in ev_asset.iter() {
        if let AssetEvent::Created { handle } = ev {
            if let Some((entity, pose, pixels_per_meter)) = loading_drawings.0.remove(handle) {
                let img = assets.get(handle).unwrap();
                let width = img.texture_descriptor.size.width as f32;
                let height = img.texture_descriptor.size.height as f32;
                // We set this up so that the origin of the drawing is in
                let mut mesh = make_flat_rect_mesh(width, height).transform_by(
                    Affine3A::from_translation(Vec3::new(width / 2.0, -height / 2.0, 0.0)),
                );
                // TODO Z layering
                let mut pose = pose.clone();
                let transform = pose.transform().with_scale(Vec3::new(
                    1.0 / pixels_per_meter.0,
                    1.0 / pixels_per_meter.0,
                    1.,
                ));

                commands
                    .entity(entity.clone())
                    .insert_bundle(PbrBundle {
                        mesh: meshes.add(mesh.into()),
                        material: materials.add(StandardMaterial {
                            base_color_texture: Some(handle.clone()),
                            ..default()
                        }),
                        transform,
                        ..default()
                    })
                    .insert(Selectable::new(entity))
                    .insert(Category::Drawing);
            }
        }
    }
}

pub fn update_drawing_asset_source(
    changed_drawings: Query<(Entity, &AssetSource, &Pose, &PixelsPerMeter), Changed<AssetSource>>,
    asset_server: Res<AssetServer>,
    mut loading_drawings: ResMut<LoadingDrawings>,
    current_site: Res<CurrentSite>,
    site_files: Query<&DefaultFile>,
) {
    let file_path = match get_current_site_path(current_site, site_files) {
        Some(file_path) => file_path,
        None => return,
    };
    for (e, source, pose, pixels_per_meter) in &changed_drawings {
        // TODO cleanup
        let asset_source = match source {
            AssetSource::Local(name) => AssetSource::Local(String::from(file_path.with_file_name(name).to_str().unwrap())),
            AssetSource::Remote(uri) => source.clone(),
        };
        let texture_handle: Handle<Image> = asset_server.load(&String::from(asset_source));
        loading_drawings
            .0
            .insert(texture_handle, (e, pose.clone(), pixels_per_meter.clone()));
    }
}

pub fn update_drawing_pixels_per_meter(
    mut changed_drawings: Query<(&mut Transform, &PixelsPerMeter), Changed<PixelsPerMeter>>,
) {
    for (mut tf, pixels_per_meter) in &mut changed_drawings {
        tf.scale = Vec3::new(1.0 / pixels_per_meter.0, 1.0 / pixels_per_meter.0, 1.);
    }
}
