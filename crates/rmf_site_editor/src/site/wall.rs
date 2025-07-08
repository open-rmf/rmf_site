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
use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_mod_outline::GenerateOutlineNormalsError;
use rmf_site_format::{Edge, WallMarker, DEFAULT_LEVEL_HEIGHT};
use rmf_site_mesh::*;
use rmf_site_picking::Selectable;
use thiserror::Error;

pub const DEFAULT_WALL_THICKNESS: f32 = 0.1;

#[derive(Debug, Error)]
pub enum MeshCreationError {
    /// The given [`Entity`]'s components do not match the query.
    ///
    /// Either it does not have a requested component, or it has a component which the query filters out.
    #[error("Failed getting anchor transform: {0}")]
    GetAnchorTransformError(#[from] QueryEntityError),
    #[error("Error when generating normals: {0}")]
    GenerateOutlineNormalsError(#[from] GenerateOutlineNormalsError),
}

fn make_wall(
    entity: Entity,
    wall: &Edge<Entity>,
    texture: &Texture,
    anchors: &AnchorParams,
) -> Result<Mesh, MeshCreationError> {
    // TODO(luca) map texture rotation to UV coordinates
    let p_start = anchors.point_in_parent_frame_of(wall.start(), Category::Wall, entity)?;
    let p_end = anchors.point_in_parent_frame_of(wall.end(), Category::Wall, entity)?;
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
    .map_err(Into::into)
}

pub fn add_wall_visual(
    mut commands: Commands,
    walls: Query<(Entity, &Edge<Entity>, &Affiliation<Entity>), Added<WallMarker>>,
    anchors: AnchorParams,
    textures: Query<(Option<&TextureImage>, &Texture)>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, edge, texture_source) in &walls {
        let (base_color_texture, texture) = from_texture_source(texture_source, &textures);
        let (base_color, alpha_mode) = if let Some(alpha) = texture.alpha.filter(|a| a < &1.0) {
            (Color::default().with_alpha(alpha), AlphaMode::Blend)
        } else {
            (Color::default(), AlphaMode::Opaque)
        };
        let wall_mesh = match make_wall(e, edge, &texture, &anchors) {
            Ok(mesh) => mesh,
            Err(err) => {
                error!("Error while adding a wall: {err}");
                continue;
            }
        };

        commands
            .entity(e)
            .insert((
                Mesh3d(meshes.add(wall_mesh)),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color_texture,
                    base_color,
                    alpha_mode,
                    perceptual_roughness: 0.089,
                    metallic: 0.01,
                    ..default()
                })),
                Transform::default(),
                Visibility::default(),
            ))
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

pub fn update_walls_for_moved_anchors(
    walls: Query<(Entity, &Edge<Entity>, &Affiliation<Entity>, &Mesh3d), With<WallMarker>>,
    anchors: AnchorParams,
    textures: Query<(Option<&TextureImage>, &Texture)>,
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
            if let Some((e, edge, texture_source, mesh)) = walls.get(*dependent).ok() {
                let (_, texture) = from_texture_source(texture_source, &textures);
                let Some(mesh) = meshes.get_mut(&mesh.0) else {
                    continue;
                };
                *mesh = match make_wall(e, edge, &texture, &anchors) {
                    Ok(mesh) => mesh,
                    Err(err) => {
                        error!("Error while changing wall anchors: {err}");
                        continue;
                    }
                };
            }
        }
    }
}

pub fn update_walls(
    walls: Query<
        (
            &Edge<Entity>,
            &Affiliation<Entity>,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        ),
        With<WallMarker>,
    >,
    changed_walls: Query<
        Entity,
        (
            With<WallMarker>,
            Or<(Changed<Affiliation<Entity>>, Changed<Edge<Entity>>)>,
        ),
    >,
    changed_texture_sources: Query<
        &Members,
        (With<Group>, Or<(Changed<TextureImage>, Changed<Texture>)>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    anchors: AnchorParams,
    textures: Query<(Option<&TextureImage>, &Texture)>,
) {
    for e in changed_walls.iter().chain(
        changed_texture_sources
            .iter()
            .flat_map(|members| members.iter().cloned()),
    ) {
        let Ok((edge, texture_source, mesh, material)) = walls.get(e) else {
            continue;
        };
        let (base_color_texture, texture) = from_texture_source(texture_source, &textures);
        let Some(mesh) = meshes.get_mut(&mesh.0) else {
            continue;
        };
        *mesh = match make_wall(e, edge, &texture, &anchors) {
            Ok(mesh) => mesh,
            Err(err) => {
                error!("Error while creating wall mesh: {err}");
                continue;
            }
        };
        if let Some(material) = materials.get_mut(material) {
            let (base_color, alpha_mode) = if let Some(alpha) = texture.alpha.filter(|a| a < &1.0) {
                (Color::default().with_alpha(alpha), AlphaMode::Blend)
            } else {
                (Color::default(), AlphaMode::Opaque)
            };
            material.base_color_texture = base_color_texture;
            material.base_color = base_color;
            material.alpha_mode = alpha_mode;
        }
    }
}
