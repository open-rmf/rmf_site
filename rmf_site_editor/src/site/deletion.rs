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
    interaction::{Select, Selection},
    log::Log,
    site::{
        Category, CurrentLevel, Dependents, LevelElevation, LevelProperties, NameInSite,
        SiteUpdateSet,
    },
    Issue,
};
use bevy::{
    ecs::system::{BoxedSystem, SystemParam, SystemState},
    prelude::*,
};
use rmf_site_format::{Edge, Path, Point};
use std::collections::HashSet;

// TODO(MXG): Use this module to implement the deletion buffer. The role of the
// deletion buffer will be to preserve deleted entities so that they can be
// easily restored if the user wants to undo the deletion.

/// Components tagged with this will not be deleted.
#[derive(Component, Default, Debug, Clone)]
pub struct PreventDeletion {
    pub reason: Option<String>,
}

impl PreventDeletion {
    pub fn because(reason: String) -> Self {
        PreventDeletion {
            reason: Some(reason),
        }
    }
}

/// This is an event used to delete site elements. Deleting the element is
/// recursive, so all its children will be deleted along with it.
#[derive(Debug, Clone, Copy, Eq, Event, Hash, PartialEq)]
pub struct Delete {
    pub element: Entity,
    /// If this is true, all dependents of the element or any of its children
    /// will also be deleted. This will also delete dependents of the dependents
    /// and their children recursively.
    ///
    /// If this is false, the entity will not be deleted if it or its children
    /// have any dependents that do not descent from the root entity that is
    /// being deleted.
    pub and_dependents: bool,
}

impl Delete {
    pub fn new(element: Entity) -> Self {
        Self {
            element,
            and_dependents: false,
        }
    }

    pub fn and_dependents(mut self) -> Self {
        self.and_dependents = true;
        self
    }
}

#[derive(SystemParam)]
struct DeletionParams<'w, 's> {
    commands: Commands<'w, 's>,
    preventions: Query<'w, 's, &'static PreventDeletion>,
    edges: Query<'w, 's, &'static Edge<Entity>>,
    points: Query<'w, 's, &'static Point<Entity>>,
    paths: Query<'w, 's, &'static Path<Entity>>,
    parents: Query<'w, 's, &'static mut Parent>,
    dependents: Query<'w, 's, &'static mut Dependents>,
    children: Query<'w, 's, &'static Children>,
    selection: Res<'w, Selection>,
    current_level: ResMut<'w, CurrentLevel>,
    levels: Query<'w, 's, Entity, With<LevelElevation>>,
    select: EventWriter<'w, Select>,
    log: EventWriter<'w, Log>,
    issues: Query<'w, 's, (Entity, &'static mut Issue)>,
}

pub struct DeletionPlugin;

impl Plugin for DeletionPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            First,
            (SiteUpdateSet::Deletion, SiteUpdateSet::DeletionFlush).chain(),
        )
        .add_systems(First, apply_deferred.in_set(SiteUpdateSet::DeletionFlush))
        .add_event::<Delete>()
        .init_resource::<DeletionFilters>()
        .add_systems(
            First,
            handle_deletion_requests.in_set(SiteUpdateSet::Deletion),
        );
    }
}

#[derive(Deref, DerefMut)]
pub struct DeletionBox(pub BoxedSystem<HashSet<Delete>, HashSet<Delete>>);

#[derive(Default, Resource)]
pub struct DeletionFilters {
    boxed_systems: Vec<DeletionBox>,
    pending_insertion: Vec<DeletionBox>,
}

impl DeletionFilters {
    pub fn insert(&mut self, filter: DeletionBox) {
        self.pending_insertion.push(filter);
    }

    fn insert_boxes(&mut self, world: &mut World) {
        for mut inserted in self.pending_insertion.drain(..) {
            inserted.initialize(world);
            self.boxed_systems.push(inserted);
        }
    }

