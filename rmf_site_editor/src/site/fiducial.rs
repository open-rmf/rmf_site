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
use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Component)]
pub struct UnusedFiducials {
    site: Entity,
    unused: HashSet<Entity>,
}

impl UnusedFiducials {
    pub fn unused(&self) -> &HashSet<Entity> {
        &self.unused
    }
}

pub fn add_unused_fiducial_tracker(
    mut commands: Commands,
    new_fiducial_scope: Query<Entity, Or<(Added<DrawingMarker>, Added<NameOfSite>)>>,
    sites: Query<(), With<NameOfSite>>,
    parent: Query<&Parent>,
    fiducials: Query<&Affiliation<Entity>, With<FiducialMarker>>,
    fiducial_groups: Query<(Entity, &Parent), (With<Group>, With<FiducialMarker>)>,
    children: Query<&Children>,
) {
    for e in &new_fiducial_scope {
        if let Some(site) = find_parent_site(e, &sites, &parent) {
            let mut tracker = UnusedFiducials {
                site,
                unused: Default::default(),
            };
            reset_unused(e, &mut tracker, &fiducials, &fiducial_groups, &children);
            commands.entity(e).insert(tracker);
        }
    }
}

pub fn update_unused_fiducial_tracker(
    mut unused_fiducial_trackers: Query<&mut UnusedFiducials>,
    changed_parent: Query<Entity, (With<DrawingMarker>, Changed<Parent>)>,
    changed_fiducial: Query<&Parent, (Changed<Affiliation<Entity>>, With<FiducialMarker>)>,
    sites: Query<(), With<NameOfSite>>,
    parent: Query<&Parent>,
    children: Query<&Children>,
    fiducials: Query<&Affiliation<Entity>, With<FiducialMarker>>,
    fiducial_groups: Query<(Entity, &Parent), (With<Group>, With<FiducialMarker>)>,
    new_fiducial_groups: Query<Entity, Or<(
        (Added<Group>, With<FiducialMarker>),
        (With<Group>, Added<FiducialMarker>),
        (Changed<Parent>, With<Group>, With<FiducialMarker>),
    )>>,
    removed_fiducial_group: RemovedComponents<Group>,
) {
    for e in &changed_parent {
        if let Some(site) = find_parent_site(e, &sites, &parent) {
            if let Ok(mut unused) = unused_fiducial_trackers.get_mut(e) {
                unused.site = site;
            }
        }
    }

    for e in changed_parent.iter()
        .chain(changed_fiducial.iter().map(|p| p.get()))
    {
        let Ok(mut tracker) = unused_fiducial_trackers.get_mut(e) else { continue };
        reset_unused(e, &mut tracker, &fiducials, &fiducial_groups, &children);
    }

    for new_group in &new_fiducial_groups {
        let Ok((_, site)) = fiducial_groups.get(new_group) else { continue };
        for mut tracker in &mut unused_fiducial_trackers {
            if tracker.site == site.get() {
                tracker.unused.insert(new_group);
            } else {
                // If we ever want to support moving a fiducial group between
                // sites, this will take care of that. Otherwise this line will
                // never be executed.
                tracker.unused.remove(&new_group);
            }
        }
    }

    for removed_group in &removed_fiducial_group {
        for mut tracker in &mut unused_fiducial_trackers {
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

fn reset_unused(
    entity: Entity,
    tracker: &mut UnusedFiducials,
    fiducials: &Query<&Affiliation<Entity>, With<FiducialMarker>>,
    fiducial_groups: &Query<(Entity, &Parent), (With<Group>, With<FiducialMarker>)>,
    children: &Query<&Children>,
) {
    tracker.unused.clear();
    for (group, site) in fiducial_groups {
        if site.get() == tracker.site {
            tracker.unused.insert(group);
        }
    }

    let Ok(scope_children) = children.get(entity) else { return };
    for child in scope_children {
        let Ok(affiliation) = fiducials.get(*child) else { return };
        if let Some(group) = affiliation.0 {
            tracker.unused.remove(&group);
        }
    }
}

pub fn add_fiducial_visuals(
    mut commands: Commands,
    fiducials: Query<(Entity, &Point<Entity>, Option<&Transform>), Added<FiducialMarker>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, point, tf) in fiducials.iter() {
        if let Ok(mut deps) = dependents.get_mut(point.0) {
            deps.insert(e);
        }

        if tf.is_none() {
            commands.entity(e).insert(SpatialBundle::VISIBLE_IDENTITY);
        }

        commands
            .entity(e)
            .insert(assets.fiducial_mesh.clone())
            .insert(assets.fiducial_material.clone())
            .insert(Category::Fiducial)
            .insert(VisualCue::outline());
    }
}

pub fn update_changed_fiducial(
    mut fiducials: Query<
        (Entity, &Point<Entity>, &mut Transform),
        (Changed<Point<Entity>>, With<FiducialMarker>),
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
