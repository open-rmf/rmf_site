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
    site::*,
};
use bevy::prelude::*;
use rmf_site_format::{Edge, MeasurementMarker};

pub fn add_measurement_visuals(
    mut commands: Commands,
    measurements: Query<(Entity, &Edge<Entity>), Added<MeasurementMarker>>,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    site_properties: Query<&SiteProperties>,
) {
    for (e, edge) in &measurements {
        commands
            .entity(e)
            .insert(PbrBundle {
                mesh: assets.lane_mid_mesh.clone(),
                material: assets.measurement_material.clone(),
                transform: line_stroke_transform(
                    &anchors
                        .point_in_parent_frame_of(edge.start(), Category::Measurement, e)
                        .unwrap(),
                    &anchors
                        .point_in_parent_frame_of(edge.end(), Category::Measurement, e)
                        .unwrap(),
                    site_properties.get_single().unwrap_or(&Default::default()).preferences.unwrap_or_default().default_lane_width,
                ),
                ..default()
            })
            .insert(Selectable::new(e))
            .insert(Category::Measurement)
            .insert(EdgeLabels::StartEnd);

        for anchor in &edge.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }
    }
}

fn update_measurement_visual(
    entity: Entity,
    edge: &Edge<Entity>,
    anchors: &AnchorParams,
    transform: &mut Transform,
    site_properties: &Query<&SiteProperties>,
) {
    let start_anchor = anchors
        .point_in_parent_frame_of(edge.start(), Category::Measurement, entity)
        .unwrap();
    let end_anchor = anchors
        .point_in_parent_frame_of(edge.end(), Category::Measurement, entity)
        .unwrap();
    *transform = line_stroke_transform(&start_anchor, &end_anchor, site_properties.get_single().unwrap_or(&Default::default()).preferences.unwrap_or_default().default_lane_width);
}

pub fn update_changed_measurement(
    mut measurements: Query<
        (Entity, &Edge<Entity>, &mut Transform),
        (Changed<Edge<Entity>>, With<MeasurementMarker>),
    >,
    anchors: AnchorParams,
    site_properties: Query<&SiteProperties>,
) {
    for (e, edge, mut tf) in &mut measurements {
        update_measurement_visual(e, edge, &anchors, tf.as_mut(), &site_properties);
    }
}

pub fn update_measurement_for_moved_anchors(
    mut measurements: Query<(Entity, &Edge<Entity>, &mut Transform), With<MeasurementMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    site_properties: Query<&SiteProperties>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in changed_anchor.iter() {
            if let Some((e, measurement, mut tf)) = measurements.get_mut(*dependent).ok() {
                update_measurement_visual(e, measurement, &anchors, tf.as_mut(), &site_properties);
            }
        }
    }
}
