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

use crate::{interaction::Selectable, site::*, shapes::*};
use bevy::prelude::*;
use rmf_site_format::{Edge, WallMarker, DEFAULT_LEVEL_HEIGHT};

pub const DEFAULT_WALL_THICKNESS: f32 = 0.1;

fn make_wall(
    wall: &Edge<Entity>,
    anchors: &Query<(&Anchor, &GlobalTransform)>,
) -> Option<Mesh> {
    let p_start = Anchor::point_q(wall.start(), Category::Wall, anchors).ok()?;
    let p_end = Anchor::point_q(wall.end(), Category::Wall, anchors).ok()?;
    let (p_start, p_end) = if wall.start() == wall.end() {
        (
            p_start - DEFAULT_WALL_THICKNESS / 2.0 * Vec3::X,
            p_start + DEFAULT_WALL_THICKNESS / 2.0 * Vec3::X,
        )
    } else {
        (p_start, p_end)
    };

    Some(make_wall_mesh(p_start, p_end, DEFAULT_WALL_THICKNESS, DEFAULT_LEVEL_HEIGHT).into())
}

pub fn add_wall_visual(
    mut commands: Commands,
    walls: Query<(Entity, &Edge<Entity>), Added<WallMarker>>,
    anchors: Query<(&Anchor, &GlobalTransform)>,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge) in &walls {
        if let Some(mesh) = make_wall(edge, &anchors) {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(mesh),
                    // TODO(MXG): load the user-specified texture when one is given
                    material: assets.wall_material.clone(),
                    ..default()
                })
                .insert(Selectable::new(e))
                .insert(Category::Wall)
                .insert(EdgeLabels::StartEnd);
        } else {
            panic!("Anchor was not initialized correctly");
        }

        for anchor in &edge.array() {
            if let Ok(mut dep) = dependents.get_mut(*anchor) {
                dep.dependents.insert(e);
            }
        }
    }
}

fn update_wall_visuals(
    wall: &Edge<Entity>,
    anchors: &Query<(&Anchor, &GlobalTransform)>,
    mesh: &mut Handle<Mesh>,
    meshes: &mut Assets<Mesh>,
) {
    *mesh = meshes.add(make_wall(wall, anchors).unwrap());
}

pub fn update_wall_edge(
    mut walls: Query<
        (Entity, &Edge<Entity>, &mut Handle<Mesh>),
        (With<WallMarker>, Changed<Edge<Entity>>),
    >,
    anchors: Query<(&Anchor, &GlobalTransform)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge, mut mesh) in &mut walls {
        update_wall_visuals(edge, &anchors, mesh.as_mut(), meshes.as_mut());
    }
}

pub fn update_wall_for_moved_anchors(
    mut walls: Query<(&Edge<Entity>, &mut Handle<Mesh>), With<WallMarker>>,
    anchors: Query<(&Anchor, &GlobalTransform)>,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((wall, mut mesh)) = walls.get_mut(*dependent).ok() {
                update_wall_visuals(wall, &anchors, mesh.as_mut(), meshes.as_mut());
            }
        }
    }
}
