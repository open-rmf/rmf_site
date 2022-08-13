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

use bevy::prelude::*;
use crate::shapes::*;

pub struct SiteAssets {
    pub default_floor_material: Handle<StandardMaterial>,
    pub lane_mid_mesh: Handle<Mesh>,
    pub lane_end_mesh: Handle<Mesh>,
    pub box_mesh: Handle<Mesh>,
    pub physical_camera_mesh: Handle<Mesh>,
    pub passive_lane_material: Handle<StandardMaterial>,
    pub passive_anchor_material: Handle<StandardMaterial>,
    pub hover_material: Handle<StandardMaterial>,
    pub select_material: Handle<StandardMaterial>,
    pub hover_select_material: Handle<StandardMaterial>,
    pub measurement_material: Handle<StandardMaterial>,
    pub anchor_mesh: Handle<Mesh>,
    pub wall_material: Handle<StandardMaterial>,
    pub door_material: Handle<StandardMaterial>,
    pub physical_camera_material: Handle<StandardMaterial>,
}

impl FromWorld for SiteAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let wall_texture = asset_server.load("sandbox://textures/default.png");

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let passive_lane_material = materials.add(Color::rgb(1.0, 0.5, 0.3).into());
        let select_material = materials.add(Color::rgb(1., 0.3, 1.).into());
        let hover_material = materials.add(Color::rgb(0.3, 1., 1.).into());
        let hover_select_material = materials.add(Color::rgb(1., 0.6, 1.).into());
        let measurement_material = materials.add(Color::rgb_u8(250, 234, 72).into());
        let passive_vertex_material = materials.add(Color::rgb(0.4, 0.7, 0.6).into());
        let wall_material = materials.add(StandardMaterial {
            base_color_texture: Some(wall_texture),
            unlit: false,
            ..default()
        });
        let default_floor_material = materials.add(StandardMaterial {
            base_color: Color::rgb(0.3, 0.3, 0.3).into(),
            perceptual_roughness: 0.5,
            ..default()
        });
        let door_material = materials.add(StandardMaterial {
            base_color: Color::rgba(1., 1., 1., 0.8),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let physical_camera_material = materials.add(Color::rgb(0.6, 0.7, 0.8).into());

        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let vertex_mesh = meshes.add(Mesh::from(shape::Capsule {
            radius: 0.15, // TODO(MXG): Make the vertex radius configurable
            rings: 2,
            depth: 0.05,
            latitudes: 8,
            longitudes: 16,
            uv_profile: shape::CapsuleUvProfile::Fixed,
        }));
        let lane_mid_mesh = meshes.add(shape::Quad::new(Vec2::from([1., 1.])).into());
        let lane_end_mesh = meshes.add(shape::Circle::new(LANE_WIDTH / 2.).into());
        let box_mesh = meshes.add(shape::Box::new(1., 1., 1.));
        let physical_camera_mesh = meshes.add(make_physical_camera_mesh());

        Self {
            anchor_mesh: vertex_mesh,
            default_floor_material,
            lane_mid_mesh,
            lane_end_mesh,
            box_mesh,
            physical_camera_mesh,
            passive_lane_material,
            hover_material,
            select_material,
            hover_select_material,
            measurement_material,
            passive_anchor_material: passive_vertex_material,
            wall_material,
            door_material,
            physical_camera_material,
        }
    }
}
