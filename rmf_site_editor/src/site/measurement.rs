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

use crate::{interaction::Selectable, site::*};
use bevy::prelude::*;
use rmf_site_format::{Edge, MeasurementMarker};

pub fn add_measurement_visuals(
    mut commands: Commands,
    measurements: Query<(Entity, &Edge<Entity>), Added<MeasurementMarker>>,
    anchors: AnchorParams,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
) {
    for (e, edge) in &measurements {
        commands
            .entity(e)
            .insert_bundle(PbrBundle {
                mesh: assets.lane_mid_mesh.clone(),
                material: assets.measurement_material.clone(),
                transform: line_stroke_transform(
                    &anchors.point_in_parent_frame_of(edge.start(), Category::Measurement, e).unwrap(),
                    &anchors.point_in_parent_frame_of(edge.end(), Category::Measurement, e).unwrap(),
                    LANE_WIDTH,
                ),
                ..default()
            })
            .insert(Selectable::new(e))
            .insert(Category::Measurement)
            .insert(EdgeLabels::StartEnd);

        for anchor in &edge.array() {
            if let Ok(mut dep) = dependents.get_mut(*anchor) {
                dep.dependents.insert(e);
            }
        }
    }
}

fn update_measurement_visual(
    entity: Entity,
    edge: &Edge<Entity>,
    anchors: &AnchorParams,
    transform: &mut Transform,
) {
    let start_anchor = anchors.point_in_parent_frame_of(edge.start(), Category::Measurement, entity).unwrap();
    let end_anchor = anchors.point_in_parent_frame_of(edge.end(), Category::Measurement, entity).unwrap();
    *transform = line_stroke_transform(&start_anchor, &end_anchor, LANE_WIDTH);
}

pub fn update_changed_measurement(
    mut measurements: Query<
        (Entity, &Edge<Entity>, &mut Transform),
        (Changed<Edge<Entity>>, With<MeasurementMarker>),
    >,
    anchors: AnchorParams,
) {
    for (e, edge, mut tf) in &mut measurements {
        update_measurement_visual(e, edge, &anchors, tf.as_mut());
    }
}

pub fn update_measurement_for_changed_anchor(
    mut measurements: Query<(Entity, &Edge<Entity>, &mut Transform), With<MeasurementMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((e, measurement, mut tf)) = measurements.get_mut(*dependent).ok() {
                update_measurement_visual(e, measurement, &anchors, tf.as_mut());
            }
        }
    }
}
