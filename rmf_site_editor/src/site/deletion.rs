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
    site::AnchorDependents,
    interaction::{Selection, Select},
};
use rmf_site_format::{Edge, Point, Path};
use bevy::prelude::*;

// TODO(MXG): Use this module to implement the deletion buffer. The role of the
// deletion buffer will be to preserve deleted entities so that they can be
// easily restored if the user wants to undo the deletion.

/// Components tagged with this will not be deleted.
#[derive(Component, Default, Debug, Clone)]
pub struct PreventDeletion{
    pub reason: Option<String>,
}

impl PreventDeletion {
    pub fn because(reason: String) -> Self {
        PreventDeletion{reason: Some(reason)}
    }
}

/// This is an event used to delete site elements. Deleting the element is
/// recursive, so all its children will be deleted along with it.
#[derive(Debug, Clone, Copy)]
pub struct Delete {
    pub element: Entity,
}

impl Delete {
    pub fn new(element: Entity) -> Self {
        Self{element}
    }
}

fn perform_deletions(
    mut commands: Commands,
    preventions: Query<&PreventDeletion>,
    edges: Query<&Edge<Entity>>,
    points: Query<&Point<Entity>>,
    paths: Query<&Path<Entity>>,
    mut dependents: Query<&mut AnchorDependents>,
    mut deletions: EventReader<Delete>,
    selection: Res<Selection>,
    mut select: EventWriter<Select>,
) {
    for delete in deletions.iter() {
        if let Ok(anchor) = dependents.get(delete.element) {
            if !anchor.dependents.is_empty() {
                println!(
                    "Cannot delete anchor {:?} because it has {} dependents. \
                    Only anchors with no dependents can be deleted.",
                    delete.element,
                    anchor.dependents.len(),
                );
                continue;
            }
        }

        if let Ok(prevent) = preventions.get(delete.element) {
            if let Some(reason) = &prevent.reason {
                println!(
                    "Element {:?} cannot be deleted because: {}",
                    delete.element,
                    reason,
                );
                continue;
            }
        }

        if let Ok(edge) = edges.get(delete.element) {
            for anchor in edge.array() {
                if let Ok(mut dep) = dependents.get_mut(anchor) {
                    dep.dependents.remove(&delete.element);
                }
            }
        }

        if let Ok(point) = points.get(delete.element) {
            if let Ok(mut dep) = dependents.get_mut(point.0) {
                dep.dependents.remove(&delete.element);
            }
        }

        if let Ok(path) = paths.get(delete.element) {
            for anchor in &path.0 {
                if let Ok(mut dep) = dependents.get_mut(*anchor) {
                    dep.dependents.remove(&delete.element);
                }
            }
        }

        if **selection == Some(delete.element) {
            select.send(Select(None));
        }

        // TODO(MXG): Replace this with a move to the trash bin.
        commands.entity(delete.element).despawn_recursive();
    }
}

pub struct DeletionPlugin;

impl Plugin for DeletionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<Delete>()
            .add_system_to_stage(CoreStage::First, perform_deletions);
    }
}
