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

use crate::{shapes::*, site::*};
use bevy::{math::Affine3A, prelude::*};

#[derive(Resource)]
pub struct SiteAssets {
    pub default_floor_material: Handle<StandardMaterial>,
    pub lane_mid_mesh: Handle<Mesh>,
    pub lane_mid_outline: Handle<Mesh>,
    pub lane_end_mesh: Handle<Mesh>,
    pub lane_end_outline: Handle<Mesh>,
    pub box_mesh: Handle<Mesh>,
    pub location_mesh: Handle<Mesh>,
    pub physical_camera_mesh: Handle<Mesh>,
    pub unassigned_lane_material: Handle<StandardMaterial>,
    pub passive_anchor_material: Handle<StandardMaterial>,
    pub unassigned_anchor_material: Handle<StandardMaterial>,
    pub hover_anchor_material: Handle<StandardMaterial>,
    pub select_anchor_material: Handle<StandardMaterial>,
    pub hover_select_anchor_material: Handle<StandardMaterial>,
    pub preview_anchor_material: Handle<StandardMaterial>,
    pub hover_material: Handle<StandardMaterial>,
    pub select_material: Handle<StandardMaterial>,
    pub hover_select_material: Handle<StandardMaterial>,
    pub measurement_material: Handle<StandardMaterial>,
    pub level_anchor_mesh: Handle<Mesh>,
    pub lift_anchor_mesh: Handle<Mesh>,
    pub site_anchor_mesh: Handle<Mesh>,
    pub wall_material: Handle<StandardMaterial>,
    pub lift_wall_material: Handle<StandardMaterial>,
    pub door_body_material: Handle<StandardMaterial>,
    pub translucent_black: Handle<StandardMaterial>,
    pub translucent_white: Handle<StandardMaterial>,
    pub physical_camera_material: Handle<StandardMaterial>,
    pub occupied_material: Handle<StandardMaterial>,
}

