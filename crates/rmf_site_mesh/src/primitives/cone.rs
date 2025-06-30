use bevy_math::Affine3A;

use super::*;
use crate::*;

pub fn make_cone(circle: OffsetCircle, peak: [f32; 3], resolution: u32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles([circle], resolution + 1, 0.)
        .take(resolution as usize) // skip the last vertex which would close the circle
        .chain([peak].into_iter().cycle().take(resolution as usize))
        .collect();

    let peak_start = resolution;
    let indices: Vec<u32> = [[0, 1, peak_start]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
        .take(3 * (resolution as usize - 1))
        .chain([peak_start - 1, 0, (positions.len() - 1) as u32])
        .collect();

    let mut normals = Vec::<[f32; 3]>::new();
    let base_p = Vec3::new(peak[0], peak[1], circle.height);
    normals.resize(positions.len(), [0., 0., 1.]);
    for i in 0..resolution {
        // Normals around the ring
        let calculate_normal = |theta: f32| -> [f32; 3] {
            let p = circle.radius * Vec3::new(theta.cos(), theta.sin(), circle.height);
            let r = (p - base_p).length();
            let h = peak[2] - circle.height;
            let phi = r.atan2(h);
            let r_y = Affine3A::from_rotation_y(-phi);
            let r_z = Affine3A::from_rotation_z(theta);
            (r_z * r_y).transform_vector3(Vec3::new(1., 0., 0.)).into()
        };

        let theta = (i as f32) / (resolution as f32) * 2.0 * std::f32::consts::PI;
        normals[i as usize] = calculate_normal(theta);

        let mid_theta = (i as f32 + 0.5) / (resolution as f32) * 2.0 * std::f32::consts::PI;
        normals[(i + peak_start) as usize] = calculate_normal(mid_theta);
    }

    return MeshBuffer::new(positions, normals, indices);
}
