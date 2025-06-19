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

use bevy::ecs::{hierarchy::ChildOf, relationship::AncestorIter, system::EntityCommands};
use bevy::math::primitives;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use rmf_site_camera::MODEL_PREVIEW_LAYER;
use rmf_site_mesh::*;

use crate::interaction::{DragPlaneBundle, Selectable};
use crate::site::SiteAssets;

use rmf_site_format::PrimitiveShape;

/// An empty component to mark this entity as a visual mesh
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct VisualMeshMarker;

/// An empty component to mark this entity as a collision mesh
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct CollisionMeshMarker;

pub fn handle_new_primitive_shapes(
    mut commands: Commands,
    primitives: Query<(Entity, &PrimitiveShape), Added<PrimitiveShape>>,
    child_of: Query<&ChildOf>,
    selectables: Query<&Selectable>,
    render_layers: Query<&RenderLayers>,
    mut meshes: ResMut<Assets<Mesh>>,
    site_assets: Res<SiteAssets>,
) {
    for (e, primitive) in primitives.iter() {
        let mesh = match primitive {
            PrimitiveShape::Box { size } => Mesh::from(Cuboid::new(size[0], size[1], size[2])),
            PrimitiveShape::Cylinder { radius, length } => {
                Mesh::from(make_cylinder(*length, *radius))
            }
            PrimitiveShape::Capsule { radius, length } => {
                Mesh::from(primitives::Capsule3d::new(*radius, *length))
            }
            PrimitiveShape::Sphere { radius } => Mesh::from(primitives::Sphere::new(*radius)),
        };
        // If there is a parent with a Selectable component, use it to make this primitive
        // point to it. Otherwise set the Selectable to point to itself.
        let id = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(site_assets.default_mesh_grey_material.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .insert(ChildOf(e))
            .id();

        let spawn_selectable = |mut cmd: EntityCommands| {
            let selectable = if let Some(selectable) = AncestorIter::new(&child_of, e)
                .filter_map(|p| selectables.get(p).ok())
                .last()
            {
                selectable.element
            } else {
                e
            };
            cmd.insert(DragPlaneBundle::new(selectable, Vec3::Z));
        };
        let mut entity_commands = commands.entity(id);
        if let Some(render_layer) = AncestorIter::new(&child_of, e)
            .filter_map(|p| render_layers.get(p).ok())
            .last()
        {
            entity_commands.insert(render_layer.clone());
            if !render_layer.iter().all(|l| l == MODEL_PREVIEW_LAYER) {
                spawn_selectable(entity_commands);
            }
        } else {
            spawn_selectable(entity_commands);
        }
    }
}
