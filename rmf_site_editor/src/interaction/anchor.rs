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
    site::{Anchor, Delete, SiteAssets},
};
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct AnchorVisualCue {
    pub dagger: Entity,
    pub halo: Entity,
    pub body: Entity,
    pub drag: Option<Entity>,
}

pub fn add_anchor_visual_cues(
    mut commands: Commands,
    new_anchors: Query<Entity, (Added<Anchor>, Without<Preview>)>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    for e in &new_anchors {
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

            let body = parent
                .spawn_bundle(PbrBundle {
                    mesh: site_assets.anchor_mesh.clone(),
                    material: site_assets.passive_anchor_material.clone(),
                    transform: Transform::from_rotation(Quat::from_rotation_x(90_f32.to_radians())),
                    ..default()
                })
                .insert(Selectable::new(e))
                .insert(DragPlane {
                    in_plane: Vec3::new(0., 1., 0.),
                })
                .insert(Draggable::new(e, None))
                .id();

            (dagger, halo, body)
        });

        commands.insert(AnchorVisualCue {
            dagger,
            halo,
            body,
            drag: None,
        });
    }
}

pub fn move_anchor(
    mut anchors: Query<&mut Transform, With<Anchor>>,
    mut move_to: EventReader<MoveTo>,
) {
    for move_to in move_to.iter() {
        if let Ok(mut tf) = anchors.get_mut(move_to.entity) {
            tf.translation.x = move_to.transform.translation.x;
            tf.translation.y = move_to.transform.translation.y;
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
            &mut AnchorVisualCue,
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
    for (v, hovered, selected, mut cue, select_tracker) in &mut anchors {
        if hovered.cue() || selected.cue() {
            set_visibility(cue.dagger, &mut visibility, true);
        }

        if hovered.is_hovered {
            set_visibility(cursor.frame, &mut visibility, false);
        }

        if selected.cue() {
            set_visibility(cue.halo, &mut visibility, false);
        }

        let anchor_height = 0.15 + 0.05 / 2.;
        if selected.cue() {
            set_bobbing(cue.dagger, anchor_height, anchor_height, &mut bobbing);
        }

        if hovered.cue() && selected.cue() {
            set_material(cue.body, &site_assets.hover_select_material, &mut materials);
        } else if hovered.cue() {
            // Hovering but not selected
            set_visibility(cue.halo, &mut visibility, true);
            set_material(cue.body, &site_assets.hover_material, &mut materials);
            set_bobbing(cue.dagger, anchor_height, anchor_height + 0.2, &mut bobbing);
        } else if selected.cue() {
            // Selected but not hovering
            set_material(cue.body, &site_assets.select_material, &mut materials);
        } else {
            set_material(
                cue.body,
                &site_assets.passive_anchor_material,
                &mut materials,
            );
            set_visibility(cue.dagger, &mut visibility, false);
            set_visibility(cue.halo, &mut visibility, false);
        }

        if select_tracker.is_changed() {
            if selected.cue() {
                if cue.drag.is_none() {
                    interaction_assets.add_anchor_draggable_arrows(&mut command, v, cue.as_mut());
                }
            } else {
                if let Some(drag) = cue.drag {
                    command.entity(drag).despawn_recursive();
                }
                cue.drag = None;
            }
        }
    }
}

// NOTE(MXG): Currently only anchors ever have support cues, so we filter down
// to entities with AnchorVisualCues. We will need to broaden that if any other
// visual cue types ever have a supporting role.
pub fn remove_deleted_supports_from_visual_cues(
    mut hovered: Query<&mut Hovered, With<AnchorVisualCue>>,
    mut selected: Query<&mut Selected, With<AnchorVisualCue>>,
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
