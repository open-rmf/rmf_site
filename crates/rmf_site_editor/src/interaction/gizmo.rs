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

use crate::{exit_confirmation::SiteChanged, interaction::*};
use bevy::{
    math::Affine3A,
    picking::{
        backend::ray::RayMap,
        pointer::{PointerId, PointerInteraction},
    },
    prelude::*,
};
use rmf_site_camera::{active_camera_maybe, ActiveCameraQuery};
use rmf_site_format::Pose;

#[derive(Debug, Clone, Copy)]
pub struct InitialDragConditions {
    click_point: Vec3,
    tf_for_entity_global: Transform,
    tf_for_entity_parent_inv: Affine3A,
}

#[derive(Debug, Clone)]
pub struct GizmoMaterialSet {
    pub passive: Handle<StandardMaterial>,
    pub hover: Handle<StandardMaterial>,
    pub drag: Handle<StandardMaterial>,
}

#[derive(Resource)]
pub struct GizmoBlockers {
    pub selecting: bool,
}

impl GizmoBlockers {
    pub fn blocking(&self) -> bool {
        self.selecting
    }
}

impl Default for GizmoBlockers {
    fn default() -> Self {
        Self { selecting: false }
    }
}

impl GizmoMaterialSet {
    pub fn make_x_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::srgb(1., 0., 0.)),
            hover: materials.add(Color::srgb(1.0, 0.3, 0.3)),
            drag: materials.add(Color::srgb(0.7, 0., 0.)),
        }
    }

    pub fn make_y_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::srgb(0., 0.9, 0.)),
            hover: materials.add(Color::srgb(0.5, 1.0, 0.5)),
            drag: materials.add(Color::srgb(0., 0.6, 0.)),
        }
    }

    pub fn make_z_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::srgb(0., 0., 0.9)),
            hover: materials.add(Color::srgb(0.5, 0.5, 1.0)),
            drag: materials.add(Color::srgb(0., 0., 0.6)),
        }
    }

    pub fn make_z_plane(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self {
            passive: materials.add(Color::srgba(0., 0., 1., 0.6)),
            hover: materials.add(Color::srgba(0.3, 0.3, 1., 0.6)),
            drag: materials.add(Color::srgba(0., 0., 0.7, 0.9)),
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

#[derive(Debug, Clone, Copy, Event)]
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
    pub selectable: Selectable,
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
            selectable: Selectable {
                is_selectable: true,
                element: for_entity,
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
    pub selectable: Selectable,
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
            selectable: Selectable {
                is_selectable: true,
                element: for_entity,
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
/// an `EventReader<MoveTo>`.
#[derive(Debug, Clone, Copy, Event)]
pub struct MoveTo {
    pub entity: Entity,
    pub transform: Transform,
}

pub fn update_gizmo_click_start(
    mut gizmos: Query<(
        &Gizmo,
        Option<&mut Draggable>,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut selection_blocker: ResMut<SelectionBlockers>,
    gizmo_blocker: Res<GizmoBlockers>,
    mut visibility: Query<&mut Visibility>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    transforms: Query<(&Transform, &GlobalTransform)>,
    pointers: Query<(&PointerId, &PointerInteraction)>,
    mut cursor: ResMut<Cursor>,
    mut gizmo_state: ResMut<GizmoState>,
    mut picks: EventReader<ChangePick>,
    mut click: EventWriter<GizmoClicked>,
    mut removed_gizmos: RemovedComponents<Gizmo>,
) {
    if gizmo_blocker.blocking() {
        if gizmo_blocker.is_changed() {
            // This has started being blocked since the last cycle
            cursor.clear_blockers(&mut visibility);
        }

        // Don't start any gizmos
        return;
    }

    for e in removed_gizmos.read() {
        cursor.remove_blocker(e, &mut visibility);
    }

    for pick in picks.read() {
        if let Some(previous_pick) = pick.from {
            cursor.remove_blocker(previous_pick, &mut visibility);
            if *gizmo_state == GizmoState::Hovering(previous_pick) {
                if let Ok((gizmo, _, mut material)) = gizmos.get_mut(previous_pick) {
                    if let Some(gizmo_materials) = &gizmo.materials {
                        *material = MeshMaterial3d(gizmo_materials.passive.clone());
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
                        *material = MeshMaterial3d(gizmo_materials.hover.clone());
                    }

                    *gizmo_state = GizmoState::Hovering(new_pick);
                }
            }
        }
    }

    // Ignore if button was pressed and released in the same frame, to avoid being stuck in
    // dragging behavior in cases when the frame rate is low.
    let clicking = mouse_button_input.just_pressed(MouseButton::Left)
        && !mouse_button_input.just_released(MouseButton::Left);

    if clicking {
        if let GizmoState::Hovering(e) = *gizmo_state {
            click.write(GizmoClicked(e));
            let Some((_, interactions)) = pointers.single().ok().filter(|(id, _)| id.is_mouse())
            else {
                return;
            };
            if let Some(intersection) = interactions
                .get_nearest_hit()
                .and_then(|(_, hit_data)| hit_data.position)
            {
                if let Ok((gizmo, Some(mut draggable), mut material)) = gizmos.get_mut(e) {
                    if let Ok((local_tf, global_tf)) = transforms.get(draggable.for_entity) {
                        selection_blocker.dragging = true;
                        let tf_for_entity_parent_inv =
                            local_tf.compute_affine() * global_tf.affine().inverse();
                        draggable.drag = Some(InitialDragConditions {
                            click_point: intersection.clone(),
                            tf_for_entity_global: global_tf.compute_transform(),
                            tf_for_entity_parent_inv,
                        });
                        if let Some(drag_materials) = &gizmo.materials {
                            *material = MeshMaterial3d(drag_materials.drag.clone());
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
    mut draggables: Query<(
        &Gizmo,
        &mut Draggable,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut selection_blockers: ResMut<SelectionBlockers>,
    gizmo_blockers: Res<GizmoBlockers>,
    mut gizmo_state: ResMut<GizmoState>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut picked: ResMut<Picked>,
) {
    let mouse_released = mouse_button_input.just_released(MouseButton::Left);
    let gizmos_blocked = gizmo_blockers.blocking();
    if mouse_released || gizmos_blocked {
        if let GizmoState::Dragging(e) = *gizmo_state {
            if let Ok((gizmo, mut draggable, mut material)) = draggables.get_mut(e) {
                draggable.drag = None;
                if let Some(gizmo_materials) = &gizmo.materials {
                    *material = MeshMaterial3d(gizmo_materials.passive.clone());
                }
            }

            *gizmo_state = GizmoState::None;
            selection_blockers.dragging = false;
            // Refresh the latest pick since some pick responders were blocked
            // during the dragging activity. Without this event, users will have
            // to move the cursor off of whatever object it happens to be
            // hovering over after the drag is finished before interactions like
            // selecting or dragging can resume.
            picked.refresh = true;
        }
    }
}

pub fn update_drag_motions(
    drag_axis: Query<(&DragAxis, &Draggable, &GlobalTransform), Without<DragPlane>>,
    drag_plane: Query<(&DragPlane, &Draggable, &GlobalTransform), Without<DragAxis>>,
    drag_state: Res<GizmoState>,
    active_camera: ActiveCameraQuery,
    mut cursor_motion: EventReader<CursorMoved>,
    mut move_to: EventWriter<MoveTo>,
    ray_map: Res<RayMap>,
) {
    if let GizmoState::Dragging(dragging) = *drag_state {
        let _cursor_position = match cursor_motion.read().last() {
            Some(m) => m.position,
            None => {
                return;
            }
        };

        let Ok(active_camera) = active_camera_maybe(&active_camera) else {
            return;
        };

        let Some((_, ray)) = ray_map.iter().find(|(id, _)| id.camera == active_camera) else {
            return;
        };

        if let Ok((axis, draggable, drag_tf)) = drag_axis.get(dragging) {
            if let Some(initial) = &draggable.drag {
                let n = if axis.frame.is_local() {
                    drag_tf
                        .affine()
                        .transform_vector3(axis.along)
                        .normalize_or_zero()
                } else {
                    axis.along.normalize_or_zero()
                };
                let dp = ray.origin - initial.click_point;
                let a = ray.direction.dot(n);
                let b = ray.direction.dot(dp);
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
                    .tf_for_entity_global
                    .with_translation(initial.tf_for_entity_global.translation + delta);
                move_to.write(MoveTo {
                    entity: draggable.for_entity,
                    transform: Transform::from_matrix(
                        (initial.tf_for_entity_parent_inv * tf_goal.compute_affine()).into(),
                    ),
                });
            }
        }

        if let Ok((plane, draggable, drag_tf)) = drag_plane.get(dragging) {
            if let Some(initial) = &draggable.drag {
                let n_p = if plane.frame.is_local() {
                    drag_tf
                        .affine()
                        .transform_vector3(plane.in_plane)
                        .normalize_or_zero()
                } else {
                    plane.in_plane.normalize_or_zero()
                };

                let n_r = ray.direction;
                let denom = n_p.dot(*n_r);
                if denom.abs() < 1e-3 {
                    // The rays are nearly parallel so we should not attempt
                    // moving because the motion will be too extreme
                    return;
                }

                let t = (initial.click_point - ray.origin).dot(n_p) / denom;
                let delta = ray.get_point(t) - initial.click_point;
                let tf_goal = initial
                    .tf_for_entity_global
                    .with_translation(initial.tf_for_entity_global.translation + delta);
                move_to.write(MoveTo {
                    entity: draggable.for_entity,
                    transform: Transform::from_matrix(
                        (initial.tf_for_entity_parent_inv * tf_goal.compute_affine()).into(),
                    ),
                });
            }
        }
    }
}

pub fn move_pose(
    mut poses: Query<&mut Pose>,
    mut move_to: EventReader<MoveTo>,
    mut site_changed: ResMut<SiteChanged>,
) {
    for move_to in move_to.read() {
        if let Ok(mut pose) = poses.get_mut(move_to.entity) {
            site_changed.0 = true;
            pose.align_with(&move_to.transform);
        }
    }
}
