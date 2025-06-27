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

use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::{primitives::InfinitePlane3d, Dir3, Quat, Vec3};
use bevy_picking::{backend::ray::RayMap, prelude::*};
use bevy_render::prelude::*;
use bevy_pbr::prelude::*;
use bevy_transform::components::{GlobalTransform, Transform};
use rmf_site_animate::{Bobbing, Spinning};
use rmf_site_camera::{active_camera_maybe, ActiveCameraQuery};
use std::collections::HashSet;

use crate::{resources::{CursorDaggerMaterial, CursorDaggerMesh, CursorHaloMaterial, CursorHaloMesh}, Hovered, Selectable, VisualCue};


pub mod resources;

/// A resource that keeps track of the unique entities that play a role in
/// displaying the 3D cursor
#[derive(Debug, Clone, Resource)]
pub struct Cursor {
    pub frame: Entity,
    pub halo: Entity,
    pub dagger: Entity,
    // TODO(MXG): Switch the anchor preview when the anchor enters a lift
    // pub level_anchor_placement: Entity,
    // pub site_anchor_placement: Entity,
    // pub frame_placement: Entity,
    // pub preview_model: Option<Entity>,
    dependents: HashSet<Entity>,
    /// Use a &str to label each mode that might want to turn the cursor on
    modes: HashSet<&'static str>,
    blockers: HashSet<Entity>,
}

impl Cursor {
    pub fn add_dependent(&mut self, dependent: Entity, visibility: &mut Query<&mut Visibility>) {
        if self.dependents.insert(dependent) {
            if self.dependents.len() == 1 {
                self.toggle_visibility(visibility);
            }
        }
    }

    pub fn remove_dependent(&mut self, dependent: Entity, visibility: &mut Query<&mut Visibility>) {
        if self.dependents.remove(&dependent) {
            if self.dependents.is_empty() {
                self.toggle_visibility(visibility);
            }
        }
    }

    pub fn add_mode(&mut self, mode: &'static str, visibility: &mut Query<&mut Visibility>) {
        if self.modes.insert(mode) {
            if self.modes.len() == 1 {
                self.toggle_visibility(visibility);
            }
        }
    }

    pub fn remove_mode(&mut self, mode: &'static str, visibility: &mut Query<&mut Visibility>) {
        if self.modes.remove(&mode) {
            if self.modes.is_empty() {
                self.toggle_visibility(visibility);
            }
        }
    }

    pub fn add_blocker(&mut self, e: Entity, visibility: &mut Query<&mut Visibility>) {
        if self.blockers.insert(e) {
            if self.blockers.len() == 1 {
                self.toggle_visibility(visibility);
            }
        }
    }

    pub fn remove_blocker(&mut self, e: Entity, visibility: &mut Query<&mut Visibility>) {
        if self.blockers.remove(&e) {
            if self.blockers.is_empty() {
                self.toggle_visibility(visibility);
            }
        }
    }

    pub fn clear_blockers(&mut self, visibility: &mut Query<&mut Visibility>) {
        let had_blockers = !self.blockers.is_empty();
        self.blockers.clear();
        if had_blockers {
            self.toggle_visibility(visibility);
        }
    }

    fn toggle_visibility(&mut self, visibility: &mut Query<&mut Visibility>) {
        if let Ok(mut v) = visibility.get_mut(self.frame) {
            let new_visible = if self.should_be_visible() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if new_visible != *v {
                *v = new_visible;
            }
        }
    }

    // pub fn remove_preview(&mut self, commands: &mut Commands) {
    //     if let Some(current_preview) = self.preview_model.take() {
    //         commands.entity(current_preview).despawn();
    //     }
    // }

    // pub fn set_model_instance_preview(
    //     &mut self,
    //     commands: &mut Commands,
    //     model_loader: &mut ModelLoader,
    //     model_instance: Option<ModelInstance<Entity>>,
    // ) {
    //     self.remove_preview(commands);
    //     self.preview_model = model_instance.map(|instance| {
    //         model_loader
    //             .spawn_model_instance(self.frame, instance)
    //             .insert(Pending)
    //             .id()
    //     });
    // }

    pub fn should_be_visible(&self) -> bool {
        (!self.dependents.is_empty() || !self.modes.is_empty()) && self.blockers.is_empty()
    }

}

