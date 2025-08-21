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

use crate::site::*;
use bevy::prelude::*;
use rmf_site_camera::{CameraTarget, PanToElement};
use rmf_site_picking::DoubleClickSelect;

pub fn register_double_click_event(
    mut double_click: EventReader<DoubleClickSelect>,
    mut pan_to: ResMut<PanToElement>,
) {
    for double_click_entity in double_click.read() {
        let Some(target_entity) = double_click_entity.0 else {
            return;
        };
        pan_to.target = Some(target_entity);
    }
}

fn calculate_new_target(
    entity: Entity,
    points: Query<&Point<Entity>>,
    edges: Query<&Edge<Entity>>,
    paths: Query<&Path<Entity>>,
    transforms: Query<&GlobalTransform>,
) -> Vec3 {
    //check points
    if let Ok(point) = points.get(entity) {
        let Ok(anchor) = transforms.get(point.0) else {
            return Vec3::ZERO;
        };
        return anchor.translation();
    //check edges
    } else if let Ok(edge) = edges.get(entity) {
        let (Ok(start_anchor), Ok(end_anchor)) =
            (transforms.get(edge.start()), transforms.get(edge.end()))
        else {
            return Vec3::ZERO;
        };
        let middle = (start_anchor.translation() + end_anchor.translation()) * 0.5;
        return middle;

    //check path
    } else if let Ok(path) = paths.get(entity) {
        let anchors = &path.0;

        if anchors.iter().count() == 0 {
            return Vec3::ZERO;
        }
        let mut total_position = Vec3::ZERO;

        for anchor in anchors.iter() {
            let Ok(global_transform) = transforms.get(*anchor) else {
                return Vec3::ZERO;
            };
            total_position += global_transform.translation();
        }
        let average_anchor = total_position / anchors.iter().count() as f32;
        return average_anchor;
    } else {
        return Vec3::ZERO;
    }
}

pub fn update_camera_targets(
    mut commands: Commands,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    changed_elements: Query<
        Entity,
        Or<(
            Changed<Point<Entity>>,
            Changed<Edge<Entity>>,
            Changed<Path<Entity>>,
        )>,
    >,
    points: Query<&Point<Entity>>,
    edges: Query<&Edge<Entity>>,
    paths: Query<&Path<Entity>>,
    transforms: Query<&GlobalTransform>,
) {
    for e in &changed_elements {
        let new_target = calculate_new_target(e, points, edges, paths, transforms);
        commands
            .entity(e)
            .insert(CameraTarget { point: new_target });
    }

    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            let new_target = calculate_new_target(*dependent, points, edges, paths, transforms);
            commands
                .entity(*dependent)
                .insert(CameraTarget { point: new_target });
        }
    }
}
