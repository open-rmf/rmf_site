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

use crate::{layers::ZLayer, site::*};
use bevy::{
    asset::embedded_asset,
    math::{primitives, Affine3A},
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};
use bevy_rich_text3d::TextAtlas;
use rmf_site_mesh::*;

const LANE_SHADER_PATH: &str = "embedded://librmf_site_editor/site/shaders/lane_arrow_shader.wgsl";

pub const SELECT_COLOR: Color = Color::srgb(1., 0.3, 1.);
pub const HOVER_COLOR: Color = Color::srgb(0.3, 1., 1.);
pub const HOVER_SELECT_COLOR: Color = Color::srgb(1.0, 0.0, 0.3);
pub const NAV_UNASSIGNED_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);

pub(crate) fn add_site_assets(app: &mut App) {
    embedded_asset!(app, "src/", "textures/base.png");
    embedded_asset!(app, "src/", "textures/charging.png");
    embedded_asset!(app, "src/", "textures/parking.png");
    embedded_asset!(app, "src/", "textures/holding.png");
    embedded_asset!(app, "src/", "textures/empty.png");
    embedded_asset!(app, "src/", "textures/door_cue.png");
    embedded_asset!(app, "src/", "textures/door_cue_highlighted.png");
    embedded_asset!(app, "src/", "shaders/lane_arrow_shader.wgsl");
}

#[derive(Default, Debug, Reflect, ShaderType, Clone, Copy, Deref, DerefMut)]
pub struct BigF32 {
    #[size(16)]
    #[align(16)]
    pub value: f32,
}

impl BigF32 {
    pub fn new(value: f32) -> Self {
        Self { value }
    }
}

#[derive(Default, Debug, Reflect, ShaderType, Clone, Copy, Deref, DerefMut)]
pub struct BigU32 {
    #[size(16)]
    #[align(16)]
    pub value: u32,
}

impl BigU32 {
    pub fn new(value: u32) -> Self {
        Self { value }
    }
}

#[derive(Default, Debug, Reflect, ShaderType, Clone, Copy)]
pub struct LaneShaderSpeeds {
    pub forward: f32,
    pub backward: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Component)]
pub struct LaneArrowMaterial {
    #[uniform(100)]
    pub single_arrow_color: LinearRgba,
    #[uniform(101)]
    pub double_arrow_color: LinearRgba,
    #[uniform(102)]
    pub background_color: LinearRgba,
    #[uniform(103)]
    pub number_of_arrows: BigF32,
    #[uniform(104)]
    pub speeds: LaneShaderSpeeds,
    #[uniform(105)]
    pub bidirectional: BigU32,
    #[uniform(106)]
    pub interacting: BigU32,
}

impl MaterialExtension for LaneArrowMaterial {
    fn fragment_shader() -> ShaderRef {
        LANE_SHADER_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        LANE_SHADER_PATH.into()
    }
}

#[derive(Resource)]
pub struct SiteAssets {
    pub lift_floor_material: Handle<StandardMaterial>,
    pub billboard_base_mesh: Handle<Mesh>,
    pub billboard_mesh: Handle<Mesh>,
    pub lane_mid_mesh: Handle<Mesh>,
    pub lane_mid_outline: Handle<Mesh>,
    pub lane_end_mesh: Handle<Mesh>,
    pub lane_end_outline: Handle<Mesh>,
    pub box_mesh: Handle<Mesh>,
    pub location_mesh: Handle<Mesh>,
    pub fiducial_mesh: Handle<Mesh>,
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
    pub fiducial_material: Handle<StandardMaterial>,
    pub level_anchor_mesh: Handle<Mesh>,
    pub lift_anchor_mesh: Handle<Mesh>,
    pub site_anchor_mesh: Handle<Mesh>,
    pub lift_wall_material: Handle<StandardMaterial>,
    pub door_cue_material: Handle<StandardMaterial>,
    pub door_cue_highlighted_material: Handle<StandardMaterial>,
    pub text3d_material: Handle<StandardMaterial>,
    pub door_body_material: Handle<StandardMaterial>,
    pub translucent_black: Handle<StandardMaterial>,
    pub translucent_white: Handle<StandardMaterial>,
    pub physical_camera_material: Handle<StandardMaterial>,
    pub occupied_material: Handle<StandardMaterial>,
    pub default_mesh_grey_material: Handle<StandardMaterial>,
    pub location_tag_mesh: Handle<Mesh>,
    pub base_billboard_material: Handle<StandardMaterial>,
    pub charger_material: Handle<StandardMaterial>,
    pub holding_point_material: Handle<StandardMaterial>,
    pub parking_material: Handle<StandardMaterial>,
    pub empty_billboard_material: Handle<StandardMaterial>,
    pub robot_path_rectangle_mesh: Handle<Mesh>,
    pub robot_path_circle_mesh: Handle<Mesh>,
}

