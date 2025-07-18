/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::site::*;
use crate::{Issue, ValidateWorkspace};
use bevy::ecs::{hierarchy::ChildOf, relationship::AncestorIter};
use bevy::prelude::*;
use rmf_site_picking::VisualCue;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Component)]
pub struct FiducialUsage {
    site: Entity,
    used: HashMap<Entity, String>,
    unused: HashMap<Entity, String>,
}

impl FiducialUsage {
    pub fn used(&self) -> &HashMap<Entity, String> {
        &self.used
    }

    pub fn unused(&self) -> &HashMap<Entity, String> {
        &self.unused
    }

    pub fn site(&self) -> Entity {
        self.site
    }
}

pub fn add_unused_fiducial_tracker(
    mut commands: Commands,
    new_fiducial_scope: Query<Entity, Or<(Added<DrawingMarker>, Added<NameOfSite>)>>,
    sites: Query<(), With<NameOfSite>>,
    child_of: Query<&ChildOf>,
    fiducials: Query<&Affiliation, With<FiducialMarker>>,
    fiducial_groups: Query<(Entity, &NameInSite, &ChildOf), (With<Group>, With<FiducialMarker>)>,
    children: Query<&Children>,
) {
    for e in &new_fiducial_scope {
        if let Some(site) = find_parent_site(e, &sites, &child_of) {
            let mut tracker = FiducialUsage {
                site,
                used: Default::default(),
                unused: Default::default(),
            };
            reset_fiducial_usage(e, &mut tracker, &fiducials, &fiducial_groups, &children);
            commands.entity(e).insert(tracker);
        }
    }
}

pub fn update_fiducial_usage_tracker(
    mut fiducial_usage_trackers: Query<(Entity, &mut FiducialUsage)>,
    changed_scopes: Query<
        Entity,
        (
            With<DrawingMarker>,
            Or<(Changed<ChildOf>, Changed<Children>)>,
        ),
    >,
    changed_fiducials: Query<
        &ChildOf,
        (
            Or<(Changed<Affiliation>, Changed<ChildOf>)>,
            With<FiducialMarker>,
        ),
    >,
    sites: Query<(), With<NameOfSite>>,
    child_of: Query<&ChildOf>,
    children: Query<&Children>,
    fiducials: Query<&Affiliation, With<FiducialMarker>>,
    fiducial_groups: Query<(Entity, &NameInSite, &ChildOf), (With<Group>, With<FiducialMarker>)>,
    changed_fiducial_groups: Query<
        Entity,
        Or<(
            (Added<Group>, With<FiducialMarker>),
            (With<Group>, Added<FiducialMarker>),
            (
                Or<(Changed<ChildOf>, Changed<NameInSite>)>,
                With<Group>,
                With<FiducialMarker>,
            ),
        )>,
    >,
    mut removed_fiducial_groups: RemovedComponents<Group>,
) {
    for e in &changed_scopes {
        if let Some(site) = find_parent_site(e, &sites, &child_of) {
            if let Ok((_, mut unused)) = fiducial_usage_trackers.get_mut(e) {
                unused.site = site;
            }
        }
    }

    for e in changed_scopes
        .iter()
        .chain(changed_fiducials.iter().map(|co| co.parent()))
    {
        let Ok((_, mut tracker)) = fiducial_usage_trackers.get_mut(e) else {
            continue;
        };
        reset_fiducial_usage(e, &mut tracker, &fiducials, &fiducial_groups, &children);
    }

    for changed_group in &changed_fiducial_groups {
        let Ok((_, name, site)) = fiducial_groups.get(changed_group) else {
            continue;
        };
        for (e, mut tracker) in &mut fiducial_usage_trackers {
            if tracker.site == site.parent() {
                tracker.unused.insert(changed_group, name.0.clone());
                let Ok(scope_children) = children.get(e) else {
                    continue;
                };
                for child in scope_children {
                    let Ok(affiliation) = fiducials.get(*child) else {
                        continue;
                    };
                    if let Some(group) = affiliation.0 {
                        if changed_group == group {
                            tracker.unused.remove(&changed_group);
                            tracker.used.insert(changed_group, name.0.clone());
                        }
                    }
                }
            } else {
                // If we ever want to support moving a fiducial group between
                // sites, this will take care of that. Otherwise this line will
                // never be executed.
                tracker.used.remove(&changed_group);
                tracker.unused.remove(&changed_group);
            }
        }
    }

    for removed_group in removed_fiducial_groups.read() {
        for (_, mut tracker) in &mut fiducial_usage_trackers {
            tracker.used.remove(&removed_group);
            tracker.unused.remove(&removed_group);
        }
    }
}

fn find_parent_site(
    mut entity: Entity,
    sites: &Query<(), With<NameOfSite>>,
    child_of: &Query<&ChildOf>,
) -> Option<Entity> {
    loop {
        if sites.contains(entity) {
            return Some(entity);
        }

        if let Ok(child_of) = child_of.get(entity) {
            entity = child_of.parent();
        } else {
            return None;
        }
    }
}

