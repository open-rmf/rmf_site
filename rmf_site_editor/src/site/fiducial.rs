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

use crate::interaction::VisualCue;
use crate::site::*;
use bevy::prelude::*;

pub fn add_fiducial_visuals(
    mut commands: Commands,
    fiducials: Query<(Entity, &Point<Entity>), Added<FiducialMarker>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, point) in fiducials.iter() {
        if let Ok(mut deps) = dependents.get_mut(point.0) {
            deps.insert(e);
        }

        commands
            .entity(e)
            .insert(PbrBundle {
                mesh: assets.fiducial_mesh.clone(),
                material: assets.fiducial_material.clone(),
                ..default()
            })
            .insert(Category::Fiducial)
            .insert(VisualCue::outline());
    }
}

pub fn update_changed_fiducial(
    mut fiducials: Query<
        (Entity, &Point<Entity>, &mut Transform),
        (Changed<Point<Entity>>, With<FiducialMarker>),
    >,
    anchors: AnchorParams,
) {
    for (e, point, mut tf) in fiducials.iter_mut() {
        let position = anchors
            .point_in_parent_frame_of(point.0, Category::Fiducial, e)
            .unwrap();
        tf.translation = position;
    }
}

pub fn update_fiducial_for_moved_anchors(
    mut fiducials: Query<(Entity, &Point<Entity>, &mut Transform), With<FiducialMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, point, mut tf)) = fiducials.get_mut(*dependent) {
                let position = anchors
                    .point_in_parent_frame_of(point.0, Category::Fiducial, e)
                    .unwrap();
                tf.translation = position;
            }
        }
    }
}
