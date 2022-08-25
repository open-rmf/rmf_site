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
    interaction::*,
    animate::*,
    site::SiteAssets,
};
use bevy::prelude::*;
use bevy_mod_picking::PickingRaycastSet;
use bevy_mod_raycast::Intersection;

/// A resource that keeps track of the unique entities that play a role in
/// displaying the 3D cursor
#[derive(Debug, Clone)]
pub struct Cursor {
    pub frame: Entity,
    pub halo: Entity,
    pub dagger: Entity,
    pub anchor_placement: Entity,
}

impl FromWorld for Cursor {
    fn from_world(world: &mut World) -> Self {
        let interaction_assets = world.get_resource::<InteractionAssets>()
            .expect("make sure that the InteractionAssets resource is initialized before the Cursor resource");
        let site_assets = world.get_resource::<SiteAssets>()
            .expect("make sure that the SiteAssets resource is initialized before the Cursor resource");
        let halo = world.spawn()
            .insert_bundle(PbrBundle{
                transform: Transform::from_scale([0.2, 0.2, 1.].into()),
                mesh: interaction_assets.halo_mesh.clone(),
                visibility: Visibility { is_visible: false },
                ..default()
            })
            .id();

        let dagger = world.spawn()
            .insert_bundle(PbrBundle{
                mesh: interaction_assets.dagger_mesh.clone(),
                material: interaction_assets.dagger_material.clone(),
                visibility: Visibility { is_visible: false },
                ..default()
            })
            .insert(Spinning::default())
            .insert(Bobbing::default())
            .id();

        let anchor_placement = world.spawn()
            .insert_bundle(PbrBundle{
                transform: Transform{
                    rotation: Quat::from_rotation_x(90_f32.to_radians()),
                    ..default()
                },
                mesh: site_assets.anchor_mesh.clone(),
                material: materials.add(StandardMaterial{
                    base_color: Color::rgba(0.98, 0.91, 0.28, 0.5),
                    alpha_mode: AlphaMode::Blend,
                    depth_bias: 1.0,
                    ..default()
                }),
                visibility: Visibility { is_visible: false },
                ..default()
            })
            .id();

        let cursor = world.spawn()
            .push_children(&[selection_cursor, dagger_cursor, anchor_cursor])
            .insert_bundle(SpatialBundle::default());

        Self {
            frame: cursor,
            halo,
            dagger,
            anchor_placement,
        }
    }
}

pub fn update_cursor_transform(
    intersections: Query<&Intersection<PickingRaycastSet>>,
    mut transforms: Query<&mut Transform>,
    cursor: Res<Cursor>,
) {
    for intersection in &intersections {
        if let Some(mut transform) = transforms.get_mut(cursor.frame).ok() {
            if let Some(ray) = intersection.normal_ray() {
                *transform = Transform::from_matrix(ray.to_aligned_transform([0., 0., 1.].into()))
            }
        }
    }
}

pub fn hide_cursor(
    mut visibility: Query<&mut Visibility>,
    cursor: Res<Cursor>,
) {
    set_visibility(cursor.frame, &mut visibility, false);
}
