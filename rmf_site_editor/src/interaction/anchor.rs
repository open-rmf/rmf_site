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
    interaction::IntersectGroundPlaneParams,
    interaction::*,
    keyboard::DebugMode,
    site::{Anchor, Category, Delete, Dependents, SiteAssets, Subordinate},
};
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct AnchorVisualization {
    pub body: Entity,
    pub drag: Option<Entity>,
}

fn make_anchor_orientation_cue_meshes(
    commands: &mut Commands,
    interaction_assets: &InteractionAssets,
    parent: Entity,
    transform: Transform,
){
    // The arrows should originate in the mesh origin
    let pos = Vec3::splat(0.0);
    let rot = transform.rotation;
    let rot_x = rot * Quat::from_rotation_y(90_f32.to_radians());
    let rot_y = rot * Quat::from_rotation_x(90_f32.to_radians());
    let rot_z = rot * Quat::from_rotation_z(90_f32.to_radians());
    let x_mat = interaction_assets.x_axis_materials.clone();
    let y_mat = interaction_assets.y_axis_materials.clone();
    let z_mat = interaction_assets.z_axis_materials.clone();
    interaction_assets.make_axis(commands, None, parent, x_mat, pos, rot_x, 1.0);
    interaction_assets.make_axis(commands, None, parent, y_mat, pos, rot_y, 1.0);
    interaction_assets.make_axis(commands, None, parent, z_mat, pos, rot_z, 1.0);
}

pub fn add_anchor_visual_cues(
    mut commands: Commands,
    new_anchors: Query<(Entity, &Parent, Option<&Subordinate>, &Anchor), (Added<Anchor>, Without<Preview>)>,
    categories: Query<&Category>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    for (e, parent, subordinate, anchor) in &new_anchors {
        let body_mesh = match categories.get(parent.get()).unwrap() {
            Category::Level => site_assets.level_anchor_mesh.clone(),
            Category::Lift => site_assets.lift_anchor_mesh.clone(),
            _ => site_assets.site_anchor_mesh.clone(),
        };

        if let Anchor::Pose3D(pose) = anchor {
            make_anchor_orientation_cue_meshes(&mut commands, &interaction_assets, e, pose.transform());
        }

        let mut commands = commands.entity(e);
        let body = commands.add_children(|parent| {
            let mut body = parent.spawn_bundle(PbrBundle {
                mesh: body_mesh,
                material: site_assets.passive_anchor_material.clone(),
                ..default()
            });
            body.insert(Selectable::new(e));
            if subordinate.is_none() {
                body.insert_bundle(DragPlaneBundle::new(e, Vec3::Z));
            }
            let body = body.id();

            body
        });

        commands
            .insert(AnchorVisualization { body, drag: None })
            .insert(OutlineVisualization::Anchor);

        // 3D anchors should always be visible
        match anchor {
            Anchor::Pose3D(_) => {},
            _ => { commands.insert(VisualCue::outline().irregular()); }
        }
    }
}

pub fn remove_interaction_for_subordinate_anchors(
    mut commands: Commands,
    new_subordinates: Query<&Children, (With<Anchor>, Added<Subordinate>)>,
) {
    for children in &new_subordinates {
        for child in children {
            commands
                .entity(*child)
                .remove::<Gizmo>()
                .remove::<Draggable>()
                .remove::<DragPlane>();
        }
    }
}

pub fn move_anchor(
    mut anchors: Query<&mut Anchor, Without<Subordinate>>,
    mut move_to: EventReader<MoveTo>,
) {
    for move_to in move_to.iter() {
        if let Ok(mut anchor) = anchors.get_mut(move_to.entity) {
            anchor.move_to(&move_to.transform);
        }
    }
}

pub fn update_anchor_proximity_xray(
    mut anchors: Query<(&GlobalTransform, &mut VisualCue), With<Anchor>>,
    intersect_ground_params: IntersectGroundPlaneParams,
    cursor_moved: EventReader<CursorMoved>,
) {
    if cursor_moved.is_empty() {
        return;
    }

    let p_c = match intersect_ground_params.ground_plane_intersection() {
        Some(p) => p,
        None => return,
    };

    for (anchor_tf, mut cue) in &mut anchors {
        // TODO(MXG): Make the proximity range configurable
        let proximity = {
            // We make the xray effect a little "sticky" so that there isn't an
            // ugly flicker for anchors that are right at the edge of the
            // proximity range.
            if cue.xray.any() {
                1.0
            } else {
                0.2
            }
        };

        let xray = 'xray: {
            let p_a = anchor_tf.translation();
            if p_a.x < p_c.x - proximity || p_c.x + proximity < p_a.x {
                break 'xray false;
            }

            if p_a.y < p_c.y - proximity || p_c.y + proximity < p_a.y {
                break 'xray false;
            }

            true
        };

        if xray != cue.xray.proximity() {
            cue.xray.set_proximity(xray);
        }
    }
}

