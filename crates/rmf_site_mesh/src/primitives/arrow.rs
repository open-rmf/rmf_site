use bevy_asset::RenderAssetUsages;
use bevy_math::Affine3A;
use bevy_render::mesh::PrimitiveTopology;

use super::*;
use crate::*;

pub fn make_dagger_mesh() -> Mesh {
    let lower_ring = OffsetCircle {
        radius: 0.01,
        height: 0.1,
    };
    let upper_ring = OffsetCircle {
        radius: 0.02,
        height: 0.4,
    };
    let top_height = 0.42;
    let segments = 4u32;

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    make_cuboidy_wrap([lower_ring, upper_ring], segments).merge_into(&mut mesh);
    make_pyramid(upper_ring, [0., 0., top_height], segments).merge_into(&mut mesh);
    make_pyramid(lower_ring.flip_height(), [0., 0., 0.], segments)
        .transform_by(Affine3A::from_quat(Quat::from_rotation_y(
            180_f32.to_radians(),
        )))
        .merge_into(&mut mesh);
    return mesh;
}

pub fn flat_arrow_mesh(
    handle_length: f32,
    handle_width: f32,
    tip_length: f32,
    tip_width: f32,
) -> MeshBuffer {
    let half_handle_width = handle_width / 2.0;
    let half_tip_width = tip_width / 2.0;
    let positions: Vec<[f32; 3]> = vec![
        [0.0, half_handle_width, 0.0],            // 0
        [0.0, -half_handle_width, 0.0],           // 1
        [handle_length, -half_handle_width, 0.0], // 2
        [handle_length, half_handle_width, 0.0],  // 3
        [handle_length, half_tip_width, 0.0],     // 4
        [handle_length, -half_tip_width, 0.0],    // 5
        [handle_length + tip_length, 0.0, 0.0],   // 6
    ];

    let normals: Vec<[f32; 3]> = {
        let mut normals = Vec::new();
        normals.resize(positions.len(), [0.0, 0.0, 1.0]);
        normals
    };

    let indices: Vec<u32> = vec![0, 1, 3, 1, 2, 3, 4, 5, 6];

    let outline: Vec<u32> = vec![0, 1, 1, 2, 2, 5, 5, 6, 6, 4, 4, 3, 3, 0];

    MeshBuffer::new(positions, normals, indices).with_outline(outline)
}

pub fn make_cylinder_arrow_mesh() -> Mesh {
    let tip = [0., 0., 1.0];
    let l_head = 0.2;
    let r_head = 0.15;
    let r_base = 0.1;
    let head_base = OffsetCircle {
        radius: r_head,
        height: 1.0 - l_head,
    };
    let cylinder_top = OffsetCircle {
        radius: r_base,
        height: 1.0 - l_head,
    };
    let cylinder_bottom = OffsetCircle {
        radius: r_base,
        height: 0.0,
    };
    let resolution = 32u32;

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    make_cone(head_base, tip, resolution).merge_into(&mut mesh);
    make_smooth_wrap([cylinder_top, cylinder_bottom], resolution).merge_into(&mut mesh);
    make_smooth_wrap([head_base, cylinder_top], resolution).merge_into(&mut mesh);
    make_bottom_circle(cylinder_bottom, resolution).merge_into(&mut mesh);
    return mesh;
}

pub fn flat_arrow_mesh_between(
    start: Vec3,
    stop: Vec3,
    handle_width: f32,
    tip_length: f32,
    tip_width: f32,
) -> MeshBuffer {
    let total_length = (stop - start).length();
    let tip_length = total_length.min(tip_length);
    let handle_length = total_length - tip_length;
    let dp = stop - start;
    let yaw = dp.y.atan2(dp.x);

    flat_arrow_mesh(handle_length, handle_width, tip_length, tip_width).transform_by(
        Affine3A::from_scale_rotation_translation(
            Vec3::new(1.0, 1.0, 1.0),
            Quat::from_rotation_z(yaw),
            start,
        ),
    )
}

pub fn make_triangular_arrow() -> MeshBuffer {
    let base_positions = vec![
        [0.0, 0.0, 1.0],  // 0
        [0.5, 0.0, 0.0],  // 1
        [-0.5, 0.0, 0.0], // 2
        [0.0, 0.0, 0.2],  // 3
        [0.0, 0.3, 0.4],  // 4
    ];

    let base_indices: Vec<u32> = vec![0, 1, 4, 0, 4, 2, 3, 4, 1, 3, 2, 4, 0, 3, 1, 0, 2, 3];

    let indices: Vec<u32> = (0..18).collect();

    let positions: Vec<[f32; 3]> = base_indices
        .clone()
        .into_iter()
        .map(|idx| base_positions[idx as usize])
        .collect();

    let normals: Vec<[f32; 3]> = (0..base_indices.len())
        .step_by(3)
        .into_iter()
        .map(|idx| {
            let chunk = [
                base_indices[idx as usize],
                base_indices[(idx + 1) as usize],
                base_indices[(idx + 2) as usize],
            ];
            let p0: Vec3 = base_positions[chunk[0] as usize].into();
            let p1: Vec3 = base_positions[chunk[1] as usize].into();
            let p2: Vec3 = base_positions[chunk[2] as usize].into();
            let n = (p1 - p0).cross(p2 - p0).normalize();

            n.into()
        })
        .flat_map(|normal| vec![normal, normal, normal])
        .collect();

    MeshBuffer::new(positions, normals, indices)
}

pub fn make_triangular_arrow_mesh() -> Mesh {
    make_triangular_arrow().into()
}