pub fn old_default_material(base_color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color,
        perceptual_roughness: 0.089,
        metallic: 0.01,
        // fog_enabled: false,
        ..default()
    }
}

pub fn billboard_material(base_color_texture: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(base_color_texture),
        alpha_mode: AlphaMode::Blend,
        depth_bias: 15.0,
        unlit: true,
        ..default()
    }
}

impl FromWorld for SiteAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let base_billboard_texture =
            asset_server.load("embedded://librmf_site_editor/site/textures/base.png");
        let charger_texture =
            asset_server.load("embedded://librmf_site_editor/site/textures/charging.png");
        let holding_point_texture =
            asset_server.load("embedded://librmf_site_editor/site/textures/holding.png");
        let parking_texture =
            asset_server.load("embedded://librmf_site_editor/site/textures/parking.png");
        let empty_billboard_texture =
            asset_server.load("embedded://librmf_site_editor/site/textures/empty.png");
        let door_cue_texture =
            asset_server.load("embedded://librmf_site_editor/site/textures/door_cue.png");
        let door_cue_highlighted_texture = asset_server
            .load("embedded://librmf_site_editor/site/textures/door_cue_highlighted.png");

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let unassigned_lane_material = materials.add(old_default_material(NAV_UNASSIGNED_COLOR));
        let select_material = materials.add(old_default_material(SELECT_COLOR));
        let hover_material = materials.add(old_default_material(HOVER_COLOR));
        let hover_select_material = materials.add(old_default_material(HOVER_SELECT_COLOR));
        // let hover_select_material = materials.add(Color::srgb_u8(177, 178, 255));
        // let hover_select_material = materials.add(Color::srgb_u8(214, 28, 78));
        let measurement_material =
            materials.add(old_default_material(Color::srgb_u8(250, 234, 72)));
        let fiducial_material = materials.add(old_default_material(Color::srgb(0.1, 0.1, 0.8)));
        let passive_anchor_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.7, 0.6),
            // unlit: true,
            unlit: false,
            perceptual_roughness: 0.089,
            metallic: 0.01,
            ..default()
        });
        let unassigned_anchor_material = materials.add(StandardMaterial {
            // unlit: true,
            unlit: false,
            ..old_default_material(Color::srgb(1.0, 0.9, 0.05))
        });
        let hover_anchor_material = materials.add(StandardMaterial {
            // unlit: true,
            unlit: false,
            ..old_default_material(HOVER_COLOR)
        });
        let select_anchor_material = materials.add(StandardMaterial {
            // unlit: true,
            unlit: false,
            ..old_default_material(SELECT_COLOR)
        });
        let hover_select_anchor_material = materials.add(StandardMaterial {
            // unlit: true,
            unlit: false,
            ..old_default_material(HOVER_SELECT_COLOR)
        });
        let preview_anchor_material = materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            depth_bias: 1.0,
            // unlit: true,
            unlit: false,
            ..old_default_material(Color::srgba(0.98, 0.91, 0.28, 0.5))
        });
        let lift_wall_material =
            materials.add(old_default_material(Color::srgba(0.7, 0.7, 0.7, 1.0)));
        let lift_floor_material = materials.add(old_default_material(Color::srgb(0.3, 0.3, 0.3)));
        let door_body_material = materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..old_default_material(Color::srgba(1., 1., 1., 0.8))
        });
        let translucent_black = materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..old_default_material(Color::srgba(0., 0., 0., 0.8))
        });
        let door_cue_material = materials.add(StandardMaterial {
            base_color_texture: Some(door_cue_texture),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let door_cue_highlighted_material = materials.add(StandardMaterial {
            base_color_texture: Some(door_cue_highlighted_texture),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let text3d_material = materials.add(StandardMaterial {
            base_color_texture: Some(TextAtlas::DEFAULT_IMAGE.clone_weak()),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            cull_mode: None,
            ..Default::default()
        });
        let translucent_white = materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..old_default_material(Color::srgba(1., 1., 1., 0.8))
        });
        let physical_camera_material =
            materials.add(old_default_material(Color::srgb(0.6, 0.7, 0.8)));
        let occupied_material =
            materials.add(old_default_material(Color::srgba(0.8, 0.1, 0.1, 0.2)));
        let default_mesh_grey_material =
            materials.add(old_default_material(Color::srgb(0.7, 0.7, 0.7)));

        let base_billboard_material = materials.add(billboard_material(base_billboard_texture));
        let charger_material: Handle<StandardMaterial> =
            materials.add(billboard_material(charger_texture));
        let holding_point_material = materials.add(billboard_material(holding_point_texture));
        let parking_material = materials.add(billboard_material(parking_texture));
        let empty_billboard_material = materials.add(billboard_material(empty_billboard_texture));

        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let billboard_base_mesh =
            meshes.add(Rectangle::new(BILLBOARD_LENGTH, BILLBOARD_LENGTH / 3.0));
        let billboard_mesh: Handle<Mesh> =
            meshes.add(Rectangle::new(BILLBOARD_LENGTH, BILLBOARD_LENGTH));
        let level_anchor_mesh = meshes.add(
            Mesh::from(
                primitives::Sphere::new(0.05), // TODO(MXG): Make the vertex radius configurable
            )
            .with_generated_outline_normals()
            .unwrap(),
        );
        let lift_anchor_mesh = meshes
            .add(Mesh::from(make_diamond(0.15 / 2.0, 0.15).transform_by(
                Affine3A::from_translation([0.0, 0.0, 0.15 / 2.0].into()),
            )));
        let site_anchor_mesh = meshes.add(Mesh::from(
            primitives::Sphere::new(0.05), // TODO(MXG): Make the vertex radius configurable
        ));
        let lane_mid_mesh = meshes.add(make_flat_square_mesh(1.0));
        let lane_mid_outline = meshes.add(make_flat_rect_mesh(1.0, 1.125));
        let lane_end_mesh = meshes.add(make_flat_disk(
            OffsetCircle {
                radius: LANE_WIDTH / 2.0,
                height: 0.0,
            },
            32,
        ));
        let lane_end_outline = meshes.add(make_flat_disk(
            OffsetCircle {
                radius: 1.125 * LANE_WIDTH / 2.0,
                height: 0.0,
            },
            32,
        ));
        let box_mesh = meshes.add(
            Mesh::from(primitives::Cuboid::new(1., 1., 1.))
                .with_generated_outline_normals()
                .unwrap(),
        );
        let location_mesh = meshes.add(
            Mesh::from(Torus::new(LANE_WIDTH / 2.0, 1.4 * LANE_WIDTH / 2.0))
                .rotated_by(Quat::from_rotation_x(90_f32.to_radians()))
                .scaled_by(Vec3::new(1.0, 1.0, 0.3))
                .with_generated_outline_normals()
                .unwrap(),
        );
        let fiducial_mesh = meshes.add(
            Mesh::from(
                make_icon_halo(1.1 * LANE_WIDTH / 2.0, 0.01, 4).transform_by(
                    Affine3A::from_translation((0.00125 + ZLayer::Location.to_z()) * Vec3::Z),
                ),
            )
            .with_generated_outline_normals()
            .unwrap(),
        );
        let location_tag_mesh = meshes.add(make_location_icon(1.1 * LANE_WIDTH / 2.0, 0.01, 6));
        let physical_camera_mesh = meshes.add(
            make_physical_camera_mesh()
                .with_generated_outline_normals()
                .unwrap(),
        );

        let robot_path_rectangle_mesh = meshes.add(Rectangle::new(1.0, 1.0));
        let robot_path_circle_mesh = meshes.add(Circle::new(1.0));

        Self {
            level_anchor_mesh,
            lift_anchor_mesh,
            site_anchor_mesh,
            lift_floor_material,
            lane_mid_mesh,
            lane_mid_outline,
            lane_end_mesh,
            lane_end_outline,
            billboard_mesh,
            billboard_base_mesh,
            box_mesh,
            location_mesh,
            fiducial_mesh,
            physical_camera_mesh,
            unassigned_lane_material,
            hover_anchor_material,
            select_anchor_material,
            hover_select_anchor_material,
            hover_material,
            select_material,
            hover_select_material,
            measurement_material,
            fiducial_material,
            passive_anchor_material,
            unassigned_anchor_material,
            preview_anchor_material,
            lift_wall_material,
            door_cue_material,
            door_cue_highlighted_material,
            text3d_material,
            door_body_material,
            translucent_black,
            translucent_white,
            physical_camera_material,
            occupied_material,
            default_mesh_grey_material,
            location_tag_mesh,
            base_billboard_material,
            charger_material,
            holding_point_material,
            parking_material,
            empty_billboard_material,
            robot_path_rectangle_mesh,
            robot_path_circle_mesh,
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
