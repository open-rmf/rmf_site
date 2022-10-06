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
use bevy::prelude::*;

#[derive(Clone, Debug)]
pub struct InteractionAssets {
    pub dagger_mesh: Handle<Mesh>,
    pub dagger_material: Handle<StandardMaterial>,
    pub halo_mesh: Handle<Mesh>,
    pub halo_material: Handle<StandardMaterial>,
    pub arrow_mesh: Handle<Mesh>,
    pub flat_square_mesh: Handle<Mesh>,
    pub x_axis_materials: DraggableMaterialSet,
    pub y_axis_materials: DraggableMaterialSet,
    pub z_plane_materials: DraggableMaterialSet,
}

impl InteractionAssets {
    pub fn make_draggable_axis(
        &self,
        command: &mut Commands,
        // What entity will be moved when this gizmo is dragged
        for_entity: Entity,
        // What entity should be the parent frame of this gizmo
        parent: Entity,
        material_set: DraggableMaterialSet,
        offset: Vec3,
        rotation: Quat,
        scale: f32,
    ) -> Entity {
        return command.entity(parent).add_children(|parent| {
            parent
                .spawn_bundle(PbrBundle {
                    transform: Transform::from_rotation(rotation)
                        .with_translation(offset)
                        .with_scale(Vec3::splat(scale)),
                    mesh: self.arrow_mesh.clone(),
                    material: material_set.passive.clone(),
                    ..default()
                })
                .insert(DragAxis {
                    along: [0., 0., 1.].into(),
                })
                .insert(Draggable::new(for_entity, Some(material_set)))
                .id()
        });
    }

    pub fn add_anchor_draggable_arrows(
        &self,
        command: &mut Commands,
        anchor: Entity,
        cue: &mut AnchorVisualCue,
    ) {
        let drag_parent = command
            .entity(anchor)
            .add_children(|parent| parent.spawn_bundle(SpatialBundle::default()).id());

        let height = 0.01;
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
            self.make_draggable_axis(command, anchor, drag_parent, m, p, r, scale);
        }

        cue.drag = Some(drag_parent);
    }
}

impl FromWorld for InteractionAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let dagger_mesh = meshes.add(make_dagger_mesh());
        let halo_mesh = meshes.add(make_halo_mesh());
        let arrow_mesh = meshes.add(make_cylinder_arrow_mesh());
        let flat_square_mesh = meshes.add(make_flat_square_mesh(1.0).into());

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
        let x_axis_materials = DraggableMaterialSet::make_x_axis(&mut materials);
        let y_axis_materials = DraggableMaterialSet::make_y_axis(&mut materials);
        let z_plane_materials = DraggableMaterialSet::make_z_plane(&mut materials);

        Self {
            dagger_mesh,
            dagger_material,
            halo_mesh,
            halo_material,
            arrow_mesh,
            flat_square_mesh,
            x_axis_materials,
            y_axis_materials,
            z_plane_materials,
        }
    }
}
