/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
use crate::{interaction::*, site::*};
use rmf_site_camera::*;


pub fn add_billboard_visual_cues(
    mut commands: Commands,
    new_mesh_billboards: Query<Entity, Added<BillboardMarker>>,
) {

    for e in &new_mesh_billboards {
        commands.entity(e).insert(VisualCue::no_outline());
        commands.entity(e).insert(Selectable::new(e));
    }
}

fn new_billboard_position(
    pivot_vec: Vec3,
    billboard_vec: Vec3,
    camera_vec: Vec3,
) -> Vec3 {
    let a = pivot_vec;
    let b = billboard_vec;
    let c = camera_vec;

    let radius = (b-a).length();
    if radius == 0.0 {
        return a;
    }

    let ab_vec = b - a;
    let c_norm = c.normalize();
    let proj = ab_vec - ab_vec.dot(c_norm) * c_norm;


    if proj.length_squared() < f32::EPSILON {
        let new_vec = if c_norm.x.abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let mut ad_dir = c_norm.cross(new_vec);

        if ad_dir.length_squared() < f32::EPSILON {
            let new_vec = if c_norm.y.abs() < 0.9 { Vec3::Y } else { Vec3::Z };
            ad_dir = c_norm.cross(new_vec);
        }
        ad_dir = ad_dir.normalize();
        let ad_vec = ad_dir * radius;
        return a + ad_vec;
    } else {
        let ad_dir = proj.normalize();
        let ad_vec = ad_dir * radius;
        a + ad_vec
    }
}

pub fn update_billboard_hover_mesh_location (
    query_mesh: Query<
        (&mut Transform, &BillboardMarker),
        With<BillboardMarker>>,
    query_cameras: Query<(&Projection, &GlobalTransform), Without<BillboardMarker>>,
    active_camera: ActiveCameraQuery,
) {
    let Ok(active_camera) = active_camera_maybe(&active_camera) else {
        return;
    };

    let Ok((_camera_projection, camera_transform)) = query_cameras.get(active_camera) else {
        warn!("No main camera found");
        return;
    };

    let camera_direction = camera_transform.forward().into();

    for (mut transform, marker) in query_mesh {
        let a = marker.pivot;
        let b = marker.offset;
        let new_vec = new_billboard_position(a, b, camera_direction);
        transform.translation = new_vec - a;
    }


}

pub fn update_billboard_text_visibility_on_hover(
    query_billboards: Query<
        (&Hovered, &BillboardMarker),
        (
            With<BillboardMarker>,
            Changed<Hovered>,
        ),
    >,
    mut query_text_children: Query<&mut Visibility, With<BillboardTextMarker>>,

) {
    for (hovered, marker) in &query_billboards {
        if hovered.cue() {
            if let Ok(mut caption_visibility) = query_text_children.get_mut(marker.caption_entity) {
                *caption_visibility = Visibility::Visible;
            }
        } else {
            if let Ok(mut caption_visibility) = query_text_children.get_mut(marker.caption_entity) {
                *caption_visibility = Visibility::Hidden;
            }
        }

    }
}