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

use crate::{
    animate::*,
    interaction::*,
    site::{AnchorBundle, AnchorDependents, Pending, SiteAssets},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_mod_picking::PickingRaycastSet;
use bevy_mod_raycast::{Intersection, Ray3d};
use std::collections::HashSet;

/// A resource that keeps track of the unique entities that play a role in
/// displaying the 3D cursor
#[derive(Debug, Clone)]
pub struct Cursor {
    pub frame: Entity,
    pub halo: Entity,
    pub dagger: Entity,
    pub anchor_placement: Entity,
    dependents: HashSet<Entity>,
    /// Use a &str to label each mode that might want to turn the cursor on
    modes: HashSet<&'static str>,
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

    fn toggle_visibility(&mut self, visibility: &mut Query<&mut Visibility>) {
        if let Ok(mut v) = visibility.get_mut(self.frame) {
            let visible = self.should_be_visible();
            if v.is_visible != visible {
                v.is_visible = visible;
            }
        }
    }

    pub fn should_be_visible(&self) -> bool {
        !self.dependents.is_empty() || !self.modes.is_empty()
    }
}

impl FromWorld for Cursor {
    fn from_world(world: &mut World) -> Self {
        let interaction_assets = world.get_resource::<InteractionAssets>()
            .expect("make sure that the InteractionAssets resource is initialized before the Cursor resource");
        let site_assets = world.get_resource::<SiteAssets>().expect(
            "make sure that the SiteAssets resource is initialized before the Cursor resource",
        );
        let halo_mesh = interaction_assets.halo_mesh.clone();
        let halo_material = interaction_assets.halo_material.clone();
        let dagger_mesh = interaction_assets.dagger_mesh.clone();
        let dagger_material = interaction_assets.dagger_material.clone();
        let anchor_mesh = site_assets.anchor_mesh.clone();
        let preview_anchor_material = site_assets.preview_anchor_material.clone();

        let halo = world
            .spawn()
            .insert_bundle(PbrBundle {
                transform: Transform::from_scale([0.2, 0.2, 1.].into()),
                mesh: halo_mesh,
                material: halo_material,
                visibility: Visibility { is_visible: true },
                ..default()
            })
            .insert(Spinning::default())
            .id();

        let dagger = world
            .spawn()
            .insert_bundle(PbrBundle {
                mesh: dagger_mesh,
                material: dagger_material,
                visibility: Visibility { is_visible: true },
                ..default()
            })
            .insert(Spinning::default())
            .insert(Bobbing::default())
            .id();

        let anchor_placement = world
            .spawn()
            .insert_bundle(AnchorBundle::new([0., 0.]).visible(false))
            .insert(Pending)
            .insert(Preview)
            .with_children(|parent| {
                parent.spawn_bundle(PbrBundle {
                    mesh: anchor_mesh,
                    material: preview_anchor_material,
                    transform: Transform::from_rotation(Quat::from_rotation_x(90_f32.to_radians())),
                    ..default()
                });
            })
            .id();

        let cursor = world
            .spawn()
            .push_children(&[halo, dagger, anchor_placement])
            .insert_bundle(SpatialBundle::default())
            .id();

        Self {
            frame: cursor,
            halo,
            dagger,
            anchor_placement,
            dependents: Default::default(),
            modes: Default::default(),
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
    windows: Res<'w, Windows>,
    camera_controls: Res<'w, CameraControls>,
    cameras: Query<'w, 's, &'static Camera>,
    global_transforms: Query<'w, 's, &'static GlobalTransform>,
}

fn intersect_ground_plane(params: &IntersectGroundPlaneParams) -> Option<Vec3> {
    let window = params.windows.get_primary()?;
    let cursor_position = window.cursor_position()?;
    let e_active_camera = params.camera_controls.active_camera();
    let active_camera = params.cameras.get(e_active_camera).ok()?;
    let camera_tf = params.global_transforms.get(e_active_camera).ok()?;
    let ray = Ray3d::from_screenspace(cursor_position, active_camera, camera_tf)?;
    let n_p = Vec3::Z;
    let n_r = ray.direction();
    let denom = n_p.dot(n_r);
    if denom.abs() < 1e-3 {
        // Too close to parallel
        return None;
    }

    Some(ray.origin() - n_r * ray.origin().dot(n_p) / denom)
}

pub fn update_cursor_transform(
    mode: Res<InteractionMode>,
    cursor: Res<Cursor>,
    intersections: Query<&Intersection<PickingRaycastSet>>,
    mut transforms: Query<&mut Transform>,
    select_anchor_params: IntersectGroundPlaneParams,
) {
    match *mode {
        InteractionMode::Inspect => {
            let intersection = match intersections.iter().last() {
                Some(intersection) => intersection,
                None => {
                    return;
                }
            };

            let mut transform = match transforms.get_mut(cursor.frame) {
                Ok(transform) => transform,
                Err(_) => {
                    return;
                }
            };

            let ray = match intersection.normal_ray() {
                Some(ray) => ray,
                None => {
                    return;
                }
            };

            *transform = Transform::from_matrix(ray.to_aligned_transform([0., 0., 1.].into()));
        }
        InteractionMode::SelectAnchor(_) => {
            let intersection = match intersect_ground_plane(&select_anchor_params) {
                Some(intersection) => intersection,
                None => {
                    return;
                }
            };

            let mut transform = match transforms.get_mut(cursor.frame) {
                Ok(transform) => transform,
                Err(_) => {
                    return;
                }
            };

            *transform = Transform::from_translation(intersection);
        }
    }
}

pub fn hide_cursor(mut visibility: Query<&mut Visibility>, cursor: Res<Cursor>) {
    set_visibility(cursor.frame, &mut visibility, false);
}
