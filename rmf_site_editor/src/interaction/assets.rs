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

use crate::{interaction::*, shapes::*};
use bevy::{math::Affine3A, prelude::*, render::view::visibility::RenderLayers};
use bevy_polyline::{
    material::PolylineMaterial,
    polyline::{Polyline, PolylineBundle},
};

#[derive(Clone, Debug, Resource)]
pub struct InteractionAssets {
    pub dagger_mesh: Handle<Mesh>,
    pub dagger_material: Handle<StandardMaterial>,
    pub halo_mesh: Handle<Mesh>,
    pub halo_material: Handle<StandardMaterial>,
    pub arrow_mesh: Handle<Mesh>,
    pub point_light_socket_mesh: Handle<Mesh>,
    pub point_light_shine_mesh: Handle<Mesh>,
    pub spot_light_cover_mesh: Handle<Mesh>,
    pub spot_light_shine_mesh: Handle<Mesh>,
    pub directional_light_cover_mesh: Handle<Mesh>,
    pub directional_light_shine_mesh: Handle<Mesh>,
    pub physical_light_cover_material: Handle<StandardMaterial>,
    pub direction_light_cover_material: Handle<StandardMaterial>,
    pub x_axis_materials: GizmoMaterialSet,
    pub y_axis_materials: GizmoMaterialSet,
    pub z_axis_materials: GizmoMaterialSet,
    pub z_plane_materials: GizmoMaterialSet,
    pub lift_doormat_available_materials: GizmoMaterialSet,
    pub lift_doormat_unavailable_materials: GizmoMaterialSet,
    pub centimeter_finite_grid: Vec<(Handle<Polyline>, Handle<PolylineMaterial>)>,
}

impl InteractionAssets {
    pub fn make_orientation_cue_meshes(&self, commands: &mut Commands, parent: Entity, scale: f32) {
        // The arrows should originate in the mesh origin
        let pos = Vec3::splat(0.0);
        let rot_x = Quat::from_rotation_y(90_f32.to_radians());
        let rot_y = Quat::from_rotation_x(-90_f32.to_radians());
        let rot_z = Quat::default();
        let x_mat = self.x_axis_materials.clone();
        let y_mat = self.y_axis_materials.clone();
        let z_mat = self.z_axis_materials.clone();
        self.make_axis(commands, None, parent, x_mat, pos, rot_x, scale);
        self.make_axis(commands, None, parent, y_mat, pos, rot_y, scale);
        self.make_axis(commands, None, parent, z_mat, pos, rot_z, scale);
    }

    pub fn make_axis(
        &self,
        command: &mut Commands,
        // What entity will be moved when this gizmo is dragged
        for_entity_opt: Option<Entity>,
        // What entity should be the parent frame of this gizmo
        parent: Entity,
        material_set: GizmoMaterialSet,
        offset: Vec3,
        rotation: Quat,
        scale: f32,
    ) -> Entity {
        return command.entity(parent).add_children(|parent| {
            let mut child_entity = parent.spawn(PbrBundle {
                transform: Transform::from_rotation(rotation)
                    .with_translation(offset)
                    .with_scale(Vec3::splat(scale)),
                mesh: self.arrow_mesh.clone(),
                material: material_set.passive.clone(),
                ..default()
            });

            child_entity.insert(LimitScaleFactor {
                distance_to_start_scaling: 10.0,
                original_scale: 0.2,
            });
            if let Some(for_entity) = for_entity_opt {
                child_entity
                    .insert(DragAxisBundle::new(for_entity, Vec3::Z).with_materials(material_set));
            }
            child_entity.id()
        });
    }

    pub fn make_draggable_axis(
        &self,
        command: &mut Commands,
        // What entity will be moved when this gizmo is dragged
        for_entity: Entity,
        // What entity should be the parent frame of this gizmo
        parent: Entity,
        material_set: GizmoMaterialSet,
        offset: Vec3,
        rotation: Quat,
        scale: f32,
    ) -> Entity {
        self.make_axis(
            command,
            Some(for_entity),
            parent,
            material_set,
            offset,
            rotation,
            scale,
        )
    }

