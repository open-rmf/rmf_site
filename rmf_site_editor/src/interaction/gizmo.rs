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
use bevy_mod_picking::{PickableBundle, PickingRaycastSet};
use bevy_mod_raycast::{Intersection, Ray3d};
use rmf_site_format::Pose;

#[derive(Debug, Clone, Copy)]
pub struct InitialDragConditions {
    click_point: Vec3,
    entity_tf: Transform,
}

#[derive(Debug, Clone)]
pub struct GizmoMaterialSet {
    pub passive: Handle<StandardMaterial>,
    pub hover: Handle<StandardMaterial>,
    pub drag: Handle<StandardMaterial>,
}

impl GizmoMaterialSet {
    pub fn make_x_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::rgb(1., 0., 0.).into()),
            hover: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
            drag: materials.add(Color::rgb(0.7, 0., 0.).into()),
        }
    }

    pub fn make_y_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::rgb(0., 0.9, 0.).into()),
            hover: materials.add(Color::rgb(0.5, 1.0, 0.5).into()),
            drag: materials.add(Color::rgb(0., 0.6, 0.).into()),
        }
    }

    pub fn make_z_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::rgb(0., 0., 0.9).into()),
            hover: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
            drag: materials.add(Color::rgb(0., 0., 0.6).into()),
        }
    }

    pub fn make_z_plane(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::rgba(0., 0., 1., 0.6).into()),
            hover: materials.add(Color::rgba(0.3, 0.3, 1., 0.6).into()),
            drag: materials.add(Color::rgba(0., 0., 0.7, 0.9).into()),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Gizmo {
    /// If the material of the draggable entity should change when interacted
    /// with, this field can be given the desired material set.
    pub materials: Option<GizmoMaterialSet>,
}

impl Gizmo {
    pub fn new() -> Self {
        Self { materials: None }
    }

    pub fn with_materials(mut self, materials: GizmoMaterialSet) -> Self {
        self.materials = Some(materials);
        self
    }
}

#[derive(Component, Debug, Clone)]
pub struct Draggable {
    pub for_entity: Entity,
    pub drag: Option<InitialDragConditions>,
}

