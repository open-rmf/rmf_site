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

use bevy::prelude::*;
use rmf_site_format::{MeasurementMarker, Edge};
use crate::{
    site::*,
    interaction::Selectable,
};

pub fn add_measurement_visuals(
    mut commands: Commands,
    measurements: Query<(Entity, &Edge<Entity>), Added<MeasurementMarker>>,
    mut anchors: Query<(&GlobalTransform, &mut AnchorDependents), With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, edge) in &measurements {
        if let Ok([
            (start_anchor, mut start_dep),
            (end_anchor, mut end_dep)
        ]) = anchors.get_many_mut(edge.array()) {
            start_dep.dependents.insert(e);
            end_dep.dependents.insert(e);
            commands.entity(e)
                .insert_bundle(PbrBundle{
                    mesh: assets.lane_mid_mesh.clone(),
                    material: assets.measurement_material.clone(),
                    transform: line_stroke_transform(start_anchor, end_anchor),
                    ..default()
                })
                .insert(Selectable::new(e))
                .insert(Category("Measurement".to_string()));
        } else {
            panic!("Anchor was not initialized correctly");
        }
    }
}

fn update_measurement_visual(
    edge: &Edge<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transform: &mut Transform,
) {
    let start_anchor = anchors.get(edge[0]).unwrap();
    let end_anchor = anchors.get(edge[1]).unwrap();
    *transform = line_stroke_transform(start_anchor, end_anchor);
}

pub fn update_changed_measurement(
    mut measurements: Query<(&Edge<Entity>, &mut Transform), (Changed<Edge<Entity>>, With<MeasurementMarker>)>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
) {
    for (edge, mut tf) in &mut measurements {
        update_measurement_visual(edge, &anchors, tf.as_mut());
    }
}

pub fn update_measurement_for_changed_anchor(
    mut measurements: Query<(&Edge<Entity>, &mut Transform), With<MeasurementMarker>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((measurement, mut tf)) = measurements.get_mut(*dependent).ok() {
                update_measurement_visual(measurement, &anchors, tf.as_mut());
            }
        }
    }
}