    #[allow(non_snake_case)]
    pub fn add_anchor_gizmos_2D(
        &self,
        commands: &mut Commands,
        anchor: Entity,
        cue: &mut AnchorVisualization,
    ) {
        let drag_parent = commands.entity(anchor).add_children(|parent| {
            parent
                .spawn(SpatialBundle::default())
                .insert(VisualCue::no_outline().irregular().always_xray())
                .id()
        });

        let height = 0.0;
        let scale = 0.2;
        let offset = 0.15;
        for (m, p, r) in [
            (
                self.x_axis_materials.clone(),
                Vec3::new(offset, 0., height),
                Quat::from_rotation_y(90_f32.to_radians()),
            ),
            (
                self.x_axis_materials.clone(),
                Vec3::new(-offset, 0., height),
                Quat::from_rotation_y(-90_f32.to_radians()),
            ),
            (
                self.y_axis_materials.clone(),
                Vec3::new(0., offset, height),
                Quat::from_rotation_x(-90_f32.to_radians()),
            ),
            (
                self.y_axis_materials.clone(),
                Vec3::new(0., -offset, height),
                Quat::from_rotation_x(90_f32.to_radians()),
            ),
        ] {
            self.make_draggable_axis(commands, anchor, drag_parent, m, p, r, scale);
        }

        cue.drag = Some(drag_parent);
    }

    #[allow(non_snake_case)]
    pub fn add_anchor_gizmos_3D(
        &self,
        commands: &mut Commands,
        anchor: Entity,
        cue: &mut AnchorVisualization,
        draggable: bool,
    ) {
        let drag_parent = commands.entity(anchor).add_children(|parent| {
            parent
                .spawn(SpatialBundle::default())
                .insert(VisualCue::no_outline().irregular().always_xray())
                .id()
        });

        let for_entity = if draggable { Some(anchor) } else { None };
        let scale = 0.2;
        let offset = 0.15;
        for (m, p, r) in [
            (
                self.x_axis_materials.clone(),
                Vec3::new(offset, 0., 0.),
                Quat::from_rotation_y(90_f32.to_radians()),
            ),
            (
                self.y_axis_materials.clone(),
                Vec3::new(0., offset, 0.),
                Quat::from_rotation_x(-90_f32.to_radians()),
            ),
            (
                self.z_axis_materials.clone(),
                Vec3::new(0., 0., offset),
                Quat::IDENTITY,
            ),
        ] {
            self.make_axis(commands, for_entity, drag_parent, m, p, r, scale);
        }

        commands.entity(drag_parent).add_children(|parent| {
            for (polyline, material) in &self.centimeter_finite_grid {
                parent.spawn(PolylineBundle {
                    polyline: polyline.clone(),
                    material: material.clone(),
                    ..default()
                });
            }
        });

        cue.drag = Some(drag_parent);
    }

    pub fn lift_doormat_materials(&self, available: bool) -> GizmoMaterialSet {
        if available {
            self.lift_doormat_available_materials.clone()
        } else {
            self.lift_doormat_unavailable_materials.clone()
        }
    }
}

