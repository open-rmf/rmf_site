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

fn make_wall(
    entity: Entity,
    wall: &Edge<Entity>,
    texture: &Texture,
    anchors: &AnchorParams,
) -> Mesh {
    // TODO(luca) map texture rotation to UV coordinates
    let p_start = anchors
        .point_in_parent_frame_of(wall.start(), Category::Wall, entity)
        .expect("Failed getting anchor transform");
    let p_end = anchors
        .point_in_parent_frame_of(wall.end(), Category::Wall, entity)
        .expect("Failed getting anchor transform");
    let (p_start, p_end) = if wall.start() == wall.end() {
        (
            p_start - DEFAULT_WALL_THICKNESS / 2.0 * Vec3::X,
            p_start + DEFAULT_WALL_THICKNESS / 2.0 * Vec3::X,
        )
    } else {
        (p_start, p_end)
    };

    Mesh::from(make_wall_mesh(
        p_start,
        p_end,
        DEFAULT_WALL_THICKNESS,
        DEFAULT_LEVEL_HEIGHT,
        texture.height,
        texture.width,
    ))
    .with_generated_outline_normals()
    .unwrap()
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
        let (base_color, alpha_mode) = if let Some(alpha) = texture.alpha.filter(|a| a < &1.0) {
            (*Color::default().set_a(alpha), AlphaMode::Blend)
        } else {
            (Color::default(), AlphaMode::Opaque)
        };
        commands
            .entity(e)
            .insert(PbrBundle {
                mesh: meshes.add(make_wall(e, edge, texture, &anchors)),
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

pub fn update_wall_edge(
    mut walls: Query<
        (Entity, &Edge<Entity>, &Texture, &mut Handle<Mesh>),
        (With<WallMarker>, Changed<Edge<Entity>>),
    >,
    anchors: AnchorParams,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge, texture, mut mesh) in &mut walls {
        *mesh = meshes.add(make_wall(e, edge, texture, &anchors));
    }
}

pub fn update_wall_for_moved_anchors(
    mut walls: Query<(Entity, &Edge<Entity>, &Texture, &mut Handle<Mesh>), With<WallMarker>>,
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
            if let Some((e, edge, texture, mut mesh)) = walls.get_mut(*dependent).ok() {
                *mesh = meshes.add(make_wall(e, edge, texture, &anchors));
            }
        }
    }
}

pub fn update_wall_for_changed_texture(
    mut changed_walls: Query<
        (
            Entity,
            &Edge<Entity>,
            &Texture,
            &mut Handle<Mesh>,
            &mut Handle<StandardMaterial>,
        ),
        (Changed<Texture>, With<WallMarker>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    anchors: AnchorParams,
    asset_server: Res<AssetServer>,
) {
    for (e, edge, texture, mut mesh, mut material) in &mut changed_walls {
        *mesh = meshes.add(make_wall(e, edge, texture, &anchors));
        let (base_color, alpha_mode) = if let Some(alpha) = texture.alpha.filter(|a| a < &1.0) {
            (*Color::default().set_a(alpha), AlphaMode::Blend)
        } else {
            (Color::default(), AlphaMode::Opaque)
        };
        *material = materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load(&String::from(&texture.source))),
            base_color,
            alpha_mode,
            ..default()
        });
    }
}
