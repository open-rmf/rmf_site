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
use rmf_site_egui::InspectFor;

pub fn add_location_billboard_visual_cues(
    mut commands: Commands,
    mut billboards: Query<(Entity, &ChildOf), Changed<LocationBillboardMarker>>,
    points: Query<&Point<Entity>>,
    locations: Query<Entity, With<LocationTags>>,
) {
    // Updates newly spawned billboards on existing locations
    for (e, parent) in billboards.iter_mut() {
        update_location_billboard_visual_cues(&mut commands, e, parent, points, locations);
    }
}

pub fn update_location_visual_cues(
    mut commands: Commands,
    billboards: Query<(Entity, &ChildOf), With<LocationBillboardMarker>>,
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
                update_location_billboard_visual_cues(
                    &mut commands,
                    bb_entity,
                    parent,
                    points,
                    locations,
                );
            }
        }
    }
}

fn update_location_billboard_visual_cues(
    commands: &mut Commands,
    e: Entity,
    parent: &ChildOf,
    points: Query<&Point<Entity>>,
    locations: Query<Entity, With<LocationTags>>,
) {
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

pub fn update_location_billboard_hover_bubbling(
    query_billboards: Query<
        (Entity, &ChildOf, &Hovered, &Selected),
        (
            With<LocationBillboardMarker>,
            Or<(Changed<Hovered>, Changed<Selected>)>,
        ),
    >,
    mut parents: Query<
        (&mut Hovered, &mut Selected),
        (
            Without<LocationBillboardMarker>,
            Or<(With<LocationTags>, With<AnchorVisualization>)>,
        ),
    >,
) {
    for (e, parent, hovered, selected) in &query_billboards {
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
