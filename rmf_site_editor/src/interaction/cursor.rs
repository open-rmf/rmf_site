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
    site::{AnchorBundle, Pending, SiteAssets, Trashcan},
};
use bevy::{ecs::system::SystemParam, prelude::*, window::PrimaryWindow};
use bevy_mod_raycast::{deferred::RaycastMesh, deferred::RaycastSource, primitives::rays::Ray3d};
use bevy_impulse::*;

use rmf_site_format::{FloorMarker, Model, ModelMarker, PrimitiveShape, WallMarker, WorkcellModel};
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
    pub trashcan: Entity,
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

    fn remove_preview(&mut self, commands: &mut Commands) {
        if let Some(current_preview) = self.preview_model {
            commands.entity(current_preview).set_parent(self.trashcan);
        }
    }

    // TODO(luca) reduce duplication here
    pub fn set_model_preview(&mut self, commands: &mut Commands, model: Option<Model>) {
        self.remove_preview(commands);
        self.preview_model = if let Some(model) = model {
            let e = commands.spawn(model).insert(Pending).id();
            commands.entity(self.frame).push_children(&[e]);
            Some(e)
        } else {
            None
        }
    }

    pub fn set_workcell_model_preview(
        &mut self,
        commands: &mut Commands,
        model: Option<WorkcellModel>,
    ) {
        self.remove_preview(commands);
        self.preview_model = if let Some(model) = model {
            let mut cmd = commands.spawn(Pending);
            let e = cmd.id();
            model.add_bevy_components(&mut cmd);
            commands.entity(self.frame).push_children(&[e]);
            Some(e)
        } else {
            None
        }
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
        let interaction_assets = world.get_resource::<InteractionAssets>()
            .expect("make sure that the InteractionAssets resource is initialized before the Cursor resource");
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

        let trashcan = world.spawn(Trashcan).id();

        Self {
            frame: cursor,
            halo,
            dagger,
            level_anchor_placement,
            site_anchor_placement,
            frame_placement,
            trashcan,
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
    pub fn ground_plane_intersection(&self) -> Option<Vec3> {
        let window = self.primary_windows.get_single().ok()?;
        let cursor_position = window.cursor_position()?;
        let e_active_camera = self.camera_controls.active_camera();
        let active_camera = self.cameras.get(e_active_camera).ok()?;
        let camera_tf = self.global_transforms.get(e_active_camera).ok()?;
        let primary_window = self.primary_window.get_single().ok()?;
        let ray =
            Ray3d::from_screenspace(cursor_position, active_camera, camera_tf, primary_window)?;
        let n_p = Vec3::Z;
        let n_r = ray.direction();
        let denom = n_p.dot(n_r);
        if denom.abs() < 1e-3 {
            // Too close to parallel
            return None;
        }

        Some(ray.origin() - n_r * ray.origin().dot(n_p) / denom)
    }
}

/// Update the virtual cursor (dagger and circle) transform while in inspector mode
pub fn inspector_cursor_transform(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    requests: ContinuousQuery<(), ()>,
    cursor: Res<Cursor>,
    raycast_sources: Query<&RaycastSource<SiteRaycastSet>>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(requests) = requests.view(&key) else {
        return;
    };

    if requests.is_empty() {
        return;
    }

    let Ok(source) = raycast_sources.get_single() else {
        return;
    };
    let intersection = match source.get_nearest_intersection() {
        Some((_, intersection)) => intersection,
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

    let ray = Ray3d::new(intersection.position(), intersection.normal());

    *transform = Transform::from_matrix(ray.to_aligned_transform([0., 0., 1.].into()));
}

/// Update the virtual cursor (dagger and circle) transform while in select anchor mode
pub fn select_anchor_cursor_transform(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    requests: ContinuousQuery<(), ()>,
    cursor: Res<Cursor>,
    mut transforms: Query<&mut Transform>,
    intersect_ground_params: IntersectGroundPlaneParams,
) {
    let Some(requests) = requests.view(&key) else {
        return;
    };

    if requests.is_empty() {
        return;
    }

    let intersection = match intersect_ground_params.ground_plane_intersection() {
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

pub fn select_3d_cursor_transform(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    requests: ContinuousQuery<(), ()>,
    cursor: Res<Cursor>,
    raycast_sources: Query<&RaycastSource<SiteRaycastSet>>,
    models: Query<(), Or<(With<ModelMarker>, With<PrimitiveShape>)>>,
    mut transforms: Query<&mut Transform>,
    hovering: Res<Hovering>,
    intersect_ground_params: IntersectGroundPlaneParams,
    mut visibility: Query<&mut Visibility>,
) {
    let Some(requests) = requests.view(&key) else {
        return;
    };

    if requests.is_empty() {
        return;
    }

    let mut transform = match transforms.get_mut(cursor.frame) {
        Ok(transform) => transform,
        Err(_) => {
            error!("No cursor transform found");
            return;
        }
    };

    let Ok(source) = raycast_sources.get_single() else {
        return;
    };

    // Check if there is an intersection to a mesh, if there isn't fallback to ground plane
    if let Some((_, intersection)) = source.get_nearest_intersection() {
        let Some(triangle) = intersection.triangle() else {
            return;
        };
        // Make sure we are hovering over a model and not anything else (i.e. anchor)
        match cursor.preview_model {
            None => {
                if hovering.0.and_then(|e| models.get(e).ok()).is_some() {
                    // Find the closest triangle vertex
                    // TODO(luca) Also snap to edges of triangles or just disable altogether and snap
                    // to area, then populate a MeshConstraint component to be used by downstream
                    // spawning methods
                    // TODO(luca) there must be a better way to find a minimum given predicate in Rust
                    let triangle_vecs = vec![triangle.v1, triangle.v2];
                    let position = intersection.position();
                    let mut closest_vertex = triangle.v0;
                    let mut closest_dist = position.distance(triangle.v0.into());
                    for v in triangle_vecs {
                        let dist = position.distance(v.into());
                        if dist < closest_dist {
                            closest_dist = dist;
                            closest_vertex = v;
                        }
                    }
                    //closest_vertex = *triangle_vecs.iter().min_by(|position, ver| position.distance(**ver).cmp(closest_dist)).unwrap();
                    let ray = Ray3d::new(closest_vertex.into(), intersection.normal());
                    *transform = Transform::from_matrix(
                        ray.to_aligned_transform([0., 0., 1.].into()),
                    );
                    set_visibility(cursor.frame, &mut visibility, true);
                } else {
                    // Hide the cursor
                    set_visibility(cursor.frame, &mut visibility, false);
                }
            }
            Some(_) => {
                // If we are placing a model avoid snapping to faced and just project to
                // ground plane
                let intersection = match intersect_ground_params.ground_plane_intersection()
                {
                    Some(intersection) => intersection,
                    None => {
                        return;
                    }
                };
                set_visibility(cursor.frame, &mut visibility, true);
                *transform = Transform::from_translation(intersection);
            }
        }
    } else {
        let intersection = match intersect_ground_params.ground_plane_intersection() {
            Some(intersection) => intersection,
            None => {
                return;
            }
        };
        set_visibility(cursor.frame, &mut visibility, true);
        *transform = Transform::from_translation(intersection);
    }
}

pub fn update_cursor_transform(
    mode: Res<InteractionMode>,
    cursor: Res<Cursor>,
    raycast_sources: Query<&RaycastSource<SiteRaycastSet>>,
    models: Query<(), Or<(With<ModelMarker>, With<PrimitiveShape>)>>,
    mut transforms: Query<&mut Transform>,
    hovering: Res<Hovering>,
    intersect_ground_params: IntersectGroundPlaneParams,
    mut visibility: Query<&mut Visibility>,
) {
    match &*mode {
        InteractionMode::Inspect => {
            // TODO(luca) this will not work if more than one raycast source exist
            let Ok(source) = raycast_sources.get_single() else {
                return;
            };
            let intersection = match source.get_nearest_intersection() {
                Some((_, intersection)) => intersection,
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

            let ray = Ray3d::new(intersection.position(), intersection.normal());

            *transform = Transform::from_matrix(ray.to_aligned_transform([0., 0., 1.].into()));
        }
        InteractionMode::SelectAnchor(_) => {
            let intersection = match intersect_ground_params.ground_plane_intersection() {
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
        // TODO(luca) snap to features of meshes
        InteractionMode::SelectAnchor3D(_mode) => {
            let mut transform = match transforms.get_mut(cursor.frame) {
                Ok(transform) => transform,
                Err(_) => {
                    error!("No cursor transform found");
                    return;
                }
            };

            let Ok(source) = raycast_sources.get_single() else {
                return;
            };

            // Check if there is an intersection to a mesh, if there isn't fallback to ground plane
            if let Some((_, intersection)) = source.get_nearest_intersection() {
                let Some(triangle) = intersection.triangle() else {
                    return;
                };
                // Make sure we are hovering over a model and not anything else (i.e. anchor)
                match cursor.preview_model {
                    None => {
                        if hovering.0.and_then(|e| models.get(e).ok()).is_some() {
                            // Find the closest triangle vertex
                            // TODO(luca) Also snap to edges of triangles or just disable altogether and snap
                            // to area, then populate a MeshConstraint component to be used by downstream
                            // spawning methods
                            // TODO(luca) there must be a better way to find a minimum given predicate in Rust
                            let triangle_vecs = vec![triangle.v1, triangle.v2];
                            let position = intersection.position();
                            let mut closest_vertex = triangle.v0;
                            let mut closest_dist = position.distance(triangle.v0.into());
                            for v in triangle_vecs {
                                let dist = position.distance(v.into());
                                if dist < closest_dist {
                                    closest_dist = dist;
                                    closest_vertex = v;
                                }
                            }
                            //closest_vertex = *triangle_vecs.iter().min_by(|position, ver| position.distance(**ver).cmp(closest_dist)).unwrap();
                            let ray = Ray3d::new(closest_vertex.into(), intersection.normal());
                            *transform = Transform::from_matrix(
                                ray.to_aligned_transform([0., 0., 1.].into()),
                            );
                            set_visibility(cursor.frame, &mut visibility, true);
                        } else {
                            // Hide the cursor
                            set_visibility(cursor.frame, &mut visibility, false);
                        }
                    }
                    Some(_) => {
                        // If we are placing a model avoid snapping to faced and just project to
                        // ground plane
                        let intersection = match intersect_ground_params.ground_plane_intersection()
                        {
                            Some(intersection) => intersection,
                            None => {
                                return;
                            }
                        };
                        set_visibility(cursor.frame, &mut visibility, true);
                        *transform = Transform::from_translation(intersection);
                    }
                }
            } else {
                let intersection = match intersect_ground_params.ground_plane_intersection() {
                    Some(intersection) => intersection,
                    None => {
                        return;
                    }
                };
                set_visibility(cursor.frame, &mut visibility, true);
                *transform = Transform::from_translation(intersection);
            }
        }
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

// This system makes sure model previews are not picked up by raycasting
pub fn make_model_previews_not_selectable(
    mut commands: Commands,
    new_models: Query<Entity, (With<ModelMarker>, Added<Selectable>)>,
    cursor: Res<Cursor>,
) {
    if let Some(e) = cursor.preview_model.and_then(|m| new_models.get(m).ok()) {
        commands
            .entity(e)
            .remove::<Selectable>()
            .remove::<RaycastMesh<SiteRaycastSet>>();
    }
}
