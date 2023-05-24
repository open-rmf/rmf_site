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
    site::{
        get_current_workspace_path, Anchor, Category, DefaultFile, FiducialMarker, FloorVisibility,
        RecencyRank, FLOOR_LAYER_START,
    },
    CurrentWorkspace,
};
use bevy::{asset::LoadState, math::Affine3A, prelude::*};
use rmf_site_format::{AssetSource, DrawingMarker, PixelsPerMeter, Pose};

pub const DRAWING_LAYER_START: f32 = 0.0;

#[derive(Debug, Clone, Copy, Component)]
pub struct DrawingSegments {
    leaf: Entity,
}

// We need to keep track of the drawing data until the image is loaded
// since we will need to scale the mesh according to the size of the image
#[derive(Component)]
pub struct LoadingDrawing(Handle<Image>);

fn drawing_layer_height(rank: Option<&RecencyRank<DrawingMarker>>) -> f32 {
    rank.map(|r| r.proportion() * (FLOOR_LAYER_START - DRAWING_LAYER_START) + DRAWING_LAYER_START)
        .unwrap_or(DRAWING_LAYER_START)
}

pub fn add_drawing_visuals(
    mut commands: Commands,
    new_drawings: Query<(Entity, &AssetSource), (With<DrawingMarker>, Changed<AssetSource>)>,
    asset_server: Res<AssetServer>,
    current_workspace: Res<CurrentWorkspace>,
    site_files: Query<&DefaultFile>,
    mut default_floor_vis: ResMut<FloorVisibility>,
) {
    // TODO(luca) depending on when this system is executed, this function might be called between
    // the creation of the drawing and the change of the workspace, making this silently fail
    // Look into reordering systems, or adding a marker component, to make sure this doesn't happen
    let file_path = match get_current_workspace_path(current_workspace, site_files) {
        Some(file_path) => file_path,
        None => return,
    };
    for (e, source) in &new_drawings {
        // Append file name to path if it's a local file
        // TODO(luca) cleanup
        let asset_source = match source {
            AssetSource::Local(name) => AssetSource::Local(String::from(
                file_path.with_file_name(name).to_str().unwrap(),
            )),
            _ => source.clone(),
        };
        let texture_handle: Handle<Image> = asset_server.load(&String::from(&asset_source));
        commands.entity(e).insert(LoadingDrawing(texture_handle));
    }

    if !new_drawings.is_empty() {
        *default_floor_vis = FloorVisibility::new_semi_transparent();
    }
}

// Asset event handler for loaded drawings
pub fn handle_loaded_drawing(
    mut commands: Commands,
    assets: Res<Assets<Image>>,
    loading_drawings: Query<(
        Entity,
        &AssetSource,
        &Pose,
        &PixelsPerMeter,
        &LoadingDrawing,
    )>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    children: Query<&Children>,
    mut anchors: Query<&mut Anchor>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    rank: Query<&RecencyRank<DrawingMarker>>,
    segments: Query<&DrawingSegments>,
) {
    for (entity, source, pose, pixels_per_meter, handle) in loading_drawings.iter() {
        match asset_server.get_load_state(&handle.0) {
            LoadState::Loaded => {
                let img = assets.get(&handle.0).unwrap();
                let width = img.texture_descriptor.size.width as f32;
                let height = img.texture_descriptor.size.height as f32;

                //let centering_vec = Vec3::new(width / 2.0, -height / 2.0, 0.0);
                let mut centering_vec = Vec3::new(0.0, 0.0, 0.0);

                // We set this up so that the origin of the drawing is in
                let mesh = make_flat_rect_mesh(width, height)
                    .transform_by(Affine3A::from_translation(centering_vec));
                let mesh = mesh_assets.add(mesh.into());

                centering_vec = Vec3::new(-width / 2.0, height / 2.0, 0.0);

                // Also translate the child anchors to center them
                if let Ok(children) = children.get(entity) {
                    for child in children.iter() {
                        if let Ok(mut anchor) = anchors.get_mut(*child) {
                            let original_tf = anchor.local_transform(Category::General);
                            anchor.move_to(
                                &(Transform::from_translation(centering_vec) * original_tf),
                            );
                        }
                    }
                }

                let leaf = if let Ok(segment) = segments.get(entity) {
                    segment.leaf
                    // We can ignore the layer height here since that update
                    // will be handled by another system.
                } else {
                    let mut cmd = commands.entity(entity);
                    let leaf = cmd.add_children(|p| p.spawn_empty().id());

                    cmd.insert(DrawingSegments { leaf })
                        .insert(SpatialBundle::from_transform(pose.transform().with_scale(
                            Vec3::new(1.0 / pixels_per_meter.0, 1.0 / pixels_per_meter.0, 1.),
                        )))
                        .insert(Selectable::new(entity));
                    leaf
                };
                let z = drawing_layer_height(rank.get(entity).ok());
                commands.entity(leaf).insert(PbrBundle {
                    mesh,
                    material: materials.add(StandardMaterial {
                        base_color_texture: Some(handle.0.clone()),
                        ..default()
                    }),
                    transform: Transform::from_xyz(0.0, 0.0, z),
                    ..default()
                });
                commands.entity(entity).remove::<LoadingDrawing>();
            }
            LoadState::Failed => {
                error!("Failed loading drawing {:?}", String::from(source));
                commands.entity(entity).remove::<LoadingDrawing>();
            }
            _ => {}
        }
    }
}

pub fn update_drawing_rank(
    changed_rank: Query<
        (&DrawingSegments, &RecencyRank<DrawingMarker>),
        Changed<RecencyRank<DrawingMarker>>,
    >,
    mut transforms: Query<&mut Transform>,
) {
    for (segments, rank) in &changed_rank {
        if let Ok(mut tf) = transforms.get_mut(segments.leaf) {
            let z = drawing_layer_height(Some(rank));
            tf.translation.z = z;
        }
    }
}

pub fn update_drawing_pixels_per_meter(
    mut changed_drawings: Query<(&mut Transform, &PixelsPerMeter), Changed<PixelsPerMeter>>,
) {
    for (mut tf, pixels_per_meter) in changed_drawings.iter_mut() {
        tf.scale = Vec3::new(1.0 / pixels_per_meter.0, 1.0 / pixels_per_meter.0, 1.);
    }
}

pub fn update_anchor_and_fiducial_visuals_for_changed_pixels_per_meter(
    changed_drawings: Query<(Entity, &PixelsPerMeter), Changed<PixelsPerMeter>>,
    visuals: Query<
        Entity,
        (
            Without<DrawingMarker>,
            Or<(With<Anchor>, With<FiducialMarker>)>,
        ),
    >,
    children: Query<&Children>,
    mut transforms: Query<&mut Transform>,
) {
    for (e, pixels_per_meter) in changed_drawings.iter() {
        for child in DescendantIter::new(&children, e) {
            if visuals.get(child).is_ok() {
                if let Ok(mut tf) = transforms.get_mut(child) {
                    tf.scale = Vec3::new(pixels_per_meter.0, pixels_per_meter.0, 1.0);
                }
            }
        }
    }
}
