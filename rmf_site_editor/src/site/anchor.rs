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

use crate::{site::*, Issue, ValidateWorkspace};
use bevy::{prelude::*, render::primitives::Sphere, utils::Uuid};
use itertools::Itertools;
use rmf_site_format::{Anchor, LevelElevation, LiftCabin};
use std::collections::HashMap;

#[derive(Bundle, Debug)]
pub struct AnchorBundle {
    anchor: Anchor,
    transform: Transform,
    global_transform: GlobalTransform,
    dependents: Dependents,
    visibility: Visibility,
    view: ViewVisibility,
    inherited: InheritedVisibility,
    category: Category,
}

impl AnchorBundle {
    pub fn new(anchor: Anchor) -> Self {
        let transform = anchor.local_transform(Category::General);
        Self {
            anchor,
            transform,
            global_transform: transform.into(),
            dependents: Default::default(),
            visibility: Default::default(),
            view: Default::default(),
            inherited: Default::default(),
            category: Category::Anchor,
        }
    }

    pub fn at_transform(tf: &GlobalTransform) -> Self {
        let translation = tf.translation();
        Self::new([translation.x, translation.y].into())
    }

    pub fn visible(self, is_visible: bool) -> Self {
        let visibility = if is_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        Self { visibility, ..self }
    }

    /// When the parent's GlobalTransform is not an identity matrix, this can
    /// be used to make sure the initial GlobalTransform of the anchor entity
    /// is immediately correct. Bevy's builtin transform propagation system will
    /// make sure it is correct after one update cycle, but that could mean that
    /// the anchor and its dependents have the wrong values until that cycle is
    /// finished.
    pub fn parent_transform(self, parent_tf: &GlobalTransform) -> Self {
        Self {
            global_transform: parent_tf.mul_transform(self.transform),
            ..self
        }
    }

    pub fn dependents(self, dependents: Dependents) -> Self {
        Self { dependents, ..self }
    }
}

/// This component is used to indicate that an anchor is controlled by another
/// entity and therefore cannot be interacted with directly by users. Optionally
/// the entity that controls the anchor can be specified so that users can be
/// guided towards how to modify the anchor or understand its purpose.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Subordinate(pub Option<Entity>);

/// The PreviewAnchor component is held by exactly one Anchor entity that will
/// follow the cursor when the interaction mode is to add a new Anchor.
#[derive(Component)]
pub struct PreviewAnchor {
    /// If the preview anchor will be replacing an existing anchor, then this
    /// field keeps track of which anchor is being replaced. This information
    /// is helpful for sending dependents back to their original anchor if the
    /// user cancels the add-anchor interaction mode.
    replacing: Option<Entity>,
}

pub fn update_anchor_transforms(
    mut changed_anchors: Query<(&Anchor, &mut Transform), Changed<Anchor>>,
) {
    for (anchor, mut tf) in &mut changed_anchors {
        // Only update rotation and translation since scale, for drawing anchors, is managed by
        // another system.
        let new_tf = anchor.local_transform(Category::General);
        tf.translation = new_tf.translation;
        tf.rotation = new_tf.rotation;
    }
}

