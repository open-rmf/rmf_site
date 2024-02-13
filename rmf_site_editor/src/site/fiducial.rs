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

use crate::interaction::VisualCue;
use crate::site::*;
use crate::{Issue, ValidateWorkspace};
use bevy::{prelude::*, utils::Uuid};
use std::collections::HashMap;

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
    parent: Query<&Parent>,
    fiducials: Query<&Affiliation<Entity>, With<FiducialMarker>>,
    fiducial_groups: Query<(Entity, &NameInSite, &Parent), (With<Group>, With<FiducialMarker>)>,
    children: Query<&Children>,
) {
    for e in &new_fiducial_scope {
        if let Some(site) = find_parent_site(e, &sites, &parent) {
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
    mut unused_fiducial_trackers: Query<(Entity, &mut FiducialUsage)>,
    changed_parent: Query<Entity, (With<DrawingMarker>, Changed<Parent>)>,
    changed_fiducial: Query<&Parent, (Changed<Affiliation<Entity>>, With<FiducialMarker>)>,
    sites: Query<(), With<NameOfSite>>,
    parent: Query<&Parent>,
    children: Query<&Children>,
    fiducials: Query<&Affiliation<Entity>, With<FiducialMarker>>,
    changed_fiducials: Query<
        (Entity, &Parent),
        (Changed<Affiliation<Entity>>, With<FiducialMarker>),
    >,
    fiducial_groups: Query<(Entity, &NameInSite, &Parent), (With<Group>, With<FiducialMarker>)>,
    changed_fiducial_groups: Query<
        Entity,
        Or<(
            (Added<Group>, With<FiducialMarker>),
            (With<Group>, Added<FiducialMarker>),
            (
                Or<(Changed<Parent>, Changed<NameInSite>)>,
                With<Group>,
                With<FiducialMarker>,
            ),
        )>,
    >,
    mut removed_fiducial_groups: RemovedComponents<Group>,
) {
    for e in &changed_parent {
        if let Some(site) = find_parent_site(e, &sites, &parent) {
            if let Ok((_, mut unused)) = unused_fiducial_trackers.get_mut(e) {
                unused.site = site;
            }
        }
    }

    for e in changed_parent
        .iter()
        .chain(changed_fiducial.iter().map(|p| p.get()))
    {
        let Ok((_, mut tracker)) = unused_fiducial_trackers.get_mut(e) else {
            continue;
        };
        reset_fiducial_usage(e, &mut tracker, &fiducials, &fiducial_groups, &children);
    }

    for changed_group in &changed_fiducial_groups {
        let Ok((_, name, site)) = fiducial_groups.get(changed_group) else {
            continue;
        };
        for (e, mut tracker) in &mut unused_fiducial_trackers {
            if tracker.site == site.get() {
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

    for (changed_fiducial, parent) in &changed_fiducials {
        let Ok((e, mut tracker)) = unused_fiducial_trackers.get_mut(parent.get()) else {
            continue;
        };
        reset_fiducial_usage(e, &mut tracker, &fiducials, &fiducial_groups, &children);
    }

    for removed_group in removed_fiducial_groups.iter() {
        for (_, mut tracker) in &mut unused_fiducial_trackers {
            tracker.used.remove(&removed_group);
            tracker.unused.remove(&removed_group);
        }
    }
}

fn find_parent_site(
    mut entity: Entity,
    sites: &Query<(), With<NameOfSite>>,
    parents: &Query<&Parent>,
) -> Option<Entity> {
    loop {
        if sites.contains(entity) {
            return Some(entity);
        }

        if let Ok(parent) = parents.get(entity) {
            entity = parent.get();
        } else {
            return None;
        }
    }
}

fn reset_fiducial_usage(
    entity: Entity,
    tracker: &mut FiducialUsage,
    fiducials: &Query<&Affiliation<Entity>, With<FiducialMarker>>,
    fiducial_groups: &Query<(Entity, &NameInSite, &Parent), (With<Group>, With<FiducialMarker>)>,
    children: &Query<&Children>,
) {
    tracker.unused.clear();
    for (group, name, site) in fiducial_groups {
        if site.get() == tracker.site {
            tracker.unused.insert(group, name.0.clone());
            tracker.used.remove(&group);
        }
    }

    let Ok(scope_children) = children.get(entity) else {
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
    fiducials: Query<(Entity, &Point<Entity>, Option<&Transform>), Added<FiducialMarker>>,
    fiducial_groups: Query<Entity, (Added<FiducialMarker>, With<Group>)>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, point, tf) in fiducials.iter() {
        if let Ok(mut deps) = dependents.get_mut(point.0) {
            deps.insert(e);
        }

        if tf.is_none() {
            commands.entity(e).insert(SpatialBundle::INHERITED_IDENTITY);
        }

        commands
            .entity(e)
            .insert(assets.fiducial_mesh.clone())
            .insert(assets.fiducial_material.clone())
            .insert(VisibilityBundle::default())
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
        (Entity, &Point<Entity>),
        (With<FiducialMarker>, Without<Parent>, Without<Pending>),
    >,
    anchors: Query<&Parent, With<Anchor>>,
    site_id: Query<&SiteID>,
) {
    for (e, point) in &orphans {
        if let Ok(parent) = anchors.get(point.0) {
            commands.entity(e).set_parent(parent.get());
        }
    }
}

pub fn update_changed_fiducial(
    mut fiducials: Query<
        (Entity, &Point<Entity>, &mut Transform),
        (
            With<FiducialMarker>,
            Or<(Changed<Point<Entity>>, Changed<Parent>)>,
        ),
    >,
    anchors: AnchorParams,
) {
    for (e, point, mut tf) in fiducials.iter_mut() {
        let position = anchors
            .point_in_parent_frame_of(point.0, Category::Fiducial, e)
            .unwrap();
        tf.translation = position;
    }
}

pub fn update_fiducial_for_moved_anchors(
    mut fiducials: Query<(Entity, &Point<Entity>, &mut Transform), With<FiducialMarker>>,
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
                let position = anchors
                    .point_in_parent_frame_of(point.0, Category::Fiducial, e)
                    .unwrap();
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
    parents: Query<&Parent>,
    fiducial_affiliations: Query<(Entity, &Affiliation<Entity>), With<FiducialMarker>>,
) {
    const ISSUE_HINT: &str = "Fiducial affiliations are used by the site editor to map matching \
                            fiducials between different floors or drawings and calculate their \
                            relative transform, fiducials without affiliation are ignored";
    for root in validate_events.iter() {
        for (e, affiliation) in &fiducial_affiliations {
            if AncestorIter::new(&parents, e).any(|p| p == **root) {
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
