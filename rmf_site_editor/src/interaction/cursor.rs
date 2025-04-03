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
    site::{AnchorBundle, ModelLoader, Pending, SiteAssets},
};
use bevy::{ecs::system::SystemParam, prelude::*, window::PrimaryWindow};
use bevy_mod_raycast::primitives::{rays::Ray3d, Primitive3d};

use rmf_site_format::{FloorMarker, ModelInstance, WallMarker};
use std::collections::HashSet;

/// A resource that keeps track of the unique entities that play a role in
/// displaying the 3D cursor
#[derive(Debug, Clone, Resource)]
pub struct Cursor {
    pub frame: Entity,
    pub halo: Entity,
    pub dagger: Entity,
    // TODO(MXG): Switch the anchor preview when the anchor enters a lift
    pub level_anchor_placement: Entity,
    pub site_anchor_placement: Entity,
    pub frame_placement: Entity,
    pub preview_model: Option<Entity>,
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

    pub fn remove_preview(&mut self, commands: &mut Commands) {
        if let Some(current_preview) = self.preview_model.take() {
            commands.entity(current_preview).despawn_recursive();
        }
    }

    pub fn set_model_instance_preview(
        &mut self,
        commands: &mut Commands,
        model_loader: &mut ModelLoader,
        model_instance: Option<ModelInstance<Entity>>,
    ) {
        self.remove_preview(commands);
        self.preview_model = model_instance.map(|instance| {
            model_loader
                .spawn_model_instance(self.frame, instance)
                .insert(Pending)
                .id()
        });
    }

    pub fn should_be_visible(&self) -> bool {
        (!self.dependents.is_empty() || !self.modes.is_empty()) && self.blockers.is_empty()
    }

    pub fn is_placement_anchor(&self, entity: Entity) -> bool {
        self.level_anchor_placement == entity || self.site_anchor_placement == entity
    }
}

impl FromWorld for Cursor {
    fn from_world(world: &mut World) -> Self {
        // startup
        let interaction_assets = world.get_resource::<InteractionAssets>()
            .expect("make sure that the InteractionAssets resource is initialized before the Cursor resource");
        // startup
        let site_assets = world.get_resource::<SiteAssets>().expect(
            "make sure that the SiteAssets resource is initialized before the Cursor resource",
        );
        let halo_mesh = interaction_assets.halo_mesh.clone();
        let halo_material = interaction_assets.halo_material.clone();
        let dagger_mesh = interaction_assets.dagger_mesh.clone();
        let dagger_material = interaction_assets.dagger_material.clone();
        let level_anchor_mesh = site_assets.level_anchor_mesh.clone();
        let site_anchor_mesh = site_assets.site_anchor_mesh.clone();
        let frame_mesh = interaction_assets.arrow_mesh.clone();
        let preview_anchor_material = site_assets.preview_anchor_material.clone();
        let preview_frame_material = site_assets.preview_anchor_material.clone();

        let halo = world
            .spawn(PbrBundle {
                transform: Transform::from_scale([0.2, 0.2, 1.].into()),
                mesh: halo_mesh,
                material: halo_material,
                visibility: Visibility::Inherited,
                ..default()
            })
            .insert(Spinning::default())
            .insert(VisualCue::no_outline())
            .id();

        let dagger = world
            .spawn(PbrBundle {
                mesh: dagger_mesh,
                material: dagger_material,
                visibility: Visibility::Inherited,
                ..default()
            })
            .insert(Spinning::default())
            .insert(Bobbing::default())
            .insert(VisualCue::no_outline())
            .id();

        let level_anchor_placement = world
            .spawn(AnchorBundle::new([0., 0.].into()).visible(false))
            .insert(Pending)
            .insert(Preview)
            .insert(VisualCue::no_outline())
            .with_children(|parent| {
                parent.spawn(PbrBundle {
                    mesh: level_anchor_mesh,
                    material: preview_anchor_material.clone(),
                    ..default()
                });
            })
            .id();

        let site_anchor_placement = world
            .spawn(AnchorBundle::new([0., 0.].into()).visible(false))
            .insert(Pending)
            .insert(Preview)
            .insert(VisualCue::no_outline())
            .with_children(|parent| {
                parent.spawn(PbrBundle {
                    mesh: site_anchor_mesh,
                    material: preview_anchor_material,
                    ..default()
                });
            })
            .id();

        let frame_placement = world
            .spawn(AnchorBundle::new([0., 0.].into()).visible(false))
            .insert(Pending)
            .insert(Preview)
            .insert(VisualCue::no_outline())
            .with_children(|parent| {
                parent.spawn(PbrBundle {
                    mesh: frame_mesh,
                    material: preview_frame_material,
                    transform: Transform::from_scale(Vec3::new(0.2, 0.2, 0.2)),
                    ..default()
                });
            })
            .id();

        let cursor = world
            .spawn(VisualCue::no_outline())
            .push_children(&[
                halo,
                dagger,
                level_anchor_placement,
                site_anchor_placement,
                frame_placement,
            ])
            .insert(SpatialBundle {
                visibility: Visibility::Hidden,
                ..default()
            })
            .id();

        Self {
            frame: cursor,
            halo,
            dagger,
            level_anchor_placement,
            site_anchor_placement,
            frame_placement,
            preview_model: None,
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
    primary_windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_controls: Res<'w, CameraControls>,
    cameras: Query<'w, 's, &'static Camera>,
    global_transforms: Query<'w, 's, &'static GlobalTransform>,
    primary_window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}

impl<'w, 's> IntersectGroundPlaneParams<'w, 's> {
    pub fn ground_plane_intersection(&self) -> Option<Transform> {
        let ground_plane = Primitive3d::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Z,
        };
        self.primitive_intersection(ground_plane)
    }

    pub fn frame_plane_intersection(&self, frame: Entity) -> Option<Transform> {
        let tf = self.global_transforms.get(frame).ok()?;
        let affine = tf.affine();
        let point = affine.translation.into();
        let normal = affine.matrix3.col(2).into();
        self.primitive_intersection(Primitive3d::Plane { point, normal })
    }

    pub fn primitive_intersection(&self, primitive: Primitive3d) -> Option<Transform> {
        let window = self.primary_windows.get_single().ok()?;
        let cursor_position = window.cursor_position()?;
        let e_active_camera = self.camera_controls.active_camera();
        let active_camera = self.cameras.get(e_active_camera).ok()?;
        let camera_tf = self.global_transforms.get(e_active_camera).ok()?;
        let primary_window = self.primary_window.get_single().ok()?;
        let ray =
            Ray3d::from_screenspace(cursor_position, active_camera, camera_tf, primary_window)?;

        let n = *match &primitive {
            Primitive3d::Plane { normal, .. } => normal,
            _ => {
                warn!("Unsupported primitive type found");
                return None;
            }
        };
        let p = ray
            .intersects_primitive(primitive)
            .map(|intersection| intersection.position())?;

        Some(Transform::from_translation(p).with_rotation(aligned_z_axis(n)))
    }
}

pub fn hide_cursor(mut visibility: Query<&mut Visibility>, cursor: Res<Cursor>) {
    set_visibility(cursor.frame, &mut visibility, false);
}

#[derive(Component, Debug, Copy, Clone)]
pub struct CursorHoverVisualization;

pub fn add_cursor_hover_visualization(
    mut commands: Commands,
    new_entities: Query<Entity, Or<(Added<FloorMarker>, Added<WallMarker>)>>,
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
