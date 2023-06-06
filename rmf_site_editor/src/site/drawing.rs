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
        get_current_workspace_path, Anchor, DefaultFile, FiducialMarker, GlobalFloorVisibility,
        LayerVisibility, MeasurementMarker, MeasurementSegment, RecencyRank,
        DEFAULT_MEASUREMENT_OFFSET, FLOOR_LAYER_START,
    },
    CurrentWorkspace,
};
use bevy::{asset::LoadState, math::Affine3A, prelude::*};
use rmf_site_format::{
    AssetSource, Category, Drawing, IsPrimary, NameInSite, PixelsPerMeter, Pose,
};

#[derive(Bundle, Debug, Clone)]
pub struct DrawingBundle {
    pub category: Category,
    pub name: NameInSite,
    pub source: AssetSource,
    pub pose: Pose,
    pub pixels_per_meter: PixelsPerMeter,
    pub is_primary: IsPrimary,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed: ComputedVisibility,
    pub marker: DrawingMarker,
}

impl DrawingBundle {
    pub fn new(drawing: &Drawing) -> Self {
        DrawingBundle {
            category: Category::Drawing,
            name: drawing.name.clone(),
            source: drawing.source.clone(),
            pose: drawing.pose.clone(),
            pixels_per_meter: drawing.pixels_per_meter.clone(),
            is_primary: drawing.is_primary.clone(),
            transform: Transform::IDENTITY,
            global_transform: GlobalTransform::IDENTITY,
            visibility: Visibility::VISIBLE,
            computed: ComputedVisibility::default(),
            marker: DrawingMarker::default(),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct DrawingMarker;

#[derive(Debug, Clone, Copy, Default, Deref, DerefMut, Resource)]
pub struct GlobalDrawingVisibility(pub LayerVisibility);

pub const DRAWING_LAYER_START: f32 = 0.0;

// Semi transparency for drawings, more opaque than floors to make them visible
const DEFAULT_DRAWING_SEMI_TRANSPARENCY: f32 = 0.5;

/// Resource used to set what the alpha value for partially transparent drawings should be
#[derive(Clone, Resource, Deref, DerefMut)]
pub struct DrawingSemiTransparency(f32);

impl Default for DrawingSemiTransparency {
    fn default() -> Self {
        DrawingSemiTransparency(DEFAULT_DRAWING_SEMI_TRANSPARENCY)
    }
}

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
    changed_drawings: Query<
        (Entity, &AssetSource, &IsPrimary),
        (With<DrawingMarker>, Changed<AssetSource>),
    >,
    asset_server: Res<AssetServer>,
    current_workspace: Res<CurrentWorkspace>,
    site_files: Query<&DefaultFile>,
    mut default_floor_vis: ResMut<GlobalFloorVisibility>,
    drawing_transparency: Res<DrawingSemiTransparency>,
) {
    // TODO(luca) depending on when this system is executed, this function might be called between
    // the creation of the drawing and the change of the workspace, making this silently fail
    // Look into reordering systems, or adding a marker component, to make sure this doesn't happen
    let file_path = match get_current_workspace_path(current_workspace, site_files) {
        Some(file_path) => file_path,
        None => return,
    };
    for (e, source, is_primary) in &changed_drawings {
        // Append file name to path if it's a local file
        // TODO(luca) cleanup
        let asset_source = match source {
            AssetSource::Local(name) => AssetSource::Local(String::from(
                file_path.with_file_name(name).to_str().unwrap(),
            )),
            _ => source.clone(),
        };
        let texture_handle: Handle<Image> = asset_server.load(&String::from(&asset_source));
        let visibility = if is_primary.0 == true {
            LayerVisibility::Opaque
        } else {
            LayerVisibility::Alpha(**drawing_transparency)
        };
        commands
            .entity(e)
            .insert(LoadingDrawing(texture_handle))
            .insert(visibility);
    }

    if !changed_drawings.is_empty() {
        **default_floor_vis = LayerVisibility::new_semi_transparent();
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
        Option<&LayerVisibility>,
    )>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    rank: Query<&RecencyRank<DrawingMarker>>,
    segments: Query<&DrawingSegments>,
    default_drawing_vis: Res<GlobalDrawingVisibility>,
) {
    for (entity, source, pose, pixels_per_meter, handle, vis) in loading_drawings.iter() {
        match asset_server.get_load_state(&handle.0) {
            LoadState::Loaded => {
                let img = assets.get(&handle.0).unwrap();
                let width = img.texture_descriptor.size.width as f32;
                let height = img.texture_descriptor.size.height as f32;

                // We set this up so that the origin of the drawing is in the top left corner
                let mesh = make_flat_rect_mesh(width, height).transform_by(
                    Affine3A::from_translation(Vec3::new(width / 2.0, -height / 2.0, 0.0)),
                );
                let mesh = mesh_assets.add(mesh.into());

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
                let (alpha, alpha_mode) = drawing_alpha(vis, &default_drawing_vis);
                commands.entity(leaf).insert(PbrBundle {
                    mesh,
                    material: materials.add(StandardMaterial {
                        base_color_texture: Some(handle.0.clone()),
                        base_color: *Color::default().set_a(alpha),
                        alpha_mode,
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
        (
            &DrawingSegments,
            &RecencyRank<DrawingMarker>,
            Option<&Children>,
        ),
        Or<(Changed<RecencyRank<DrawingMarker>>, Changed<Children>)>,
    >,
    measurements: Query<&MeasurementSegment>,
    mut transforms: Query<&mut Transform>,
) {
    for (segments, rank, children) in &changed_rank {
        let z = drawing_layer_height(Some(rank));
        if let Ok(mut tf) = transforms.get_mut(segments.leaf) {
            tf.translation.z = z;
        }
        if let Some(children) = children {
            for child in children {
                // TODO(luca) consider adding fiducials, for now they have a thickness hence
                // are always visible
                if let Ok(segment) = measurements.get(*child) {
                    transforms
                        .get_mut(**segment)
                        .map(|mut tf| tf.translation.z = z + DEFAULT_MEASUREMENT_OFFSET)
                        .ok();
                }
            }
        }
    }
}

pub fn update_drawing_pixels_per_meter(
    mut changed_drawings: Query<(&mut Transform, &PixelsPerMeter), Changed<PixelsPerMeter>>,
) {
    for (mut tf, pixels_per_meter) in &mut changed_drawings {
        tf.scale = Vec3::new(1.0 / pixels_per_meter.0, 1.0 / pixels_per_meter.0, 1.);
    }
}

pub fn update_drawing_children_to_pixel_coordinates(
    mut commands: Commands,
    changed_drawings: Query<(&PixelsPerMeter, &Children), Changed<PixelsPerMeter>>,
    meshes: Query<Entity, Or<(With<FiducialMarker>, With<Anchor>, With<MeasurementMarker>)>>,
    mut transforms: Query<&mut Transform>,
) {
    for (pixels_per_meter, children) in changed_drawings.iter() {
        for child in children {
            if meshes.get(*child).is_ok() {
                if let Ok(mut tf) = transforms.get_mut(*child) {
                    tf.scale = Vec3::new(pixels_per_meter.0, pixels_per_meter.0, 1.0);
                } else {
                    commands
                        .entity(*child)
                        .insert(SpatialBundle::from_transform(Transform::from_scale(
                            Vec3::new(pixels_per_meter.0, pixels_per_meter.0, 1.0),
                        )));
                }
            }
        }
    }
}

fn drawing_alpha(
    specific: Option<&LayerVisibility>,
    general: &LayerVisibility,
) -> (f32, AlphaMode) {
    let alpha = specific.map(|s| s.alpha()).unwrap_or(general.alpha());
    let alpha_mode = if alpha < 1.0 {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };
    (alpha, alpha_mode)
}

fn iter_update_drawing_visibility<'a>(
    iter: impl Iterator<Item = (Option<&'a LayerVisibility>, &'a DrawingSegments)>,
    material_handles: &Query<&Handle<StandardMaterial>>,
    material_assets: &mut ResMut<Assets<StandardMaterial>>,
    default_drawing_vis: &LayerVisibility,
) {
    for (vis, segments) in iter {
        if let Ok(handle) = material_handles.get(segments.leaf) {
            if let Some(mat) = material_assets.get_mut(handle) {
                let (alpha, alpha_mode) = drawing_alpha(vis, &default_drawing_vis);
                mat.base_color = *mat.base_color.set_a(alpha);
                mat.alpha_mode = alpha_mode;
            }
        }
    }
}

// TODO(luca) RemovedComponents is brittle, maybe wrap component in an option?
// TODO(luca) This is copy-pasted from floor.rs, consider having a generic plugin
pub fn update_drawing_visibility(
    changed_floors: Query<(Option<&LayerVisibility>, &DrawingSegments), Changed<LayerVisibility>>,
    removed_vis: RemovedComponents<LayerVisibility>,
    all_floors: Query<(Option<&LayerVisibility>, &DrawingSegments)>,
    material_handles: Query<&Handle<StandardMaterial>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    default_drawing_vis: Res<GlobalDrawingVisibility>,
) {
    if default_drawing_vis.is_changed() {
        iter_update_drawing_visibility(
            all_floors.iter(),
            &material_handles,
            &mut material_assets,
            &default_drawing_vis,
        );
    } else {
        iter_update_drawing_visibility(
            changed_floors.iter(),
            &material_handles,
            &mut material_assets,
            &default_drawing_vis,
        );

        iter_update_drawing_visibility(
            removed_vis.iter().filter_map(|e| all_floors.get(e).ok()),
            &material_handles,
            &mut material_assets,
            &default_drawing_vis,
        );
    };
}