pub fn update_unassigned_anchor_cues(
    mut anchors: Query<(&Dependents, &mut VisualCue), (With<Anchor>, Changed<Dependents>)>,
) {
    for (deps, mut cue) in &mut anchors {
        if deps.is_empty() != cue.xray.unassigned() {
            cue.xray.set_unassigned(deps.is_empty())
        }
    }
}

pub fn update_anchor_cues_for_mode(
    mode: Res<InteractionMode>,
    mut anchors: Query<&mut VisualCue, With<Anchor>>,
) {
    if !mode.is_changed() {
        return;
    }

    let anchor_always_visible = mode.is_selecting_anchor();
    for mut cue in &mut anchors {
        if cue.xray.always() != anchor_always_visible {
            cue.xray.set_always(anchor_always_visible);
        }
    }
}

pub fn update_anchor_visual_cues(
    mut command: Commands,
    mut anchors: Query<
        (
            Entity,
            &Hovered,
            &Selected,
            &mut AnchorVisualization,
            &mut VisualCue,
            Option<&Subordinate>,
            ChangeTrackers<Selected>,
        ),
        Or<(Changed<Hovered>, Changed<Selected>, Changed<Dependents>)>,
    >,
    mut bobbing: Query<&mut Bobbing>,
    mut visibility: Query<&mut Visibility>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    deps: Query<&Dependents>,
    cursor: Res<Cursor>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
    debug_mode: Option<Res<DebugMode>>,
) {
    for (a, hovered, selected, mut shapes, mut cue, subordinate, select_tracker) in &mut anchors {
        if debug_mode.as_ref().filter(|d| d.0).is_some() {
            // NOTE(MXG): I have witnessed a scenario where a lane is deleted
            // and then the anchors that supported it are permanently stuck as
            // though they are selected. I have not figured out what can cause
            // that, so I am keeping this printout available to debug that
            // scenario. Press the D key to activate this.
            dbg!((a, hovered, selected));
        }

        if cue.xray.selected() != selected.is_selected {
            cue.xray.set_selected(selected.is_selected)
        }

        if cue.xray.support_selected() != !selected.support_selected.is_empty() {
            cue.xray
                .set_support_selected(!selected.support_selected.is_empty())
        }

        if cue.xray.hovered() != hovered.is_hovered {
            cue.xray.set_hovered(hovered.is_hovered);
        }

        if cue.xray.support_hovered() != !hovered.support_hovering.is_empty() {
            cue.xray
                .set_support_hovered(!hovered.support_hovering.is_empty());
        }

        if hovered.is_hovered {
            set_visibility(cursor.frame, &mut visibility, false);
        }

        if hovered.cue() && selected.cue() {
            set_material(
                shapes.body,
                &site_assets.hover_select_anchor_material,
                &mut materials,
            );
        } else if hovered.cue() {
            // Hovering but not selected
            set_material(
                shapes.body,
                &site_assets.hover_anchor_material,
                &mut materials,
            );
        } else if selected.cue() {
            // Selected but not hovering
            set_material(
                shapes.body,
                &site_assets.select_anchor_material,
                &mut materials,
            );
        } else {
            set_material(
                shapes.body,
                site_assets.decide_passive_anchor_material(a, &deps),
                &mut materials,
            );
        }

        if select_tracker.is_changed() {
            if selected.cue() {
                if shapes.drag.is_none() && subordinate.is_none() {
                    interaction_assets.add_anchor_draggable_arrows(
                        &mut command,
                        a,
                        shapes.as_mut(),
                    );
                }
            } else {
                if let Some(drag) = shapes.drag {
                    command.entity(drag).despawn_recursive();
                }
                shapes.drag = None;
            }
        }
    }
}

// NOTE(MXG): Currently only anchors ever have support cues, so we filter down
// to entities with AnchorVisualCues. We will need to broaden that if any other
// visual cue types ever have a supporting role.
pub fn remove_deleted_supports_from_visual_cues(
    mut hovered: Query<&mut Hovered, With<AnchorVisualization>>,
    mut selected: Query<&mut Selected, With<AnchorVisualization>>,
    mut deleted_elements: EventReader<Delete>,
) {
    for deletion in deleted_elements.iter() {
        for mut h in &mut hovered {
            h.support_hovering.remove(&deletion.element);
        }

        for mut s in &mut selected {
            s.support_selected.remove(&deletion.element);
        }
    }
}