    fn run_boxes(
        &mut self,
        mut pending_delete: HashSet<Delete>,
        world: &mut World,
    ) -> HashSet<Delete> {
        for boxed_system in self.boxed_systems.iter_mut() {
            pending_delete = boxed_system.0.run(pending_delete, world);
        }
        pending_delete
    }
}

fn handle_deletion_requests(
    world: &mut World,
    state: &mut SystemState<(EventReader<Delete>, DeletionParams)>,
) {
    let (mut deletions, _) = state.get_mut(world);
    if deletions.is_empty() {
        return;
    }
    let mut pending_delete: HashSet<Delete> = HashSet::new();
    for delete in deletions.read() {
        pending_delete.insert(*delete);
    }

    pending_delete =
        world.resource_scope::<DeletionFilters, _>(move |world, mut deletion_filters| {
            deletion_filters.insert_boxes(world);
            // Run through all boxed systems to filter out entities that should not
            // be sent to delete
            deletion_filters.run_boxes(pending_delete, world)
        });

    let (_, mut params) = state.get_mut(world);
    for delete in pending_delete.iter() {
        if delete.and_dependents {
            recursive_dependent_delete(delete.element, &mut params);
        } else {
            cautious_delete(delete.element, &mut params);
        }
    }
    state.apply(world);
}

fn cautious_delete(element: Entity, params: &mut DeletionParams) {
    let mut all_descendents = HashSet::new();
    let mut queue = Vec::new();
    queue.push(element);
    while let Some(top) = queue.pop() {
        all_descendents.insert(top);
        if let Ok(children) = params.children.get(top) {
            for child in children {
                queue.push(*child);
            }
        }
    }

    for descendent in &all_descendents {
        if let Ok(prevent) = params.preventions.get(*descendent) {
            if *descendent == element {
                params.log.send(Log::hint(format!(
                    "Element {:?} cannot be deleted because: {}",
                    element,
                    prevent
                        .reason
                        .as_ref()
                        .unwrap_or(&"<.. no reason given>".to_string()),
                )));
            } else {
                params.log.send(Log::hint(format!(
                    "Element {:?} is an ancestor of {:?} which cannot be \
                    deleted because: {}",
                    element,
                    descendent,
                    prevent
                        .reason
                        .as_ref()
                        .unwrap_or(&"<.. no reason given>".to_string()),
                )));
            }
            return;
        }

        if let Ok(dependents) = params.dependents.get(*descendent) {
            for dep in dependents.iter() {
                if !all_descendents.contains(dep) {
                    if *descendent == element {
                        params.log.send(Log::hint(format!(
                            "Cannot delete {:?} because it has {} dependents. \
                            Only elements with no outside dependents can be \
                            deleted.",
                            element,
                            dependents.len(),
                        )));
                    } else {
                        params.log.send(Log::hint(format!(
                            "Element {:?} is an ancestor of {:?} \
                            which cannot be deleted because {:?} depends \
                            on it.",
                            element, descendent, dep,
                        )));
                    }
                    return;
                }
            }
        }
    }

    for e in all_descendents {
        if let Ok(edge) = params.edges.get(e) {
            for anchor in edge.array() {
                if let Ok(mut deps) = params.dependents.get_mut(anchor) {
                    deps.remove(&e);
                }
            }
        }

        if let Ok(point) = params.points.get(e) {
            if let Ok(mut deps) = params.dependents.get_mut(point.0) {
                deps.remove(&e);
            }
        }

        if let Ok(path) = params.paths.get(e) {
            for anchor in &path.0 {
                if let Ok(mut deps) = params.dependents.get_mut(*anchor) {
                    deps.remove(&e);
                }
            }
        }

        if **params.selection == Some(e) {
            params.select.send(Select(None));
        }
    }

    for (e, mut issue) in &mut params.issues {
        issue.key.entities.remove(&element);
        if issue.key.entities.is_empty() {
            params.commands.entity(e).despawn_recursive();
        }
    }

    // Fetch the parent and delete this dependent
    // TODO(luca) should we add this snippet to the recursive delete also?
    if let Ok(parent) = params.parents.get(element) {
        if let Ok(mut parent_dependents) = params.dependents.get_mut(**parent) {
            parent_dependents.remove(&element);
        }
    }

    params.commands.entity(element).despawn_recursive();
}

