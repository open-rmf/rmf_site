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

use crate::{
    site::*,
    interaction::Selectable,
    shapes::*,
};
use rmf_site_format::{FloorMarker, Path};
use bevy::{
    prelude::*,
    render::mesh::{
        PrimitiveTopology, Indices,
    },
    math::Affine3A,
};
use lyon::{
    math::point,
    path::Path as LyonPath,
    tessellation::{
        *, geometry_builder::simple_builder,
    }
};

pub const FALLBACK_FLOOR_SIZE: f32 = 0.5;

fn make_fallback_floor_mesh(p: Vec3) -> Mesh {
    make_flat_square_mesh(0.5).transform_by(
        Affine3A::from_scale_rotation_translation(
            Vec3::splat(FALLBACK_FLOOR_SIZE),
            Quat::from_rotation_z(0.0),
            p,
        )
    ).into()
}

fn make_fallback_floor_mesh_at_avg(positions: Vec<Vec3>) -> Mesh {
    let p = positions.iter()
        .fold(Vec3::ZERO, |sum, x| sum + *x) / positions.len() as f32;
    return make_fallback_floor_mesh(p);
}

fn make_fallback_floor_mesh_near_path(
    path: &Path<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
) -> Mesh {
    let mut positions: Vec<Vec3> = Vec::new();
    for anchor in path.iter() {
        if let Ok(tf) = anchors.get(*anchor) {
            positions.push(tf.translation());
        }
    }
    return make_fallback_floor_mesh_at_avg(positions);
}

fn make_floor_mesh(
    anchor_path: &Path<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
) -> Mesh {
    if anchor_path.len() == 0 {
        return Mesh::new(PrimitiveTopology::TriangleList);
    } else if anchor_path.len() == 1 {
        let p = anchors.get(anchor_path[0])
            .map(|p| p.translation()).unwrap_or(Vec3::ZERO);
        return make_fallback_floor_mesh(p);
    } else if anchor_path.len() == 2 {
        let mut positions: Vec<&GlobalTransform> = Vec::new();
        let mut valid = true;
        for anchor in anchor_path.iter() {
            if let Ok(tf) = anchors.get(*anchor) {
                positions.push(tf);
            } else {
                println!("DEV ERROR: Failed to find anchor {anchor:?} used by a path");
                valid = false;
            }
        }
        if !valid {
            return make_fallback_floor_mesh_at_avg(
                positions.iter().map(|tf| tf.translation()).collect()
            );
        }

        let tf = line_stroke_transform(positions[0], positions[1], FALLBACK_FLOOR_SIZE);
        return make_flat_square_mesh(0.5).transform_by(tf.compute_affine()).into();
    }

    let mut builder = LyonPath::builder();
    let mut first = true;
    let mut valid = true;
    for anchor in &anchor_path.0 {
        let p = match anchors.get(*anchor) {
            Ok(a) => a,
            Err(_) => {
                println!("DEV ERROR: Failed to find anchor {anchor:?} used by a path");
                valid = false;
                continue;
            }
        }.translation();

        if first {
            first = false;
            builder.begin(point(p.x, p.y));
        } else {
            builder.line_to(point(p.x, p.y));
        }
    }

    if !valid {
        return make_fallback_floor_mesh_near_path(anchor_path, anchors);
    }

    builder.close();
    let path = builder.build();

    let mut buffers = VertexBuffers::new();
    {
        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tessellator = FillTessellator::new();
        let result = tessellator.tessellate_path(
            path.as_slice(),
            &FillOptions::default(),
            &mut vertex_builder,
        );

        match result {
            Err(err) => {
                println!("Failed to render floor: {err}");
                return make_fallback_floor_mesh_near_path(anchor_path, anchors);
            },
            _ => { },
        }
    }

    let positions: Vec<[f32; 3]> = buffers.vertices.iter().map(
        |v| [v.x, v.y, 0.]
    ).collect();

    let normals: Vec<[f32; 3]> = buffers.vertices.iter().map(
        |_| [0., 0., 1.]
    ).collect();

    let uv: Vec<[f32; 2]> = buffers.vertices.iter().map(
        |v| [v.x, v.y]
    ).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    for i in 0..buffers.indices.len()/3 {
        let i1 = 3*i+1;
        let i2 = 3*i+2;
        buffers.indices.swap(i1, i2);
    }
    let indices = buffers.indices.drain(..).map(|v| v as u32).collect();
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);

    mesh
}

pub fn add_floor_visuals(
    mut commands: Commands,
    floors: Query<(Entity, &Path<Entity>), Added<FloorMarker>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, new_floor) in &floors {
        let mesh = make_floor_mesh(new_floor, &anchors);
        commands.entity(e)
            .insert_bundle(PbrBundle{
                mesh: meshes.add(mesh),
                // TODO(MXG): load the user-specified texture when one is given
                material: assets.default_floor_material.clone(),
                ..default()
            })
            .insert(Selectable::new(e))
            .insert(Category("Floor".to_string()))
            .insert(PathBehavior::for_floor());

        for anchor in &new_floor.0 {
            let mut dep = dependents.get_mut(*anchor).unwrap();
            dep.dependents.insert(e);
        }
    }
}

pub fn update_changed_floor(
    mut floors: Query<(&mut Handle<Mesh>, &Path<Entity>), (Changed<Path<Entity>>, With<FloorMarker>)>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut mesh, floor) in &mut floors {
        *mesh = meshes.add(make_floor_mesh(floor, &anchors));
        // TODO(MXG): Update texture once we support textures
    }
}

pub fn update_floor_for_changed_anchor(
    mut floors: Query<(&mut Handle<Mesh>, &Path<Entity>), With<FloorMarker>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, Changed<GlobalTransform>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((mut mesh, path)) = floors.get_mut(*dependent).ok() {
                *mesh = meshes.add(make_floor_mesh(path, &anchors));
            }
        }
    }
}
