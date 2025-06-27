use bevy_asset::RenderAssetUsages;
use bevy_math::Affine3A;
use bevy_render::mesh::{Indices, PrimitiveTopology};

use crate::make_circles;

use super::*;
use crate::*;

pub fn make_halo_mesh() -> Mesh {
    let inner_ring = 1.0;
    let mid_ring = 1.1 * inner_ring;
    let outer_ring = 1.2 * inner_ring;
    let peak = 0.01;
    let segments = 100u32;
    let gap = 60_f32.to_radians();

    let positions: Vec<[f32; 3]> = make_circles(
        [
            (inner_ring, 0.).into(),
            (mid_ring, peak).into(),
            (outer_ring, 0.).into(),
        ],
        segments,
        gap,
    )
    .collect();

    let colors: Vec<[f32; 4]> = [[1., 1., 1., 1.]]
        .into_iter()
        .cycle()
        .take(2 * segments as usize)
        .chain(
            [[1., 1., 1., 0.]]
                .into_iter()
                .cycle()
                .take(segments as usize),
        )
        .collect();

    let normals: Vec<[f32; 3]> = [[0., 0., 1.]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    let indices = Indices::U32(
        [[0u32, segments, segments + 1u32, 0u32, segments + 1u32, 1u32]]
            .into_iter()
            .cycle()
            .enumerate()
            .flat_map(|(cycle, values)| {
                [(cycle as u32, values)]
                    .into_iter()
                    .cycle()
                    .enumerate()
                    .take(segments as usize - 1)
                    .flat_map(|(segment, (cycle, values))| {
                        values.map(|s| cycle * segments + segment as u32 + s)
                    })
            })
            .take(6 * 2 * (segments as usize - 1))
            .chain([
                0,
                2 * segments,
                segments,
                3 * segments - 1,
                segments - 1,
                2 * segments - 1,
            ])
            .collect(),
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(indices);
    return mesh;
}

pub fn make_icon_halo(radius: f32, height: f32, segments: usize) -> MeshBuffer {
    let angle = (360.0 / (2.0 * segments as f32)).to_radians();
    let p0 = radius * Vec3::X;
    let p1 = Affine3A::from_rotation_z(angle).transform_vector3(p0);
    let width = (p1 - p0).length();
    let mut mesh = make_ring(radius, radius + width / 2.0, 32);
    for i in 0..segments {
        mesh = mesh.merge_with(
            make_cuboid(width, width, height)
                .transform_by(Affine3A::from_translation(Vec3::new(
                    radius + width / 2.0,
                    0.0,
                    height / 2.0,
                )))
                .transform_by(Affine3A::from_rotation_z(i as f32 * 2.0 * angle)),
        );
    }

    mesh
}
