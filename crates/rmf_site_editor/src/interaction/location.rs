/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
use crate::{interaction::*, site::*};
use bevy::prelude::*;
use rmf_site_camera::*;
use rmf_site_egui::{canvas_tooltips::CanvasTooltips, InspectFor};
use std::borrow::Cow;

pub fn add_billboard_visual_cues(
    mut commands: Commands,
    mut billboards: Query<(Entity, &ChildOf), Changed<BillboardMarker>>,
    points: Query<&Point<Entity>>,
    locations: Query<Entity, With<LocationTags>>,
) {
    // Updates newly spawned billboards on existing locations
    for (e, parent) in billboards.iter_mut() {
        update_billboard_visual_cues(&mut commands, e, parent, points, locations);
    }
}

pub fn update_location_visual_cues(
    mut commands: Commands,
    billboards: Query<(Entity, &ChildOf), With<BillboardMarker>>,
    points: Query<&Point<Entity>>,
    locations: Query<Entity, With<LocationTags>>,
    changed_locations: Query<
        &BillboardMeshes,
        Or<(Changed<Point<Entity>>, Changed<BillboardMeshes>)>,
    >,
) {
    // Updates billboards on newly spawned or moved locations
    for meshes in changed_locations {
        for mesh in [
            meshes.base,
            meshes.charging,
            meshes.holding,
            meshes.parking,
            meshes.empty_billboard,
        ] {
            if let Some(e) = mesh {
                let Ok((bb_entity, parent)) = billboards.get(e) else {
                    warn!("could not find billboard");
                    return;
                };
                update_billboard_visual_cues(&mut commands, bb_entity, parent, points, locations);
            }
        }
    }
}

fn update_billboard_visual_cues(
    commands: &mut Commands,
    e: Entity,
    parent: &ChildOf,
    points: Query<&Point<Entity>>,
    locations: Query<Entity, With<LocationTags>>,
) {
    commands.entity(e).insert(VisualCue::no_outline());

    if let Ok(point) = points.get(parent.0) {
        let mut drag_plane_bundle = DragPlaneBundle::new(point.0, Vec3::Z);
        drag_plane_bundle.selectable.element = e;

        if let Ok(location) = locations.get(parent.0) {
            commands.entity(e).insert(InspectFor { entity: location });
            let mut drag_plane_bundle = DragPlaneBundle::new(point.0, Vec3::Z);
            drag_plane_bundle.selectable.element = location;

            commands.entity(location).insert(drag_plane_bundle);
        }

        commands.entity(e).insert(drag_plane_bundle);
    }
}

fn new_billboard_position(billboard_vec: Vec3, camera_vec: Vec3) -> Vec3 {
    let radius = billboard_vec.length();

    if radius == 0.0 {
        return Vec3::ZERO;
    }

    let c_norm = camera_vec.normalize();
    let ad_vec = (billboard_vec - billboard_vec.dot(c_norm) * c_norm).normalize_or_zero();

    if ad_vec.length_squared() <= f32::EPSILON {
        let mut fallback_vec = Vec3::X;
        if c_norm.x.abs() > 0.9 {
            fallback_vec = Vec3::Y;
        }

        let fallback_dir = c_norm.cross(fallback_vec).normalize();
        return fallback_dir * radius;
    }
    ad_vec * radius
}

pub fn update_billboard_location(
    query_mesh: Query<(&mut Transform, &BillboardMarker)>,
    query_cameras: Query<(&Projection, &GlobalTransform)>,
    active_camera: ActiveCameraQuery,
) {
    let Ok(active_camera_entity) = active_camera_maybe(&active_camera) else {
        return;
    };
    let Ok((_camera_projection, camera_transform)) = query_cameras.get(active_camera_entity) else {
        return;
    };

    let camera_direction = camera_transform.forward().into();

    for (mut transform, marker) in query_mesh {
        let new_position: Vec3 = new_billboard_position(marker.offset, camera_direction);
        transform.translation = new_position;

        transform.rotation = Transform::IDENTITY
            .aligned_by(
                Dir3::Z,
                Dir3::new(-camera_direction).unwrap(),
                Dir3::Y,
                Dir3::new(new_position).unwrap(),
            )
            .rotation;
    }
}

pub fn update_billboard_text_hover_visualisation(
    mut tooltips: ResMut<CanvasTooltips>,
    hovering: Res<Hovering>,
    query_billboards: Query<(&Hovered, &BillboardMarker)>,
) {
    if let Some(hovering) = hovering.0 {
        if let Ok((hovered, marker)) = query_billboards.get(hovering) {
            if hovered.cue() {
                if let Some(caption_text) = &marker.caption_text {
                    tooltips.add(Cow::Owned(caption_text.clone()));
                }
            }
        }
    }
}

pub fn update_billboard_hover_visualization(
    query_billboards: Query<
        (
            Entity,
            &ChildOf,
            &Hovered,
            &Selected,
            &BillboardMarker,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Or<(Changed<Hovered>, Changed<Selected>)>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut parents: Query<
        (&mut Hovered, &mut Selected),
        (
            Without<BillboardMarker>,
            Or<(With<LocationTags>, With<AnchorVisualization>)>,
        ),
    >,
) {
    for (e, parent, hovered, selected, marker, billboard_material) in query_billboards {
        if marker.hover_enabled {
            if let Some(material) = materials.get_mut(&billboard_material.0) {
                material.alpha_mode = if hovered.cue() {
                    AlphaMode::Mask(0.1)
                } else {
                    AlphaMode::Blend
                };
            }
        }

        if let Ok((mut parent_hovered, mut parent_selected)) = parents.get_mut(parent.0) {
            if hovered.cue() {
                parent_hovered.support_hovering.insert(e);
            } else {
                parent_hovered.support_hovering.remove(&e);
            }

            if selected.cue() {
                parent_selected.support_selected.insert(e);
            } else {
                parent_selected.support_selected.remove(&e);
            }
        }
    }
}
