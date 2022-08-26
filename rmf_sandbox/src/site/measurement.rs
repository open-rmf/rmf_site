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
use rmf_site_format::Measurement;
use crate::{
    site::*,
    interaction::Selectable,
};

pub fn init_measurements(
    mut commands: Commands,
    measurements: Query<Entity, Added<Measurement<Entity>>>,
) {
    for e in &measurements {
        commands.entity(e).insert(Pending);
    }
}

pub fn add_measurement_visuals(
    mut commands: Commands,
    measurements: Query<(Entity, &Measurement<Entity>), With<Pending>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, new_measurement) in &measurements {
        if let (Ok(start_anchor), Ok(end_anchor)) = (
            anchors.get(new_measurement.anchors.0),
            anchors.get(new_measurement.anchors.1),
        ) {
            commands.entity(e)
                .insert_bundle(PbrBundle{
                    mesh: assets.lane_mid_mesh.clone(),
                    material: assets.measurement_material.clone(),
                    transform: line_stroke_transform(start_anchor, end_anchor),
                    ..default()
                })
                .insert(Selectable::new(e))
                .remove::<Pending>();
        }
    }
}

fn update_measurement_visual(
    measurement: &Measurement<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transform: &mut Transform,
) {
    let start_anchor = anchors.get(measurement.anchors.0).unwrap();
    let end_anchor = anchors.get(measurement.anchors.1).unwrap();
    *transform = line_stroke_transform(start_anchor, end_anchor);
}

pub fn update_changed_measurement(
    mut measurements: Query<(&Measurement<Entity>, &mut Transform), Changed<Measurement<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
) {
    for (measurement, mut tf) in &mut measurements {
        update_measurement_visual(measurement, &anchors, tf.as_mut());
    }
}

pub fn update_measurement_for_changed_anchor(
    mut measurements: Query<(&Measurement<Entity>, &mut Transform)>,
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