impl FromWorld for SiteAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let wall_texture = asset_server.load(&String::from(&AssetSource::Bundled(
            "textures/default.png".to_string(),
        )));

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let unassigned_lane_material = materials.add(Color::rgb(0.1, 0.1, 0.1).into());
        let select_color = Color::rgb(1., 0.3, 1.);
        let hover_color = Color::rgb(0.3, 1., 1.);
        let hover_select_color = Color::rgb(1.0, 0.0, 0.3);
        let select_material = materials.add(select_color.into());
        let hover_material = materials.add(hover_color.into());
        let hover_select_material = materials.add(hover_select_color.into());
        // let hover_select_material = materials.add(Color::rgb_u8(177, 178, 255).into());
        // let hover_select_material = materials.add(Color::rgb_u8(214, 28, 78).into());
        let measurement_material = materials.add(Color::rgb_u8(250, 234, 72).into());
        let passive_anchor_material = materials.add(StandardMaterial {
            base_color: Color::rgb(0.4, 0.7, 0.6),
            // unlit: true,
            unlit: false,
            ..default()
        });
        let unassigned_anchor_material = materials.add(StandardMaterial {
            base_color: Color::rgb(1.0, 0.9, 0.05),
            // unlit: true,
            unlit: false,
            ..default()
        });
        let hover_anchor_material = materials.add(StandardMaterial {
            base_color: hover_color,
            // unlit: true,
            unlit: false,
            ..default()
        });
        let select_anchor_material = materials.add(StandardMaterial {
            base_color: select_color,
            // unlit: true,
            unlit: false,
            ..default()
        });
        let hover_select_anchor_material = materials.add(StandardMaterial {
            base_color: hover_select_color,
            // unlit: true,
            unlit: false,
            ..default()
        });
        let preview_anchor_material = materials.add(StandardMaterial {
            base_color: Color::rgba(0.98, 0.91, 0.28, 0.5),
            alpha_mode: AlphaMode::Blend,
            depth_bias: 1.0,
            // unlit: true,
            unlit: false,
            ..default()
        });
        let wall_material = materials.add(StandardMaterial {
            base_color_texture: Some(wall_texture),
            unlit: false,
            ..default()
        });
        let lift_wall_material = materials.add(StandardMaterial {
            base_color: Color::rgba(0.7, 0.7, 0.7, 1.0),
            perceptual_roughness: 0.3,
            ..default()
        });
        let default_floor_material = materials.add(StandardMaterial {
            base_color: Color::rgb(0.3, 0.3, 0.3).into(),
            perceptual_roughness: 0.5,
            ..default()
        });
        let door_body_material = materials.add(StandardMaterial {
            base_color: Color::rgba(1., 1., 1., 0.8),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let translucent_black = materials.add(StandardMaterial {
            base_color: Color::rgba(0., 0., 0., 0.8),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let translucent_white = materials.add(StandardMaterial {
            base_color: Color::rgba(1., 1., 1., 0.8),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let physical_camera_material = materials.add(Color::rgb(0.6, 0.7, 0.8).into());
        let occupied_material = materials.add(Color::rgba(0.8, 0.1, 0.1, 0.2).into());

        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let level_anchor_mesh = meshes.add(Mesh::from(shape::UVSphere {
            radius: 0.05, // TODO(MXG): Make the vertex radius configurable
            ..Default::default()
        }));
        let lift_anchor_mesh = meshes
            .add(Mesh::from(make_diamond(0.15 / 2.0, 0.15).transform_by(
                Affine3A::from_translation([0.0, 0.0, 0.15 / 2.0].into()),
            )));
        let site_anchor_mesh = meshes.add(Mesh::from(shape::UVSphere {
            radius: 0.05, // TODO(MXG): Make the vertex radius configurable
            ..Default::default()
        }));
        let lane_mid_mesh = meshes.add(make_flat_square_mesh(1.0).into());
        let lane_mid_outline = meshes.add(make_flat_rect_mesh(1.0, 1.125).into());
        let lane_end_mesh = meshes.add(
            make_flat_disk(
                Circle {
                    radius: LANE_WIDTH / 2.0,
                    height: 0.0,
                },
                32,
            )
            .into(),
        );
        let lane_end_outline = meshes.add(
            make_flat_disk(
                Circle {
                    radius: 1.125 * LANE_WIDTH / 2.0,
                    height: 0.0,
                },
                32,
            )
            .into(),
        );
        let box_mesh = meshes.add(shape::Box::new(1., 1., 1.).into());
        let location_mesh = meshes.add(
            make_icon_halo(1.1 * LANE_WIDTH / 2.0, 0.01, 6)
                .transform_by(Affine3A::from_translation(0.00125 * Vec3::Z))
                .into(),
        );
        let physical_camera_mesh = meshes.add(make_physical_camera_mesh());

        Self {
            level_anchor_mesh,
            lift_anchor_mesh,
            site_anchor_mesh,
            default_floor_material,
            lane_mid_mesh,
            lane_mid_outline,
            lane_end_mesh,
            lane_end_outline,
            box_mesh,
            location_mesh,
            physical_camera_mesh,
            unassigned_lane_material,
            hover_anchor_material,
            select_anchor_material,
            hover_select_anchor_material,
            hover_material,
            select_material,
            hover_select_material,
            measurement_material,
            passive_anchor_material,
            unassigned_anchor_material,
            preview_anchor_material,
            wall_material,
            lift_wall_material,
            door_body_material,
            translucent_black,
            translucent_white,
            physical_camera_material,
            occupied_material,
        }
    }
}

impl SiteAssets {
    pub fn decide_passive_anchor_material(
        &self,
        anchor: Entity,
        deps: &Query<&Dependents>,
    ) -> &Handle<StandardMaterial> {
        if deps.get(anchor).ok().filter(|d| !d.is_empty()).is_some() {
            &self.passive_anchor_material
        } else {
            &self.unassigned_anchor_material
        }
    }
}
