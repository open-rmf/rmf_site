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

use bevy::{
    prelude::*,
    render::mesh::shape::Box,
};
use rmf_site_format::Wall;
use crate::{
    site::*,
    interaction::Selectable,
};

pub const DEFAULT_WALL_THICKNESS: f32 = 0.1;

fn make_wall_components(
    wall: &Wall<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
) -> (Mesh, Transform) {
    let start_anchor = anchors.get(new_wall.anchors.0).unwrap();
    let end_anchor = anchors.get(new_wall.anchors.1).unwrap();

    let p_start = start_anchor.translation();
    let p_end = end_anchor.translation();
    let dp = p_end - p_start;
    let length = dp.length();
    let yaw = dp.y.atan2(dp.x);
    let center = (p_start + p_end)/2.0;

    let mesh = Box::new(length, DEFAULT_WALL_THICKNESS, DEFAULT_LEVEL_HEIGHT);
    let tf = Transform{
        translation: Vec3::new(center.x, center.y, DEFAULT_LEVEL_HEIGHT/2.0),
        rotation: Quat::from_rotation_z(yaw),
        ..default()
    };
    (mesh, tf)
}

fn add_wall_visual(
    mut commands: Commands,
    walls: Query<(Entity, &Wall<Entity>), Added<Wall<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, new_wall) in &walls {
        let (mesh, tf) = make_wall_components(new_wall, &anchors);
        commands.entity(e)
            .insert_bundle(PbrBundle{
                mesh: meshes.add(mesh),
                material: assets.wall_material.clone(), // TODO(MXG): load the user-specified texture when one is given
                transform: tf,
                ..default()
            })
            .insert(Selectable::new(e));
    }
}

fn update_wall_visuals(
    wall: &Wall<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transform: &mut Transform,
    mesh: &mut Handle<Mesh>,
    meshes: &mut Assets<Mesh>,
) {
    let (new_mesh, new_tf) = make_wall_components(wall, anchors);
    *mesh = meshes.add(new_mesh);
    *transform = new_tf;
}

fn update_changed_wall(
    mut walls: Query<(&Wall<Entity>, &mut Transform, &mut Handle<Mesh>), Changed<Wall<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (wall, mut tf, mut mesh) in &mut walls {
        update_wall_visuals(wall, &anchors, transform.as_mut(), mesh.as_mut(), meshes.as_mut());
    }
}

fn update_wall_for_changed_anchor(
    mut walls: Query<(&Wall<Entity>, &mut Transform, &mut Handle<Mesh>)>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut meshes: ReMut<Assets<Mesh>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((wall, mut tf, mut mesh)) = walls.get_mut(*dependent).ok() {
                update_wall_visuals(wall, &anchors, tf.as_mut(), mesh.as_mut(), meshes.as_mut());
            }
        }
    }
}