impl FromWorld for Cursor {
    fn from_world(world: &mut World) -> Self {
        // let interaction_assets = world.get_resource::<InteractionAssets>()
        //     .expect("make sure that the InteractionAssets resource is initialized before the Cursor resource");
        // let site_assets = world.get_resource::<SiteAssets>().expect(
        //     "make sure that the SiteAssets resource is initialized before the Cursor resource",
        // );
        let halo_mesh = world.get_resource::<CursorHaloMesh>().unwrap().0.clone();
        let halo_material = world.get_resource::<CursorHaloMaterial>().unwrap().0.clone();
        let dagger_mesh = world.get_resource::<CursorDaggerMesh>().unwrap().0.clone();
        let dagger_material = world.get_resource::<CursorDaggerMaterial>().unwrap().0.clone();
        // let level_anchor_mesh = site_assets.level_anchor_mesh.clone();
        // let site_anchor_mesh = site_assets.site_anchor_mesh.clone();
        // let frame_mesh = interaction_assets.arrow_mesh.clone();
        // let preview_anchor_material = site_assets.preview_anchor_material.clone();
        // let preview_frame_material = site_assets.preview_anchor_material.clone();

        let halo = world
            .spawn((
                Transform::from_scale([0.2, 0.2, 1.].into()),
                Mesh3d(halo_mesh),
                MeshMaterial3d(halo_material),
                Visibility::Inherited,
            ))
            .insert(Spinning::default())
            .insert(VisualCue::no_outline())
            .id();

        let dagger = world
            .spawn((
                Mesh3d(dagger_mesh),
                MeshMaterial3d(dagger_material),
                Transform::default(),
                Visibility::Inherited,
            ))
            .insert(Spinning::default())
            .insert(Bobbing::default())
            .insert(VisualCue::no_outline())
            .id();

        let cursor = world
            .spawn(VisualCue::no_outline())
            .add_children(&[
                halo,
                dagger,
                // frame_placement,
            ])
            .insert((Transform::default(), Visibility::Hidden))
            .id();

        Self {
            frame: cursor,
            halo,
            dagger,
            // frame_placement,
            // preview_model: None,
            dependents: Default::default(),
            modes: Default::default(),
            blockers: Default::default(),
        }
    }
}

/// A unit component that indicates the entity is only for previewing and
/// should never be interacted with. This is applied to the "anchor" that is
/// attached to the cursor.
#[derive(Component, Clone, Copy, Debug)]
pub struct Preview;

#[derive(SystemParam)]
pub struct IntersectGroundPlaneParams<'w, 's> {
    active_camera: ActiveCameraQuery<'w, 's>,
    global_transforms: Query<'w, 's, &'static GlobalTransform>,
    ray_map: Res<'w, RayMap>,
}

impl<'w, 's> IntersectGroundPlaneParams<'w, 's> {
    pub fn ground_plane_intersection(&self) -> Option<Transform> {
        self.plane_intersection(Vec3::ZERO, InfinitePlane3d { normal: Dir3::Z })
    }

    pub fn frame_plane_intersection(&self, frame: Entity) -> Option<Transform> {
        let tf = self.global_transforms.get(frame).ok()?;
        let affine = tf.affine();
        let point = affine.translation.into();
        let normal = Dir3::new(affine.matrix3.col(2).into()).ok()?;
        self.plane_intersection(point, InfinitePlane3d { normal })
    }

    pub fn plane_intersection(
        &self,
        plane_origin: Vec3,
        plane: InfinitePlane3d,
    ) -> Option<Transform> {
        let Ok(e_active_camera) = active_camera_maybe(&self.active_camera) else {
            return None;
        };

        let (_, ray) = self
            .ray_map
            .iter()
            .find(|(id, _)| id.camera == e_active_camera && id.pointer.is_mouse())?;

        let p = ray
            .intersect_plane(plane_origin, plane)
            .map(|distance| ray.get_point(distance))?;

        Some(Transform::from_translation(p).with_rotation(aligned_z_axis(*plane.normal)))
    }
}

pub fn set_visibility(entity: Entity, q_visibility: &mut Query<&mut Visibility>, visible: bool) {
    if let Some(mut visibility) = q_visibility.get_mut(entity).ok() {
        let v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        // Avoid a mutable access if nothing actually needs to change
        if *visibility != v {
            *visibility = v;
        }
    }
}

pub fn hide_cursor(mut visibility: Query<&mut Visibility>, cursor: Res<Cursor>) {
    set_visibility(cursor.frame, &mut visibility, false);
}

#[derive(Component, Debug, Copy, Clone)]
pub struct CursorHoverVisualization;

/// TODO: double check this correct with generic and doesn't generate multiple visualizations
pub fn add_cursor_hover_visualization<T: Component>(
    mut commands: Commands,
    new_entities: Query<Entity, With<T>>,
) {
    for e in &new_entities {
        commands
            .entity(e)
            .insert(CursorHoverVisualization)
            .insert(Selectable::new(e));
    }
}

pub fn update_cursor_hover_visualization(
    entities: Query<(Entity, &Hovered), (With<CursorHoverVisualization>, Changed<Hovered>)>,
    mut removed: RemovedComponents<CursorHoverVisualization>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
) {
    for (e, hovering) in &entities {
        if hovering.cue() {
            cursor.add_dependent(e, &mut visibility);
        } else {
            cursor.remove_dependent(e, &mut visibility);
        }
    }

    for e in removed.read() {
        cursor.remove_dependent(e, &mut visibility);
    }
}

pub fn aligned_z_axis(z: Vec3) -> Quat {
    let z_length = z.length();
    if z_length < 1e-8 {
        // The given direction is too close to singular
        return Quat::IDENTITY;
    }

    let axis = Vec3::Z.cross(z);
    let axis_length = axis.length();
    if axis_length < 1e-8 {
        // The change in angle is too close to zero
        return Quat::IDENTITY;
    }
    let angle = f32::asin(axis_length / z_length);
    Quat::from_axis_angle(axis / axis_length, angle)
}