pub fn assign_orphan_anchors_to_parent(
    mut orphan_anchors: Query<(Entity, &mut Anchor), Without<Parent>>,
    mut commands: Commands,
    mut current_level: ResMut<CurrentLevel>,
    lifts: Query<(&LiftCabin<Entity>, &ChildCabinAnchorGroup, &GlobalTransform)>,
    lift_anchor_groups: Query<&GlobalTransform, With<CabinAnchorGroup>>,
) {
    for (e_anchor, mut anchor) in &mut orphan_anchors {
        let global_anchor_tf = anchor.local_transform(Category::General).compute_affine();
        let p_anchor = {
            let mut p = global_anchor_tf.translation;
            // Add a little height to make sure that the anchor isn't
            // numerically unstable, right on the floor of the lift cabin.
            p.z = 0.01;
            p
        };

        let mut assigned_to_lift: bool = false;
        // First check if the new anchor is inside the footprint of any lift cabins
        for (cabin, anchor_group, global_lift_tf) in &lifts {
            let cabin_aabb = match cabin {
                LiftCabin::Rect(params) => params.aabb(),
                // LiftCabin::Model(_) => {
                //     // TODO(MXG): Support models as lift cabins
                //     continue;
                // }
            };

            let sphere = Sphere {
                center: p_anchor.into(),
                radius: 0.0,
            };
            if sphere.intersects_obb(&cabin_aabb, &global_lift_tf.affine()) {
                if let Ok(anchor_group_tf) = lift_anchor_groups.get(anchor_group.0) {
                    // The anchor is inside the lift cabin, so we should
                    // make it the anchor's parent.
                    commands.entity(anchor_group.0).add_child(e_anchor);
                    assigned_to_lift = true;

                    // Since the anchor will be in the frame of the lift, we need
                    // to update its local transform.
                    anchor.move_to(&Transform::from_matrix(
                        (anchor_group_tf.affine().inverse() * global_anchor_tf).into(),
                    ));

                    break;
                }
            }
        }

        if assigned_to_lift {
            continue;
        }

        // The anchor was not assigned to a lift, so we should assign it to the
        // current level.
        let parent = if let Some(level) = current_level.0 {
            level
        } else {
            // No level is currently assigned, so we should create one.
            let new_level_id = commands
                .spawn(LevelProperties {
                    name: NameInSite("<Unnamed>".to_owned()),
                    elevation: LevelElevation(0.),
                    ..default()
                })
                .insert(Category::Level)
                .id();

            current_level.0 = Some(new_level_id);
            new_level_id
        };

        commands.entity(parent).add_child(e_anchor);
    }
}

/// Unique UUID to identify issue of anchors being close but not connected
pub const UNCONNECTED_ANCHORS_ISSUE_UUID: Uuid =
    Uuid::from_u128(0xe1ef2a60c3bc45829effdf8ca7dd3403u128);

// When triggered by a validation request event, check if there are anchors that are very close to
// each other but not connected
pub fn check_for_close_unconnected_anchors(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    parents: Query<&Parent>,
    anchors: AnchorParams,
    anchor_entities: Query<Entity, With<Anchor>>,
    levels: Query<Entity, With<LevelElevation>>,
    dependents: Query<&Dependents>,
) {
    const ISSUE_HINT: &str = "Pair of anchors that are very close but not connected was found, \
                        review if this is intended and, if it is, suppress the issue";
    // TODO(luca) make this configurable
    const DISTANCE_THRESHOLD: f32 = 0.2;
    for root in validate_events.iter() {
        // Key is level id, value is vector of (Entity, Global tf's position)
        let mut anchor_poses: HashMap<Entity, Vec<(Entity, Vec3)>> = HashMap::new();
        for e in &anchor_entities {
            if let Some(level) = AncestorIter::new(&parents, e).find(|p| levels.get(*p).is_ok()) {
                if AncestorIter::new(&parents, level).any(|p| p == **root) {
                    // Level that belongs to requested workspace
                    let poses = anchor_poses.entry(level).or_default();
                    poses.push((
                        e,
                        anchors
                            .point_in_parent_frame_of(e, Category::General, level)
                            .expect("Failed fetching anchor pose"),
                    ));
                }
            }
        }
        // Now find close unconnected pairs, sadly n^2 problem for anchors, unless we use better
        // data structures that sort in space
        for values in anchor_poses.values() {
            for ((e0, p0), (e1, p1)) in values.iter().tuple_combinations() {
                if p0.distance(*p1) < DISTANCE_THRESHOLD {
                    let mut edge_found = false;
                    if let (Ok(d0), Ok(d1)) = (dependents.get(*e0), dependents.get(*e1)) {
                        edge_found = d0.iter().any(|d| d1.contains(d));
                    }
                    if !edge_found {
                        let issue = Issue {
                            key: IssueKey {
                                entities: [*e0, *e1].into(),
                                kind: UNCONNECTED_ANCHORS_ISSUE_UUID,
                            },
                            brief: format!(
                                "Anchors are closer than {} m but unconnected",
                                DISTANCE_THRESHOLD
                            ),
                            hint: ISSUE_HINT.to_string(),
                        };
                        let id = commands.spawn(issue).id();
                        commands.entity(**root).add_child(id);
                    }
                }
            }
        }
    }
}
