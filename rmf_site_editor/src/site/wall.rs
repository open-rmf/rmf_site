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

use crate::{interaction::Selectable, shapes::*, site::*};
use bevy::prelude::*;
use rmf_site_format::{Edge, WallMarker, DEFAULT_LEVEL_HEIGHT};

pub const DEFAULT_WALL_THICKNESS: f32 = 0.1;

fn make_wall(entity: Entity, wall: &Edge<Entity>, anchors: &AnchorParams) -> Option<Mesh> {
    let p_start = anchors
        .point_in_parent_frame_of(wall.start(), Category::Wall, entity)
        .ok()?;
    let p_end = anchors
        .point_in_parent_frame_of(wall.end(), Category::Wall, entity)
        .ok()?;
    let (p_start, p_end) = if wall.start() == wall.end() {
        (
            p_start - DEFAULT_WALL_THICKNESS / 2.0 * Vec3::X,
            p_start + DEFAULT_WALL_THICKNESS / 2.0 * Vec3::X,
        )
    } else {
        (p_start, p_end)
    };

    Some(
        Mesh::from(make_wall_mesh(
            p_start,
            p_end,
            DEFAULT_WALL_THICKNESS,
            DEFAULT_LEVEL_HEIGHT,
        ))
        .with_generated_outline_normals()
        .unwrap(),
    )
}

pub fn add_wall_visual(
    mut commands: Commands,
    walls: Query<(Entity, &Edge<Entity>, &Texture), Added<WallMarker>>,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for (e, edge, texture) in &walls {
        // TODO(luca) map texture parameters such as scale, offset, rotation, to UV coordinates
        let mesh = make_wall(e, edge, &anchors).expect("Anchor was not initialized correctly");
        let (base_color, alpha_mode) = if let Some(alpha) = texture.alpha.filter(|a| a < &1.0) {
            (*Color::default().set_a(alpha), AlphaMode::Blend)
        } else {
            (Color::default(), AlphaMode::Opaque)
        };
        commands
            .entity(e)
            .insert(PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load(&String::from(&texture.source))),
                    base_color,
                    alpha_mode,
                    ..default()
                }),
                ..default()
            })
            .insert(Selectable::new(e))
            .insert(Category::Wall)
            .insert(EdgeLabels::StartEnd);

        for anchor in &edge.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }
    }
}

fn update_wall_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    anchors: &AnchorParams,
    mesh: &mut Handle<Mesh>,
    meshes: &mut Assets<Mesh>,
) {
    *mesh = meshes.add(make_wall(entity, edge, anchors).unwrap());
}

pub fn update_wall_edge(
    mut walls: Query<
        (Entity, &Edge<Entity>, &mut Handle<Mesh>),
        (With<WallMarker>, Changed<Edge<Entity>>),
    >,
    anchors: AnchorParams,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge, mut mesh) in &mut walls {
        update_wall_visuals(e, edge, &anchors, mesh.as_mut(), meshes.as_mut());
    }
}

pub fn update_wall_for_moved_anchors(
    mut walls: Query<(Entity, &Edge<Entity>, &mut Handle<Mesh>), With<WallMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Some((e, wall, mut mesh)) = walls.get_mut(*dependent).ok() {
                update_wall_visuals(e, wall, &anchors, mesh.as_mut(), meshes.as_mut());
            }
        }
    }
}
