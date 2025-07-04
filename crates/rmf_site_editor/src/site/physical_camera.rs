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
use rmf_site_format::{PhysicalCameraProperties, Pose};
use rmf_site_picking::Selectable;

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