fn recursive_dependent_delete(element: Entity, params: &mut DeletionParams) {
    let mut all_to_delete = HashSet::new();
    let mut queue = Vec::new();
    queue.push(element);
    while let Some(top) = queue.pop() {
        if let Ok(prevent) = params.preventions.get(top) {
            if top == element {
                params.log.send(Log::hint(format!(
                    "Cannot delete {:?} because: {}",
                    element,
                    prevent
                        .reason
                        .as_ref()
                        .unwrap_or(&"<.. no reason given>".to_string()),
                )));
            } else {
                params.log.send(Log::hint(format!(
                    "Cannot delete {:?} because we would need to also delete \
                    {:?} which cannot be deleted because: {}",
                    element,
                    top,
                    prevent
                        .reason
                        .as_ref()
                        .unwrap_or(&"<.. no reason given>".to_string()),
                )));
            }
            return;
        }

        if all_to_delete.contains(&top) {
            continue;
        }

        all_to_delete.insert(top);
        if let Ok(children) = params.children.get(top) {
            for child in children {
                if !all_to_delete.contains(child) {
                    queue.push(*child);
                }
            }
        }

        if let Ok(dependents) = params.dependents.get(top) {
            for dependent in dependents.iter() {
                if !all_to_delete.contains(&dependent) {
                    queue.push(*dependent);
                }
            }
        }
    }

    perform_deletions(all_to_delete, params);
}

fn perform_deletions(all_to_delete: HashSet<Entity>, params: &mut DeletionParams) {
    for e in all_to_delete.iter().copied() {
        // TODO(MXG): Consider refactoring some of this bookkeeping to separate
        // systems that use the RemovedComponents system parameter.
        if let Ok(edge) = params.edges.get(e) {
            for anchor in edge.array() {
                if !all_to_delete.contains(&anchor) {
                    if let Ok(mut deps) = params.dependents.get_mut(anchor) {
                        deps.remove(&e);
                    }
                }
            }
        }

        if let Ok(point) = params.points.get(e) {
            if !all_to_delete.contains(&point.0) {
                if let Ok(mut deps) = params.dependents.get_mut(point.0) {
                    deps.remove(&e);
                }
            }
        }

        if let Ok(path) = params.paths.get(e) {
            for anchor in &path.0 {
                if !all_to_delete.contains(anchor) {
                    if let Ok(mut deps) = params.dependents.get_mut(*anchor) {
                        deps.remove(&e);
                    }
                }
            }
        }

        if **params.selection == Some(e) {
            params.select.send(Select(None));
        }

        if **params.current_level == Some(e) {
            // We are deleting the current level, so we should try to switch to
            // a different one.
            let found_level = {
                let mut found_level = false;
                for level in &params.levels {
                    if !all_to_delete.contains(&level) {
                        found_level = true;
                        *params.current_level = CurrentLevel(Some(level));
                    }
                }
                found_level
            };

            if !found_level {
                // We need to make a whole new level and set it as the current
                // level because all the existing levels are being deleted.
                let new_level = params
                    .commands
                    .spawn(SpatialBundle::default())
                    .insert(LevelProperties {
                        name: NameInSite("<Unnamed>".to_owned()),
                        elevation: LevelElevation(0.0),
                        ..default()
                    })
                    .insert(Category::Level)
                    .id();
                *params.current_level = CurrentLevel(Some(new_level));
            }
        }

        // TODO(MXG): Replace this with a move to the trash bin group.
        params.commands.entity(e).despawn();
    }
}
