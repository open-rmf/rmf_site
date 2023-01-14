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
    site::{Anchor, Category, Delete, SiteAssets, Subordinate},
};
use bevy::prelude::*;

/// xray an anchor because it is selected
const XRAY_SELECTED: u32 = 0;
/// xray an anchor because it is within proximity of the cursor
const XRAY_PROXIMITY: u32 = 1;

#[derive(Component, Debug, Clone, Copy)]
pub struct AnchorVisualization {
    pub dagger: Entity,
    pub halo: Entity,
    pub body: Entity,
    pub drag: Option<Entity>,
}

pub fn add_anchor_visual_cues(
    mut commands: Commands,
    new_anchors: Query<(Entity, &Parent, Option<&Subordinate>), (Added<Anchor>, Without<Preview>)>,
    categories: Query<&Category>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    for (e, parent, subordinate) in &new_anchors {
        let body_mesh = match categories.get(parent.get()).unwrap() {
            Category::Level => site_assets.level_anchor_mesh.clone(),
            Category::Lift => site_assets.lift_anchor_mesh.clone(),
            _ => site_assets.site_anchor_mesh.clone(),
        };

        let mut commands = commands.entity(e);
        let (dagger, halo, body) = commands.add_children(|parent| {
            let dagger = parent
                .spawn_bundle(PbrBundle {
                    material: interaction_assets.dagger_material.clone(),
                    mesh: interaction_assets.dagger_mesh.clone(),
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .insert(Bobbing::default())
                .insert(Spinning::default())
                .id();

            let halo = parent
                .spawn_bundle(PbrBundle {
                    material: interaction_assets.halo_material.clone(),
                    mesh: interaction_assets.halo_mesh.clone(),
                    transform: Transform::from_scale([0.2, 0.2, 1.].into()),
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .insert(Spinning::default())
                .id();

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

            (dagger, halo, body)
        });

        commands
            .insert(AnchorVisualization {
                dagger,
                halo,
                body,
                drag: None,
            })
            .insert(VisualCue::no_outline().irregular());
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
    cursor: Res<Cursor>,
    transforms: Query<&GlobalTransform>,
    cursor_moved: EventReader<CursorMoved>,
) {
    if cursor_moved.is_empty() {
        return;
    }

    let cursor_tf = match transforms.get(cursor.frame) {
        Ok(tf) => tf,
        _ => return,
    };

    let p_c = cursor_tf.translation();

    for (anchor_tf, mut cue) in &mut anchors {
        let previously_in_proximity = cue.xray_dependents.contains(&XRAY_PROXIMITY);

        // TODO(MXG): Make the proximity range configurable
        let proximity = {
            // We make the xray effect a little "sticky" so that there isn't an
            // ugly flicker for anchors that are right at the edge of the
            // proximity range.
            if previously_in_proximity {
                5.0
            } else {
                4.0
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

        if xray != previously_in_proximity {
            if xray {
                cue.xray_dependents.insert(XRAY_PROXIMITY);
            } else {
                cue.xray_dependents.remove(&XRAY_PROXIMITY);
            }
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
        Or<(Changed<Hovered>, Changed<Selected>)>,
    >,
    mut bobbing: Query<&mut Bobbing>,
    mut visibility: Query<&mut Visibility>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    cursor: Res<Cursor>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    for (a, hovered, selected, mut shapes, mut cue, subordinate, select_tracker) in &mut anchors {
        if hovered.cue() || selected.cue() {
            set_visibility(shapes.dagger, &mut visibility, true);
        }

        if hovered.is_hovered {
            set_visibility(cursor.frame, &mut visibility, false);
        }

        if selected.cue() {
            set_visibility(shapes.halo, &mut visibility, false);
            if !cue.xray_dependents.contains(&XRAY_SELECTED) {
                cue.xray_dependents.insert(XRAY_SELECTED);
            }
        } else {
            if cue.xray_dependents.contains(&XRAY_SELECTED) {
                cue.xray_dependents.remove(&XRAY_SELECTED);
            }
        }

        let anchor_height = 0.15 + 0.05 / 2.;
        if selected.cue() {
            set_bobbing(shapes.dagger, anchor_height, anchor_height, &mut bobbing);
        }

        if hovered.cue() && selected.cue() {
            set_material(shapes.body, &site_assets.hover_select_material, &mut materials);
        } else if hovered.cue() {
            // Hovering but not selected
            set_visibility(shapes.halo, &mut visibility, true);
            set_material(shapes.body, &site_assets.hover_material, &mut materials);
            set_bobbing(shapes.dagger, anchor_height, anchor_height + 0.2, &mut bobbing);
        } else if selected.cue() {
            // Selected but not hovering
            set_material(shapes.body, &site_assets.select_material, &mut materials);
        } else {
            set_material(
                shapes.body,
                &site_assets.passive_anchor_material,
                &mut materials,
            );
            set_visibility(shapes.dagger, &mut visibility, false);
            set_visibility(shapes.halo, &mut visibility, false);
        }

        if select_tracker.is_changed() {
            if selected.cue() {
                if shapes.drag.is_none() && subordinate.is_none() {
                    interaction_assets.add_anchor_draggable_arrows(&mut command, a, shapes.as_mut());
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
