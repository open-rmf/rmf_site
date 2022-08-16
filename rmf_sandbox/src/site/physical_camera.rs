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
use rmf_site_format::PhysicalCamera;
use crate::{
    site::*,
    shapes::*,
    interaction::Selectable,
};

pub fn add_physical_camera_visuals(
    mut commands: Commands,
    physical_cameras: Query<(Entity, &PhysicalCamera), Added<PhysicalCamera>>,
    assets: Res<SiteAssets>,
) {
    for (e, new_physical_camera) in &physical_cameras {
        commands.entity(e)
            .insert_bundle(PbrBundle{
                mesh: assets.physical_camera_mesh.clone(),
                material: assets.physical_camera_material.clone(),
                transform: new_physical_camera.pose.transform(),
                ..default()
            })
            .insert(Selectable::new(e));
    }
}

pub fn update_changed_physical_camera_visuals(
    physical_cameras: Query<(&PhysicalCamera, &mut Transform), Changed<PhysicalCamera>>,
) {
    for (physical_camera, mut tf) in &mut physical_cameras {
        *tf = physical_camera.pose.transform();
    }
}
