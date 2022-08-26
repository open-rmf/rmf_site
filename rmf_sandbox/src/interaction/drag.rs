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

use crate::interaction::*;
use bevy::prelude::*;
use bevy_mod_picking::{PickingRaycastSet, PickableBundle};
use bevy_mod_raycast::{Intersection, Ray3d};

#[derive(Debug, Clone, Copy)]
pub struct InitialDragConditions {
    click_point: Vec3,
    entity_tf: Transform,
}

#[derive(Debug, Clone)]
pub struct DraggableMaterialSet {
    pub passive: Handle<StandardMaterial>,
    pub hover: Handle<StandardMaterial>,
    pub drag: Handle<StandardMaterial>,
}

impl DraggableMaterialSet {
    pub fn make_x_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self{
            passive: materials.add(Color::rgb(1., 0., 0.).into()),
            hover: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
            drag: materials.add(Color::rgb(0.7, 0., 0.).into()),
        }
    }

    pub fn make_y_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self{
            passive: materials.add(Color::rgb(0., 0.9, 0.).into()),
            hover: materials.add(Color::rgb(0.5, 1.0, 0.5).into()),
            drag: materials.add(Color::rgb(0., 0.6, 0.).into()),
        }
    }

    pub fn make_z_plane(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self{
            passive: materials.add(Color::rgba(0., 0., 1., 0.6).into()),
            hover: materials.add(Color::rgba(0.3, 0.3, 1., 0.6).into()),
            drag: materials.add(Color::rgba(0., 0., 0.7, 0.9).into()),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Draggable {
    pub for_entity: Entity,
    pub materials: DraggableMaterialSet,
    pub initial: Option<InitialDragConditions>,
}

impl Draggable {
    pub fn new(
        for_entity: Entity,
        materials: DraggableMaterialSet,
    ) -> Self {
        Self{for_entity, materials, initial: None}
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DragAxis {
    /// The gizmo can only be dragged along this axis
    pub along: Vec3,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DragPlane {
    /// The gizmo can only be dragged in the plane orthogonal to this vector
    pub in_plane: Vec3,
}

/// Instruction to move an entity to a new transform. This should be caught with
/// an EventReader<MoveTo>.
#[derive(Debug, Clone, Copy)]
pub struct MoveTo {
    pub entity: Entity,
    pub transform: Transform,
}

pub fn make_gizmos_pickable(
    mut command: Commands,
    drag_axis: Query<Entity, Added<DragAxis>>,
    drag_plane: Query<Entity, Added<DragPlane>>,
) {
    for e in drag_axis.iter().chain(drag_plane.iter()) {
        command.entity(e).insert_bundle(PickableBundle::default());
    }
}

pub fn update_drag_click_start(
    mut draggables: Query<(&mut Draggable, &mut Handle<StandardMaterial>)>,
    mut selection_blocker: ResMut<SelectionBlockers>,
    mut visibility: Query<&mut Visibility>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    transforms: Query<&GlobalTransform>,
    intersections: Query<&Intersection<PickingRaycastSet>>,
    cursor: Res<Cursor>,
    mut picks: EventReader<ChangePick>,
) {
    let clicked = mouse_button_input.just_pressed(MouseButton::Left)
        || touch_input.iter_just_pressed().next().is_some();

    for pick in picks.iter() {
        if let Some(previous_pick) = pick.from {
            if let Ok((drag, mut material)) = draggables.get_mut(previous_pick) {
                if drag.initial.is_none() {
                    *material = drag.materials.passive.clone();
                }
            }
        }

        if let Some(new_pick) = pick.to {
            if let Ok((mut drag, mut material)) = draggables.get_mut(new_pick) {
                if clicked {
                    if let Ok(Some(intersection)) = intersections.get_single().map(|i| i.position()) {
                        if let Ok(tf) = transforms.get(drag.for_entity) {
                            selection_blocker.dragging = true;
                            drag.initial = Some(InitialDragConditions{
                                click_point: intersection.clone(),
                                entity_tf: tf.compute_transform(),
                            });
                            *material = drag.materials.drag.clone();
                        }
                    }
                } else {
                    if drag.initial.is_none() {
                        set_visibility(cursor.frame, &mut visibility, false);
                        *material = drag.materials.drag.clone();
                    }
                }
            }
        }
    }
}

pub fn update_drag_release(
    mut draggables: Query<(&mut Draggable, &mut Handle<StandardMaterial>)>,
    mut selection_blockers: ResMut<SelectionBlockers>,
    mouse_button_input: Res<Input<MouseButton>>,
) {
    if mouse_button_input.just_released(MouseButton::Left) {
        for (mut draggable, mut material) in &mut draggables {
            if draggable.initial.is_some() {
                draggable.initial = None;
                *material = draggable.materials.passive.clone();
            }
        }

        selection_blockers.dragging = false;
    }
}

pub fn update_drag_motions(
    drag_axis: Query<(&DragAxis, &Draggable, &GlobalTransform), Without<DragPlane>>,
    drag_plane: Query<(&DragPlane, &Draggable, &GlobalTransform), Without<DragAxis>>,
    transforms: Query<(&Transform, &GlobalTransform), Without<Draggable>>,
    cameras: Query<&Camera>,
    camera_controls: Query<&CameraControls>,
    mut cursor_motion: EventReader<CursorMoved>,
    mut move_to: EventWriter<MoveTo>,
) {
    let cursor_position = match cursor_motion.iter().last() {
        Some(m) => m.position,
        None => { return; }
    };

    let active_camera = camera_controls.single().active_camera();
    let ray = if let Some(camera) = cameras.get(active_camera).ok() {
        let camera_tf = match transforms.get(active_camera).ok() {
            Some(tf) => tf.1.clone(),
            None => { return; }
        };

        match Ray3d::from_screenspace(cursor_position, camera, &camera_tf) {
            Some(ray) => ray,
            None => { return; }
        }
    } else {
        return;
    };

    for (axis, draggable, drag_tf) in &drag_axis {
        if let Some(initial) = &draggable.initial {
            if let Some((for_local_tf, for_global_tf)) = transforms.get(draggable.for_entity).ok() {
                let n = drag_tf.affine().transform_vector3(axis.along).normalize_or_zero();
                let dp = ray.origin() - initial.click_point;
                let a = ray.direction().dot(n);
                let b = ray.direction().dot(dp);
                let c = n.dot(dp);

                let denom = a.powi(2) - 1.;
                if denom.abs() < 1e-3 {
                    // The rays are nearly parallel, so we should not attempt moving
                    // because the motion will be too extreme
                    continue;
                }

                let t = (a*b - c)/denom;
                let delta = t*n;
                let tf_goal = initial.entity_tf.with_translation(initial.entity_tf.translation + delta);
                let tf_parent_inv = for_local_tf.compute_affine() * for_global_tf.affine().inverse();
                move_to.send(MoveTo{
                    entity: draggable.for_entity,
                    transform: Transform::from_matrix((tf_parent_inv * tf_goal.compute_affine()).into()),
                });
            }
        }
    }

    for (plane, draggable, drag_tf) in &drag_plane {
        if let Some(initial) = &draggable.initial {
            if let Some((for_local_tf, for_global_tf)) = transforms.get(draggable.for_entity).ok() {
                let n_p = drag_tf.affine().transform_vector3(plane.in_plane).normalize_or_zero();
                let n_r = ray.direction();
                let denom = n_p.dot(n_r);
                if denom.abs() < 1e-3 {
                    // The rays are nearly parallel so we should not attempt moving
                    // because the motion will be too extreme
                    continue;
                }

                let t = (initial.click_point - ray.origin()).dot(n_p)/denom;
                let delta = ray.position(t) - initial.click_point;
                let tf_goal = initial.entity_tf.with_translation(initial.entity_tf.translation + delta);
                let tf_parent_inv = for_local_tf.compute_affine() * for_global_tf.affine().inverse();
                move_to.send(MoveTo{
                    entity: draggable.for_entity,
                    transform: Transform::from_matrix((tf_parent_inv * tf_goal.compute_affine()).into())
                });
            }
        }
    }
}
