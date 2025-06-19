use super::*;
use crate::*;

pub fn make_pyramid(circle: OffsetCircle, peak: [f32; 3], segments: u32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles([circle, circle], segments + 1, 0.)
        .chain([peak].into_iter().cycle().take(segments as usize))
        .collect();

    let peak_start = 2 * segments + 2;
    let complement_start = segments + 2;
    let indices = [[0, complement_start, peak_start]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
        .take(3 * segments as usize)
        .collect();

    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 0.]);
    for i in 0..segments {
        let v0 = (i + 0) as usize;
        let v1 = (i + complement_start) as usize;
        let vp = (i + peak_start) as usize;
        let p0: Vec3 = positions[v0].into();
        let p1: Vec3 = positions[v1].into();
        let p2: Vec3 = positions[vp].into();
        let n = if peak[2] < circle.height {
            (p2 - p0).cross(p1 - p0)
        } else {
            (p1 - p0).cross(p2 - p0)
        }
        .normalize();

        [v0, v1, vp].into_iter().for_each(|v| {
            normals[v] = n.into();
        });
    }

    return MeshBuffer::new(positions, normals, indices);
}