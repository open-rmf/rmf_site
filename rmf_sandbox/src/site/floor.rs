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
    render::mesh::{
        PrimitiveTopology, Indices,
    }
};
use lyon::{
    math::point,
    path::{
        Path,
        builder::*,
    },
    tessellation::{
        *, geometry_builder::simple_builder,
    }
};
use rmf_site_format::Floor;
use crate::{
    site::*,
    interaction::Selectable,
};

fn make_floor_mesh(
    floor: &Floor<Entity>,
    anchors: &Query<&Anchor>,
) -> Mesh {
    let mut builder = Path::builder();
    let mut first = true;
    for anchor in &floor.anchors {
        let p = anchors.get(*anchor).unwrap().vec();
        if first {
            first = false;
            builder.begin(point(p.x, p.y));
        } else {
            builder.line_to(point(p.x, p.y));
        }
    }
    builder.close();
    let path = builder.build();

    let mut buffers: VertexBuffers<FillVertex, u32> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tessellator = FillTessellator::new();
        let result = tessellator.tessellate_path(
            path.iter(),
            &FillOptions::default(),
            &mut vertex_builder
        );

        match result {
            Err(err) => {
                print!("Failed to render floor: {err}\nFalling back to default floor plane.");
                return shape::Plane{size: 100.0}.into();
            },
            _ => { },
        }
    }

    let positions: Vec<[f32; 3]> = buffers.vertices.iter().map(
        |v| (v.position().x, v.position().y, 0.)
    ).collect();

    let normals: Vec<[f32; 3]> = buffers.vertices.iter().map(
        |_| (0., 0., 1.)
    ).collect();

    let uv: Vec<[f32; 2]> = buffers.vertices.iter().map(
        |v| (v.position().x, v.position().y)
    ).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(buffers.indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);

    mesh
}

fn add_floor_visuals(
    mut commands: Commands,
    floors: Query<(Entity, &Floor<Entity>), Added<Floor<Entity>>>,
    anchors: Query<&Anchor>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, new_floor) in &floors {
        let mesh = make_floor_mesh(new_floor, &anchors);
        commands.entity(e)
            .insert_bundle(PbrBundle{
                mesh: meshes.add(mesh),
                material: assets.default_floor_material.clone(), // TODO(MXG): load the user-specified texture when one is given
                ..default()
            })
            .insert(Selectable::new(e));
    }
}

fn update_changed_floor(
    floors: Query<(&mut Handle<Mesh>, &Floor<Entity>), Changed<Floor<Entity>>>,
    anchors: Query<&Anchor>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut mesh, floor) in &mut floors {
        *mesh = meshes.add(make_floor_mesh(floor, &anchors));
        // TODO(MXG): Update texture once we support textures
    }
}

fn update_floor_for_changed_anchor(
    floors: Query<(&mut Handle<Mesh>, &Floor<Entity>)>,
    anchors: Query<&Anchor>,
    changed_anchors: Query<&AnchorDependents, Changed<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((mut mesh, floor)) = floors.get(*dependent).ok() {
                *mesh = meshes.add(make_floor_mesh(floor, &anchors));
            }
        }
    }
}