impl FromWorld for InteractionAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let dagger_mesh = meshes.add(make_dagger_mesh());
        let halo_mesh = meshes.add(make_halo_mesh());
        let arrow_mesh = meshes.add(make_cylinder_arrow_mesh());
        let point_light_socket_mesh = meshes.add(
            make_cylinder(0.06, 0.02)
                .transform_by(Affine3A::from_translation(0.04 * Vec3::Z))
                .into(),
        );
        let point_light_shine_mesh = meshes.add(Mesh::from(shape::UVSphere {
            radius: 0.05,
            ..Default::default()
        }));
        let spot_light_cover_mesh = meshes.add(
            make_smooth_wrap(
                [
                    Circle {
                        radius: 0.05,
                        height: 0.0,
                    },
                    Circle {
                        radius: 0.01,
                        height: 0.04,
                    },
                ],
                32,
            )
            .into(),
        );
        let spot_light_shine_mesh = meshes.add(
            Mesh::from(
                make_bottom_circle(
                    Circle {
                        radius: 0.05,
                        height: 0.0,
                    },
                    32,
                )
                .merge_with(make_top_circle(
                    Circle {
                        radius: 0.01,
                        height: 0.04,
                    },
                    32,
                )),
            )
            .with_generated_outline_normals()
            .unwrap(),
        );
        let directional_light_cover_mesh = meshes.add(
            Mesh::from(
                make_cylinder(0.02, 0.1).transform_by(Affine3A::from_translation(0.01 * Vec3::Z)),
            )
            .with_generated_outline_normals()
            .unwrap(),
        );
        let directional_light_shine_mesh = meshes.add(
            Mesh::from(
                make_cylinder(0.02, 0.1).transform_by(Affine3A::from_translation(-0.01 * Vec3::Z)),
            )
            .with_generated_outline_normals()
            .unwrap(),
        );

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let halo_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        let dagger_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        });
        let light_cover_color = Color::rgb(0.6, 0.7, 0.8);
        let physical_light_cover_material = materials.add(StandardMaterial {
            base_color: light_cover_color,
            ..default()
        });
        let direction_light_cover_material = materials.add(StandardMaterial {
            base_color: light_cover_color,
            unlit: true,
            ..default()
        });
        let x_axis_materials = GizmoMaterialSet::make_x_axis(&mut materials);
        let y_axis_materials = GizmoMaterialSet::make_y_axis(&mut materials);
        let z_axis_materials = GizmoMaterialSet::make_z_axis(&mut materials);
        let z_plane_materials = GizmoMaterialSet::make_z_plane(&mut materials);
        let lift_doormat_available_materials = GizmoMaterialSet {
            passive: materials.add(StandardMaterial {
                base_color: Color::rgba(0.1, 0.9, 0.1, 0.1),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            hover: materials.add(StandardMaterial {
                base_color: Color::rgba(0.1, 0.9, 0.1, 0.9),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            drag: materials.add(StandardMaterial {
                base_color: Color::rgba(0.1, 0.9, 0.1, 0.9),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
        };
        let lift_doormat_unavailable_materials = GizmoMaterialSet {
            passive: materials.add(StandardMaterial {
                base_color: Color::rgba(0.9, 0.1, 0.1, 0.1),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            hover: materials.add(StandardMaterial {
                base_color: Color::rgba(0.9, 0.1, 0.1, 0.9),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            drag: materials.add(StandardMaterial {
                base_color: Color::rgba(0.9, 0.1, 0.1, 0.9),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
        };

        let centimeter_finite_grid = {
            let (polylines, polyline_mats): (Vec<_>, Vec<_>) =
                make_metric_finite_grid(0.01, 100, Color::WHITE)
                    .into_iter()
                    .unzip();
            let mut polyline_assets = world.get_resource_mut::<Assets<Polyline>>().unwrap();
            let polylines: Vec<Handle<Polyline>> = polylines
                .into_iter()
                .map(|p| polyline_assets.add(p))
                .collect();
            let mut polyline_mat_assets = world
                .get_resource_mut::<Assets<PolylineMaterial>>()
                .unwrap();
            let polyline_mats: Vec<Handle<PolylineMaterial>> = polyline_mats
                .into_iter()
                .map(|m| polyline_mat_assets.add(m))
                .collect();
            polylines
                .into_iter()
                .zip(polyline_mats.into_iter())
                .collect()
        };

        Self {
            dagger_mesh,
            dagger_material,
            halo_mesh,
            halo_material,
            arrow_mesh,
            point_light_socket_mesh,
            point_light_shine_mesh,
            spot_light_cover_mesh,
            spot_light_shine_mesh,
            directional_light_cover_mesh,
            directional_light_shine_mesh,
            physical_light_cover_material,
            direction_light_cover_material,
            x_axis_materials,
            y_axis_materials,
            z_axis_materials,
            z_plane_materials,
            lift_doormat_available_materials,
            lift_doormat_unavailable_materials,
            centimeter_finite_grid,
        }
    }
}
