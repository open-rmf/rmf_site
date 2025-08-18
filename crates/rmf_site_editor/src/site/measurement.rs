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

use crate::{layers, site::*};
use bevy::prelude::*;
use rmf_site_format::{Edge, MeasurementMarker};
use rmf_site_picking::Selectable;

/// Stores which (child) entity contains the measurement mesh
#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct MeasurementSegment(pub Entity);

pub fn add_measurement_visuals(
    mut commands: Commands,
    measurements: Query<(Entity, &Edge<Entity>, Option<&Transform>), Added<MeasurementMarker>>,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, edge, tf) in &measurements {
        let mut transform = line_stroke_transform(
            &anchors
                .point_in_parent_frame_of(edge.start(), Category::Measurement, e)
                .unwrap(),
            &anchors
                .point_in_parent_frame_of(edge.end(), Category::Measurement, e)
                .unwrap(),
            LANE_WIDTH,
        );
        // TODO(luca) proper layering rather than hardcoded
        transform.translation.z = layers::ZLayer::Measurement.to_z();

        let child_id = commands
            .spawn((
                Mesh3d(assets.lane_mid_mesh.clone()),
                MeshMaterial3d(assets.measurement_material.clone()),
                transform,
                Visibility::default(),
            ))
            .insert(Selectable::new(e))
            .id();

        if tf.is_none() {
            commands
                .entity(e)
                .insert((Transform::IDENTITY, Visibility::Inherited));
        }

        commands
            .entity(e)
            .insert(Category::Measurement)
            .insert(MeasurementSegment(child_id))
            .insert(EdgeLabels::StartEnd)
            .add_children(&[child_id]);

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
) {
    let start_anchor = anchors
        .point_in_parent_frame_of(edge.start(), Category::Measurement, entity)
        .unwrap();
    let end_anchor = anchors
        .point_in_parent_frame_of(edge.end(), Category::Measurement, entity)
        .unwrap();
    let z = transform.translation.z;
    *transform = line_stroke_transform(&start_anchor, &end_anchor, LANE_WIDTH);
    transform.translation.z = z;
}

pub fn update_changed_measurement(
    measurements: Query<
        (&Edge<Entity>, &MeasurementSegment),
        (Changed<Edge<Entity>>, With<MeasurementMarker>),
    >,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
) {
    for (edge, segment) in &measurements {
        if let Ok(mut tf) = transforms.get_mut(**segment) {
            update_measurement_visual(**segment, edge, &anchors, tf.as_mut());
        }
    }
}

pub fn update_measurement_for_moved_anchors(
    measurements: Query<(&Edge<Entity>, &MeasurementSegment), With<MeasurementMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut transforms: Query<&mut Transform>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in changed_anchor.iter() {
            if let Some((measurement, segment)) = measurements.get(*dependent).ok() {
                if let Ok(mut tf) = transforms.get_mut(**segment) {
                    update_measurement_visual(**segment, measurement, &anchors, tf.as_mut());
                }
            }
        }
    }
}
