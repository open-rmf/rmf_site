use crate::*;

/// make a cuboid
///
/// renamed from `make_box` since box is a reserved word in rust and to mirror bevy's `box -> cuboid` rename.
pub fn make_cuboid(x_size: f32, y_size: f32, z_size: f32) -> MeshBuffer {
    let (min_x, max_x) = (-x_size / 2.0, x_size / 2.0);
    let (min_y, max_y) = (-y_size / 2.0, y_size / 2.0);
    let (min_z, max_z) = (-z_size / 2.0, z_size / 2.0);
    let vertices = &[
        // Top
        ([min_x, min_y, max_z], [0., 0., 1.]),
        ([max_x, min_y, max_z], [0., 0., 1.]),
        ([max_x, max_y, max_z], [0., 0., 1.]),
        ([min_x, max_y, max_z], [0., 0., 1.]),
        // Bottom
        ([min_x, max_y, min_z], [0., 0., -1.]),
        ([max_x, max_y, min_z], [0., 0., -1.]),
        ([max_x, min_y, min_z], [0., 0., -1.]),
        ([min_x, min_y, min_z], [0., 0., -1.]),
        // Right
        ([max_x, min_y, min_z], [1., 0., 0.]),
        ([max_x, max_y, min_z], [1., 0., 0.]),
        ([max_x, max_y, max_z], [1., 0., 0.]),
        ([max_x, min_y, max_z], [1., 0., 0.]),
        // Left
        ([min_x, min_y, max_z], [-1., 0., 0.]),
        ([min_x, max_y, max_z], [-1., 0., 0.]),
        ([min_x, max_y, min_z], [-1., 0., 0.]),
        ([min_x, min_y, min_z], [-1., 0., 0.]),
        // Front
        ([max_x, max_y, min_z], [0., 1., 0.]),
        ([min_x, max_y, min_z], [0., 1., 0.]),
        ([min_x, max_y, max_z], [0., 1., 0.]),
        ([max_x, max_y, max_z], [0., 1., 0.]),
        // Back
        ([max_x, min_y, max_z], [0., -1., 0.]),
        ([min_x, min_y, max_z], [0., -1., 0.]),
        ([min_x, min_y, min_z], [0., -1., 0.]),
        ([max_x, min_y, min_z], [0., -1., 0.]),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n)| *n).collect();
    let indices = vec![
        0, 1, 2, 2, 3, 0, // Top
        4, 5, 6, 6, 7, 4, // Bottom
        8, 9, 10, 10, 11, 8, // Right
        12, 13, 14, 14, 15, 12, // Left
        16, 17, 18, 18, 19, 16, // Front
        20, 21, 22, 22, 23, 20, // Back
    ];

    MeshBuffer::new(positions, normals, indices)
}

pub fn make_cuboidy_wrap(circles: [OffsetCircle; 2], segments: u32) -> MeshBuffer {
    let (bottom_circle, top_circle) = if circles[0].height < circles[1].height {
        (circles[0], circles[1])
    } else {
        (circles[1], circles[0])
    };

    let positions: Vec<[f32; 3]> = make_circles(
        [bottom_circle, bottom_circle, top_circle, top_circle],
        segments + 1,
        0.,
    )
    .collect();

    let indices = [[
        0,
        3 * segments + 4,
        2 * segments + 2,
        0,
        segments + 2,
        3 * segments + 4,
    ]]
    .into_iter()
    .cycle()
    .enumerate()
    .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
    .take(6 * segments as usize)
    .collect();

    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 0.]);
    for i in 0..segments {
        let v0 = (i + 0) as usize;
        let v1 = (i + 3 * segments + 4) as usize;
        let v2 = (i + 2 * segments + 2) as usize;
        let v3 = (i + segments + 2) as usize;
        let p0: Vec3 = positions[v0].into();
        let p1: Vec3 = positions[v1].into();
        let p2: Vec3 = positions[v2].into();
        let n = (p1 - p0).cross(p2 - p0).normalize();
        [v0, v1, v2, v3].into_iter().for_each(|v| {
            normals[v] = n.into();
        });
    }

    return MeshBuffer::new(positions, normals, indices);
}