fn reset_fiducial_usage(
    scope: Entity,
    tracker: &mut FiducialUsage,
    fiducials: &Query<&Affiliation, With<FiducialMarker>>,
    fiducial_groups: &Query<(Entity, &NameInSite, &ChildOf), (With<Group>, With<FiducialMarker>)>,
    children: &Query<&Children>,
) {
    tracker.unused.clear();
    tracker.used.clear();
    for (group, name, site) in fiducial_groups {
        if site.parent() == tracker.site {
            tracker.unused.insert(group, name.0.clone());
        }
    }

    let Ok(scope_children) = children.get(scope) else {
        return;
    };
    for child in scope_children {
        let Ok(affiliation) = fiducials.get(*child) else {
            continue;
        };
        if let Some(group) = affiliation.0 {
            tracker.unused.remove(&group);
            if let Ok((_, name, _)) = fiducial_groups.get(group) {
                tracker.used.insert(group, name.0.clone());
            }
        }
    }
}

pub fn add_fiducial_visuals(
    mut commands: Commands,
    fiducials: Query<(Entity, &Point, Option<&Transform>), Added<FiducialMarker>>,
    fiducial_groups: Query<Entity, (Added<FiducialMarker>, With<Group>)>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, point, tf) in fiducials.iter() {
        if let Ok(mut deps) = dependents.get_mut(point.0) {
            deps.insert(e);
        }

        if tf.is_none() {
            commands
                .entity(e)
                .insert((Transform::IDENTITY, Visibility::Inherited));
        }

        commands
            .entity(e)
            .insert(Mesh3d(assets.fiducial_mesh.clone()))
            .insert(MeshMaterial3d(assets.fiducial_material.clone()))
            .insert(Visibility::default())
            .insert(Category::Fiducial)
            .insert(VisualCue::outline());
    }

    for e in &fiducial_groups {
        commands.entity(e).insert(Category::FiducialGroup);
    }
}

pub fn assign_orphan_fiducials_to_parent(
    mut commands: Commands,
    orphans: Query<
        (Entity, &Point),
        (With<FiducialMarker>, Without<ChildOf>, Without<Pending>),
    >,
    anchors: Query<&ChildOf, With<Anchor>>,
) {
    for (e, point) in &orphans {
        if let Ok(child_of) = anchors.get(point.0) {
            commands.entity(e).insert(ChildOf(child_of.parent()));
        } else {
            error!(
                "No parent for anchor {:?} needed by fiducial {e:?}",
                point.0
            );
        }
    }
}

pub fn update_changed_fiducial(
    mut fiducials: Query<
        (Entity, &Point, &mut Transform),
        (
            With<FiducialMarker>,
            Or<(Changed<Point>, Changed<ChildOf>)>,
        ),
    >,
    anchors: AnchorParams,
) {
    for (e, point, mut tf) in fiducials.iter_mut() {
        let position = match anchors.point_in_parent_frame_of(point.0, Category::Fiducial, e) {
            Ok(position) => position,
            Err(err) => {
                error!("failed to update fiducial: {err}");
                return;
            }
        };
        tf.translation = position;
    }
}

pub fn update_fiducial_for_moved_anchors(
    mut fiducials: Query<(Entity, &Point, &mut Transform), With<FiducialMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, point, mut tf)) = fiducials.get_mut(*dependent) {
                let position =
                    match anchors.point_in_parent_frame_of(point.0, Category::Fiducial, e) {
                        Ok(position) => position,
                        Err(err) => {
                            error!("failed to update fiducial: {err}");
                            continue;
                        }
                    };
                tf.translation = position;
            }
        }
    }
}

/// Unique UUID to identify issue of fiducials without affiliation
pub const FIDUCIAL_WITHOUT_AFFILIATION_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x242a655f67cc4d4f9176ed5d64cd87f0u128);

// When triggered by a validation request event, check if there are fiducials without affiliation,
// generate an issue if that is the case
pub fn check_for_fiducials_without_affiliation(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    child_of: Query<&ChildOf>,
    fiducial_affiliations: Query<(Entity, &Affiliation), With<FiducialMarker>>,
) {
    const ISSUE_HINT: &str = "Fiducial affiliations are used by the site editor to map matching \
                            fiducials between different floors or drawings and calculate their \
                            relative transform, fiducials without affiliation are ignored";
    for root in validate_events.read() {
        for (e, affiliation) in &fiducial_affiliations {
            if AncestorIter::new(&child_of, e).any(|p| p == **root) {
                if affiliation.0.is_none() {
                    let issue = Issue {
                        key: IssueKey {
                            entities: [e].into(),
                            kind: FIDUCIAL_WITHOUT_AFFILIATION_ISSUE_UUID,
                        },
                        brief: format!("Fiducial without affiliation found"),
                        hint: ISSUE_HINT.to_string(),
                    };
                    let id = commands.spawn(issue).id();
                    commands.entity(**root).add_child(id);
                }
            }
        }
    }
}