impl Draggable {
    pub fn new(for_entity: Entity) -> Self {
        Self {
            for_entity,
            drag: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GizmoClicked(pub Entity);

#[derive(Clone, Copy, Debug)]
pub enum FrameOfReference {
    /// Use a local frame of reference for the drag constraints
    Local,
    /// Use a global frame of reference for the drag constraints
    Global,
}

impl FrameOfReference {
    pub fn is_local(&self) -> bool {
        matches!(self, FrameOfReference::Local)
    }
    pub fn is_global(&self) -> bool {
        matches!(self, FrameOfReference::Global)
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DragAxis {
    /// The gizmo can only be dragged along this axis
    pub along: Vec3,
    /// The axis is in this frame of reference
    pub frame: FrameOfReference,
}

#[derive(Bundle)]
pub struct DragAxisBundle {
    pub gizmo: Gizmo,
    pub draggable: Draggable,
    pub axis: DragAxis,
}

impl DragAxisBundle {
    pub fn new(for_entity: Entity, along: Vec3) -> Self {
        Self {
            gizmo: Gizmo::new(),
            draggable: Draggable::new(for_entity),
            axis: DragAxis {
                along,
                frame: FrameOfReference::Local,
            },
        }
    }

    pub fn with_materials(mut self, materials: GizmoMaterialSet) -> Self {
        self.gizmo = self.gizmo.with_materials(materials);
        self
    }

    pub fn globally(mut self) -> Self {
        self.axis.frame = FrameOfReference::Global;
        self
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DragPlane {
    /// The gizmo can only be dragged in the plane orthogonal to this vector
    pub in_plane: Vec3,
    /// The vector is in this frame of reference
    pub frame: FrameOfReference,
}

#[derive(Bundle)]
pub struct DragPlaneBundle {
    pub gizmo: Gizmo,
    pub draggable: Draggable,
    pub plane: DragPlane,
}

impl DragPlaneBundle {
    pub fn new(for_entity: Entity, in_plane: Vec3) -> Self {
        Self {
            gizmo: Gizmo::new(),
            draggable: Draggable::new(for_entity),
            plane: DragPlane {
                in_plane,
                frame: FrameOfReference::Local,
            },
        }
    }

    pub fn with_materials(mut self, materials: GizmoMaterialSet) -> Self {
        self.gizmo = self.gizmo.with_materials(materials);
        self
    }

    pub fn globally(mut self) -> Self {
        self.plane.frame = FrameOfReference::Global;
        self
    }
}

/// Used as a resource to keep track of which draggable is currently hovered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum GizmoState {
    Dragging(Entity),
    Hovering(Entity),
    None,
}

impl GizmoState {
    pub fn is_dragging(&self) -> bool {
        return matches!(self, GizmoState::Dragging(_));
    }
}

impl Default for GizmoState {
    fn default() -> Self {
        GizmoState::None
    }
}

/// Instruction to move an entity to a new transform. This should be caught with
/// an EventReader<MoveTo>.
#[derive(Debug, Clone, Copy)]
pub struct MoveTo {
    pub entity: Entity,
    pub transform: Transform,
}

pub fn make_gizmos_pickable(mut commands: Commands, new_gizmos: Query<Entity, Added<Gizmo>>) {
    for e in &new_gizmos {
        commands.entity(e).insert(PickableBundle::default());
    }
}

pub fn update_gizmo_click_start(
    mut gizmos: Query<(
        &Gizmo,
        Option<&mut Draggable>,
        &mut Handle<StandardMaterial>,
    )>,
    mut selection_blocker: ResMut<SelectionBlockers>,
    mut visibility: Query<&mut Visibility>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    transforms: Query<&GlobalTransform>,
    intersections: Query<&Intersection<PickingRaycastSet>>,
    mut cursor: ResMut<Cursor>,
    mut gizmo_state: ResMut<GizmoState>,
    mut picks: EventReader<ChangePick>,
    mut click: EventWriter<GizmoClicked>,
    removed_gizmos: RemovedComponents<Gizmo>,
) {
    for e in removed_gizmos.iter() {
        cursor.remove_blocker(e, &mut visibility);
    }

    for pick in picks.iter() {
        if let Some(previous_pick) = pick.from {
            cursor.remove_blocker(previous_pick, &mut visibility);
            if *gizmo_state == GizmoState::Hovering(previous_pick) {
                if let Ok((gizmo, _, mut material)) = gizmos.get_mut(previous_pick) {
                    if let Some(gizmo_materials) = &gizmo.materials {
                        *material = gizmo_materials.passive.clone();
                    }
                }

                *gizmo_state = GizmoState::None;
            }
        }

        if !gizmo_state.is_dragging() {
            if let Some(new_pick) = pick.to {
                if let Ok((gizmo, _, mut material)) = gizmos.get_mut(new_pick) {
                    cursor.add_blocker(new_pick, &mut visibility);
                    if let Some(gizmo_materials) = &gizmo.materials {
                        *material = gizmo_materials.hover.clone();
                    }

                    *gizmo_state = GizmoState::Hovering(new_pick);
                }
            }
        }
    }

    let clicked = mouse_button_input.just_pressed(MouseButton::Left)
        || touch_input.iter_just_pressed().next().is_some();

    if clicked {
        if let GizmoState::Hovering(e) = *gizmo_state {
            click.send(GizmoClicked(e));
            if let Ok(Some(intersection)) = intersections.get_single().map(|i| i.position()) {
                if let Ok((gizmo, Some(mut draggable), mut material)) = gizmos.get_mut(e) {
                    if let Ok(tf) = transforms.get(draggable.for_entity) {
                        selection_blocker.dragging = true;
                        draggable.drag = Some(InitialDragConditions {
                            click_point: intersection.clone(),
                            entity_tf: tf.compute_transform(),
                        });
                        if let Some(drag_materials) = &gizmo.materials {
                            *material = drag_materials.drag.clone();
                        }
                        *gizmo_state = GizmoState::Dragging(e);
                    } else {
                        *gizmo_state = GizmoState::None;
                    }
                } else {
                    // The hovered draggable is no longer draggable, so change the
                    // drag state to none
                    *gizmo_state = GizmoState::None;
                }
            }
        }
    }
}

pub fn update_gizmo_release(
    mut draggables: Query<(&Gizmo, &mut Draggable, &mut Handle<StandardMaterial>)>,
    mut selection_blockers: ResMut<SelectionBlockers>,
    mut gizmo_state: ResMut<GizmoState>,
    mouse_button_input: Res<Input<MouseButton>>,
    picked: Res<Picked>,
    mut change_pick: EventWriter<ChangePick>,
) {
    if mouse_button_input.just_released(MouseButton::Left) {
        if let GizmoState::Dragging(e) = *gizmo_state {
            if let Ok((gizmo, mut draggable, mut material)) = draggables.get_mut(e) {
                draggable.drag = None;
                if let Some(gizmo_materials) = &gizmo.materials {
                    *material = gizmo_materials.passive.clone();
                }
            }

            *gizmo_state = GizmoState::None;
            selection_blockers.dragging = false;
            // Refresh the latest pick since some pick responders were blocked
            // during the dragging activity. Without this event, users will have
            // to move the cursor off of whatever object it happens to be
            // hovering over after the drag is finished before interactions like
            // selecting or dragging can resume.
            change_pick.send(ChangePick {
                from: None,
                to: picked.0,
            });
        }
    }
}

pub fn update_drag_motions(
    drag_axis: Query<(&DragAxis, &Draggable, &GlobalTransform), Without<DragPlane>>,
    drag_plane: Query<(&DragPlane, &Draggable, &GlobalTransform), Without<DragAxis>>,
    transforms: Query<(&Transform, &GlobalTransform)>,
    cameras: Query<&Camera>,
    camera_controls: Res<CameraControls>,
    drag_state: Res<GizmoState>,
    mut cursor_motion: EventReader<CursorMoved>,
    mut move_to: EventWriter<MoveTo>,
) {
    if let GizmoState::Dragging(dragging) = *drag_state {
        let cursor_position = match cursor_motion.iter().last() {
            Some(m) => m.position,
            None => {
                return;
            }
        };

        let active_camera = camera_controls.active_camera();
        let ray = if let Some(camera) = cameras.get(active_camera).ok() {
            let camera_tf = match transforms.get(active_camera).ok() {
                Some(tf) => tf.1.clone(),
                None => {
                    return;
                }
            };

            match Ray3d::from_screenspace(cursor_position, camera, &camera_tf) {
                Some(ray) => ray,
                None => {
                    return;
                }
            }
        } else {
            return;
        };

        if let Ok((axis, draggable, drag_tf)) = drag_axis.get(dragging) {
            if let Some(initial) = &draggable.drag {
                if let Some((for_local_tf, for_global_tf)) =
                    transforms.get(draggable.for_entity).ok()
                {
                    let n = if axis.frame.is_local() {
                        drag_tf
                            .affine()
                            .transform_vector3(axis.along)
                            .normalize_or_zero()
                    } else {
                        axis.along.normalize_or_zero()
                    };
                    let dp = ray.origin() - initial.click_point;
                    let a = ray.direction().dot(n);
                    let b = ray.direction().dot(dp);
                    let c = n.dot(dp);

                    let denom = a.powi(2) - 1.;
                    if denom.abs() < 1e-3 {
                        // The rays are nearly parallel, so we should not attempt moving
                        // because the motion will be too extreme
                        return;
                    }

                    let t = (a * b - c) / denom;
                    let delta = t * n;
                    let tf_goal = initial
                        .entity_tf
                        .with_translation(initial.entity_tf.translation + delta);
                    let tf_parent_inv =
                        for_local_tf.compute_affine() * for_global_tf.affine().inverse();
                    move_to.send(MoveTo {
                        entity: draggable.for_entity,
                        transform: Transform::from_matrix(
                            (tf_parent_inv * tf_goal.compute_affine()).into(),
                        ),
                    });
                }
            }
        }

        if let Ok((plane, draggable, drag_tf)) = drag_plane.get(dragging) {
            if let Some(initial) = &draggable.drag {
                if let Some((for_local_tf, for_global_tf)) =
                    transforms.get(draggable.for_entity).ok()
                {
                    let n_p = if plane.frame.is_local() {
                        drag_tf
                            .affine()
                            .transform_vector3(plane.in_plane)
                            .normalize_or_zero()
                    } else {
                        plane.in_plane.normalize_or_zero()
                    };

                    let n_r = ray.direction();
                    let denom = n_p.dot(n_r);
                    if denom.abs() < 1e-3 {
                        // The rays are nearly parallel so we should not attempt
                        // moving because the motion will be too extreme
                        return;
                    }

                    let t = (initial.click_point - ray.origin()).dot(n_p) / denom;
                    let delta = ray.position(t) - initial.click_point;
                    let tf_goal = initial
                        .entity_tf
                        .with_translation(initial.entity_tf.translation + delta);
                    let tf_parent_inv =
                        for_local_tf.compute_affine() * for_global_tf.affine().inverse();
                    move_to.send(MoveTo {
                        entity: draggable.for_entity,
                        transform: Transform::from_matrix(
                            (tf_parent_inv * tf_goal.compute_affine()).into(),
                        ),
                    });
                }
            }
        }
    }
}

pub fn move_pose(mut poses: Query<&mut Pose>, mut move_to: EventReader<MoveTo>) {
    for move_to in move_to.iter() {
        if let Ok(mut pose) = poses.get_mut(move_to.entity) {
            dbg!(&move_to.transform);
            pose.align_with(&move_to.transform);
            dbg!(&pose);
        }
    }
}
