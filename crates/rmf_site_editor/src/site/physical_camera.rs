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
use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*};
use rmf_site_camera::{active_camera_maybe, ActiveCameraQuery};
use rmf_site_format::{PhysicalCameraProperties, Pose};
use rmf_site_picking::{DoubleClickSelection, Select, Selectable, Selection};
use std::time::Instant;

pub fn add_physical_camera_visuals(
    mut commands: Commands,
    physical_cameras: Query<(Entity, &Pose), Added<PhysicalCameraProperties>>,
    assets: Res<SiteAssets>,
) {
    for (e, pose) in &physical_cameras {
        commands
            .entity(e)
            .insert((
                Mesh3d(assets.physical_camera_mesh.clone()),
                MeshMaterial3d(assets.physical_camera_material.clone()),
                pose.transform(),
                Visibility::default(),
            ))
            .insert(Selectable::new(e))
            .insert(Category::Camera);
        // Now insert the camera as a child, needed to transform it
        let camera_sensor_transform = Pose {
            trans: [0., 0., 0.],
            rot: Rotation::EulerExtrinsicXYZ([
                Angle::Deg(90.0),
                Angle::Deg(0.0),
                Angle::Deg(-90.0),
            ]),
        };
        let child = commands
            .spawn(Camera3d::default())
            .insert((
                camera_sensor_transform.transform(),
                Camera {
                    is_active: false,
                    ..default()
                },
                Tonemapping::ReinhardLuminance,
            ))
            .id();
        commands.entity(e).add_children(&[child]);
    }
}

pub fn check_double_click_event(
    mut select: EventReader<Select>,
    selection: ResMut<Selection>,
    mut double_clicked: ResMut<DoubleClickSelection>,
) {
    let current_time = Instant::now();

    for _ in select.read() {
        if let (Some(last_entity), Some(last_time)) = (
            double_clicked.last_selected_entity,
            double_clicked.last_selected_time,
        ) {
            let elapsed_time = current_time.duration_since(last_time).as_millis();

            if let Some(selected_entity) = selection.0 {
                if last_entity == selected_entity && elapsed_time < 500 {
                    double_clicked.entity = selection.0;
                }
            }
        }
        double_clicked.last_selected_entity = selection.0;
        double_clicked.last_selected_time = Some(current_time);
    }
}

pub fn focus_camera_on_double_clicked_object(
    time: Res<Time>,
    active_camera: ActiveCameraQuery,
    mut transforms: Query<(&mut GlobalTransform, &mut Transform)>,
    lanes: Query<&LaneSegments>,
    walls: Query<&Edge<Entity>, With<WallMarker>>,
    floors: Query<&Path<Entity>, With<FloorMarker>>,
    measurements: Query<&Edge<Entity>, With<MeasurementMarker>>,
    mut double_clicking: ResMut<DoubleClickSelection>,
) {
    if double_clicking.entity.is_none() {
        return;
    }

    let mut target_position;
    if let Some(double_clicked) = double_clicking.entity {
        if let Ok((global_transform, _)) = transforms.get(double_clicked) {
            target_position = global_transform.translation();

            // if transform returns position of Vec3::ZERO, check if they belong to these groups
            if target_position.x == 0. && target_position.y == 0. {
                // check lane segments for entity
                if let Ok(marker) = lanes.get(double_clicked) {
                    let mid = marker.mid;
                    if let Ok((global_transform, _)) = transforms.get(mid) {
                        target_position = global_transform.translation();
                    }
                //check walls for entity
                } else if let Ok(edge) = walls.get(double_clicked) {
                    if let (Ok((start_anchor, _)), Ok((end_anchor, _))) =
                        (transforms.get(edge.start()), transforms.get(edge.end()))
                    {
                        let middle = (start_anchor.translation() + end_anchor.translation()) * 0.5;
                        target_position = middle;
                    }
                //check floors for entity
                } else if let Ok(floor) = floors.get(double_clicked) {
                    let anchors = &floor.0;

                    if anchors.iter().count() == 0 {
                        return;
                    }

                    let mut total_position = Vec3::ZERO;

                    for anchor in anchors.iter() {
                        if let Ok((global_transform, _)) = transforms.get(*anchor) {
                            total_position += global_transform.translation();
                        }
                    }
                    let average_anchor = total_position / anchors.iter().count() as f32;
                    target_position = average_anchor;
                //check measurements for entity
                } else if let Ok(edge) = measurements.get(double_clicked) {
                    if let (Ok((start_anchor, _)), Ok((end_anchor, _))) =
                        (transforms.get(edge.start()), transforms.get(edge.end()))
                    {
                        let middle = (start_anchor.translation() + end_anchor.translation()) * 0.5;
                        target_position = middle;
                    }
                }
            }
        } else {
            warn!(
                "Could not find transform for double clicked entity: {:?}",
                double_clicked
            );
            double_clicking.entity = None;
            return;
        }
    } else {
        return;
    }

    let Ok(active_camera_entity) = active_camera_maybe(&active_camera) else {
        return;
    };
    let Ok((_, mut camera_transform)) = transforms.get_mut(active_camera_entity) else {
        return;
    };

    let rotation_speed = 2.0;
    let camera_motion = camera_transform.looking_at(target_position, Vec3::Z);

    let current_direction: Vec3 = camera_transform.forward().into();
    let target_direction: Vec3 = camera_motion.forward().into();
    let rotation_difference = current_direction - target_direction;

    if rotation_difference.length() > 0.05 {
        camera_transform.rotation = camera_transform
            .rotation
            .slerp(camera_motion.rotation, rotation_speed * time.delta_secs());
    } else {
        double_clicking.entity = None;
    }
}
