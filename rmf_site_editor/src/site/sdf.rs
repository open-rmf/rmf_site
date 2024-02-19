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
use bevy::render::mesh::shape::{Capsule, UVSphere};

use crate::interaction::Selectable;
use crate::shapes::make_cylinder;
use crate::site::SiteAssets;

use rmf_site_format::{ModelMarker, PrimitiveShape};

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
    parents: Query<&Parent>,
    selectables: Query<
        &Selectable,
        Or<(
            With<ModelMarker>,
            With<VisualMeshMarker>,
            With<CollisionMeshMarker>,
        )>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    site_assets: Res<SiteAssets>,
) {
    for (e, primitive) in primitives.iter() {
        let mesh = match primitive {
            PrimitiveShape::Box { size } => Mesh::from(shape::Box::new(size[0], size[1], size[2])),
            PrimitiveShape::Cylinder { radius, length } => {
                Mesh::from(make_cylinder(*length, *radius))
            }
            PrimitiveShape::Capsule { radius, length } => Mesh::from(Capsule {
                radius: *radius,
                depth: *length,
                ..default()
            }),
            PrimitiveShape::Sphere { radius } => Mesh::from(UVSphere {
                radius: *radius,
                ..default()
            }),
        };
        // If there is a parent with a Selectable component, use it to make this primitive
        // point to it. Otherwise set the Selectable to point to itself.
        let selectable = if let Some(selectable) = AncestorIter::new(&parents, e)
            .filter_map(|p| selectables.get(p).ok())
            .last()
        {
            selectable.clone()
        } else {
            Selectable::new(e)
        };
        commands
            .spawn(PbrBundle {
                mesh: meshes.add(mesh),
                material: site_assets.default_mesh_grey_material.clone(),
                ..default()
            })
            .insert(selectable)
            .set_parent(e);
    }
}
