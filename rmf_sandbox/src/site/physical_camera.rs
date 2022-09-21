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
use rmf_site_format::{PhysicalCameraProperties, Pose};
use crate::{
    site::*,
    interaction::Selectable,
};

pub fn add_physical_camera_visuals(
    mut commands: Commands,
    physical_cameras: Query<(Entity, &Pose), Added<PhysicalCameraProperties>>,
    assets: Res<SiteAssets>,
) {
    for (e, pose) in &physical_cameras {
        commands.entity(e)
            .insert_bundle(PbrBundle{
                mesh: assets.physical_camera_mesh.clone(),
                material: assets.physical_camera_material.clone(),
                transform: pose.transform(),
                ..default()
            })
            .insert(Selectable::new(e))
            .insert(Category("Camera".to_string()));
    }
}
