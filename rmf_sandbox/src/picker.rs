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
        Mesh, PrimitiveTopology, Indices,
    }
};

#[derive(Debug)]
pub struct Cursor {
    select_cursor: Entity,
    vertex_cursor: Entity,
}

fn select_cursor_mesh() -> Mesh {
    // TODO(MXG): Consider moving this to an asset file instead of hard-coding it
    let inner_gap = 0.1;
    let line_width = 0.05;
    let height = 0.005;

    let in_min = -inner_gap/2.0;
    let in_max = inner_gap/2.0;
    let out_min = -inner_gap/2.0 - line_width;
    let out_max = inner_gap/2.0 + line_width;
    let leg_max = 1.5*inner_gap + line_width;
    let leg_min = -leg_max;

    // Creating a 3D # shape
    let positions = vec![
        // Top North
        [out_min, in_max, height],
        [out_max, in_max, height],
        [out_max, out_max, height],
        [out_min, out_max, height],
        // Top South
        [out_min, out_min, height],
        [out_max, out_min, height],
        [out_max, in_min, height],
        [out_min, in_min, height],
        // Top West
        [out_min, in_min, height],
        [in_min, in_min, height],
        [in_min, in_max, height],
        [out_min, in_max, height],
        // Top East
        [in_max, in_min, height],
        [out_max, in_min, height],
        [out_max, in_max, height],
        [in_max, in_max, height],
        // Top North by North West Leg
        [out_min, out_max, height],
        [in_min, out_max, height],
        [in_min, leg_max, height],
        [out_min, leg_max, height],
        // Top North by North East Leg
        [in_max, out_max, height],
        [out_max, out_max, height],
        [out_max, leg_max, height],
        [in_max, leg_max, height],
        // Top West by North West Leg
        [out_min, in_max, height],
        [out_min, out_max, height],
        [leg_min, out_max, height],
        [leg_min, in_max, height],
        // Top West by South West Leg
        [out_min, out_min, height],
        [out_min, in_min, height],
        [leg_min, in_min, height],
        [leg_min, out_min, height],
        // Top South by South West Leg
        [in_min, out_min, height],
        [out_min, out_min, height],
        [out_min, leg_min, height],
        [in_min, leg_min, height],
        // Top South by South East Leg
        [out_max, out_min, height],
        [in_max, out_min, height],
        [in_max, leg_min, height],
        [out_max, leg_min, height],
        // Top East by North East Leg
        [out_max, out_max, height],
        [out_max, in_max, height],
        [leg_max, in_max, height],
        [leg_max, out_max, height],
        // Top East by South East Leg
        [out_max, in_min, height],
        [out_max, out_min, height],
        [leg_max, out_min, height],
        [leg_max, in_min, height],
    ];

    let colors: Vec<[f32; 4]> = [
        [1., 1., 1., 1.],
    ].into_iter().cycle().take(4*4) // Take this color 4 times for each of the 4 cardinal directions
    .chain(
        [
            [1., 1., 1., 1.],
            [1., 1., 1., 1.],
            [1., 1., 1., 0.],
            [1., 1., 1., 0.],
        ].into_iter().cycle().take(4*8) // Take from this cycling color set 4 times for each of the 8 legs
    ).collect();

    let normals: Vec<[f32; 3]> = [
        [0., 0., 1.]
    ].into_iter().cycle().take(positions.len()).collect();

    let indices = Indices::U32([
        [0, 1, 2, 0, 2, 3]
    ].into_iter().cycle().enumerate()
    .flat_map(|(i, values)| {
        let offset = 4 * i as u32;
        values.map(|v| v + offset)
    })
    .take(24).collect());

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.set_indices(Some(indices));
    return mesh;
}

pub fn init_cursors(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {

}

