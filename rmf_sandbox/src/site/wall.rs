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
) -> Option<(Mesh, Transform)> {
    if let (Ok(start_anchor), Ok(end_anchor)) = (
        anchors.get(wall.anchors.0),
        anchors.get(wall.anchors.1),
    ) {
        let p_start = start_anchor.translation();
        let p_end = end_anchor.translation();
        let dp = p_end - p_start;
        let length = dp.length();
        let yaw = dp.y.atan2(dp.x);
        let center = (p_start + p_end)/2.0;

        let mut mesh: Mesh = Box::new(length, DEFAULT_WALL_THICKNESS, DEFAULT_LEVEL_HEIGHT).into();
        // The default UV coordinates made by bevy do not work well for walls,
        // so we customize them here
        let uv = vec![
            // Top
            [0., 0.], // 0
            [0., 0.], // 1
            [0., 0.], // 2
            [0., 0.], // 3
            // Bottom
            [0., 1.], // 4
            [0., 1.], // 5
            [0., 1.], // 6
            [0., 1.], // 7
            // right
            [length, 1.], // 8
            [0., 1.], // 9
            [0., 0.], // 10
            [length, 0.], // 11
            // left
            [0., 0.], // 12
            [length, 0.], // 13
            [length, 1.], // 14
            [0., 1.], // 15
            // front
            [0., 1.], // 16
            [length, 1.], // 17
            [length, 0.], // 18
            [0., 0.], // 19
            // back
            [length, 0.], // 20
            [0., 0.], // 21
            [0., 1.], // 22
            [length, 1.], // 23
        ];
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);

        let tf = Transform{
            translation: Vec3::new(center.x, center.y, DEFAULT_LEVEL_HEIGHT/2.0),
            rotation: Quat::from_rotation_z(yaw),
            ..default()
        };
        return Some((mesh.into(), tf));
    }

    None
}

pub fn add_wall_visual(
    mut commands: Commands,
    walls: Query<(Entity, &Wall<Entity>), Added<Wall<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, new_wall) in &walls {
        if let Some((mesh, tf)) = make_wall_components(new_wall, &anchors) {
            commands.entity(e)
                .insert_bundle(PbrBundle{
                    mesh: meshes.add(mesh),
                    material: assets.wall_material.clone(), // TODO(MXG): load the user-specified texture when one is given
                    transform: tf,
                    ..default()
                })
                .insert(Selectable::new(e));
        } else {
            panic!("Anchor was not initialized correctly");
        }

        for mut dep in dependents.get_many_mut(
            [new_wall.anchors.0, new_wall.anchors.1]
        ).unwrap() {
            dep.dependents.insert(e);
        }
    }
}

fn update_wall_visuals(
    wall: &Wall<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transform: &mut Transform,
    mesh: &mut Handle<Mesh>,
    meshes: &mut Assets<Mesh>,
) {
    let (new_mesh, new_tf) = make_wall_components(wall, anchors).unwrap();
    *mesh = meshes.add(new_mesh);
    *transform = new_tf;
}

pub fn update_changed_wall(
    mut walls: Query<(&Wall<Entity>, &mut Transform, &mut Handle<Mesh>), Changed<Wall<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (wall, mut tf, mut mesh) in &mut walls {
        update_wall_visuals(wall, &anchors, tf.as_mut(), mesh.as_mut(), meshes.as_mut());
    }
}

pub fn update_wall_for_changed_anchor(
    mut walls: Query<(&Wall<Entity>, &mut Transform, &mut Handle<Mesh>)>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((wall, mut tf, mut mesh)) = walls.get_mut(*dependent).ok() {
                update_wall_visuals(wall, &anchors, tf.as_mut(), mesh.as_mut(), meshes.as_mut());
            }
        }
    }
}
