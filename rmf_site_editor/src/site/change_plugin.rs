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

use crate::site::SiteUpdateSet;
use bevy::prelude::*;
use std::{fmt::Debug, sync::Arc};

use super::{RevisionTracker, UndoEvent};

/// The Change component is used as an event to indicate that the value of a
/// component should change for some entity. Using these events instead of
/// modifying the component directly helps with managing an undo/redo buffer.
#[derive(Debug, Clone, Event)]
pub struct Change<T: Component + Clone + Debug> {
    pub to_value: T,
    pub for_element: Entity,
    pub allow_insert: bool,
}

impl<T: Component + Clone + Debug> Change<T> {
    pub fn new(to_value: T, for_element: Entity) -> Self {
        Self {
            to_value,
            for_element,
            allow_insert: false,
        }
    }

    pub fn or_insert(mut self) -> Self {
        self.allow_insert = true;
        self
    }
}

// TODO(MXG): We could consider allowing the user to specify a query filter so
// this plugin only targets certain types.
pub struct ChangePlugin<T: Component + Clone + Debug> {
    _ignore: std::marker::PhantomData<T>,
}

impl<T: Component + Clone + Debug> Default for ChangePlugin<T> {
    fn default() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

/// This is a changelog used for the undo/redo system
struct ChangeLog<T: Component + Clone + Debug> {
    entity: Entity,
    from: Option<T>,
    to: T,
}

#[derive(Resource)]
struct ChangeHistory<T: Component + Clone + Debug> {
    pub(crate) revisions: std::collections::HashMap<usize, ChangeLog<T>>,
}

impl<T: Component + Clone + Debug> Default for ChangeHistory<T> {
    fn default() -> Self {
        Self {
            revisions: Default::default(),
        }
    }
}

impl<T: Component + Clone + Debug> Plugin for ChangePlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<Change<T>>()
            .init_resource::<ChangeHistory<T>>()
            .add_systems(
                PreUpdate,
                (
                    update_changed_values::<T>.in_set(SiteUpdateSet::ProcessChanges),
                    undo_change::<T>.in_set(SiteUpdateSet::ProcessChanges),
                ), // TODO do this on another stage
            );
    }
}

fn undo_change<T: Component + Clone + Debug>(
    mut commands: Commands,
    mut values: Query<&mut T>,
    change_history: ResMut<ChangeHistory<T>>,
    mut undo_cmds: EventReader<UndoEvent>,
) {
    for undo in undo_cmds.read() {
        let Some(change) = change_history.revisions.get(&undo.action_id) else {
            continue;
        };

        if let Ok(mut component_to_change) = values.get_mut(change.entity) {
            if let Some(old_value) = &change.from {
                *component_to_change = old_value.clone();
            } else {
                commands.entity(change.entity).remove::<T>();
            }
        } else {
            error!("Undo history corrupted.");
        }
    }
}

fn update_changed_values<T: Component + Clone + Debug>(
    mut commands: Commands,
    mut values: Query<&mut T>,
    mut changes: EventReader<Change<T>>,
    mut undo_buffer: ResMut<RevisionTracker>,
    mut change_history: ResMut<ChangeHistory<T>>,
) {
    for change in changes.read() {
        if let Ok(mut component_to_change) = values.get_mut(change.for_element) {
            change_history.revisions.insert(
                undo_buffer.get_next_revision(),
                ChangeLog {
                    entity: change.for_element,
                    to: change.to_value.clone(),
                    from: Some(component_to_change.clone()),
                },
            );
            *component_to_change = change.to_value.clone();
        } else {
            if change.allow_insert {
                commands
                    .entity(change.for_element)
                    .insert(change.to_value.clone());
                change_history.revisions.insert(
                    undo_buffer.get_next_revision(),
                    ChangeLog {
                        entity: change.for_element,
                        to: change.to_value.clone(),
                        from: None,
                    },
                );
            } else {
                error!(
                    "Unable to change {} data to {:?} for entity {:?} \
                    because the entity does not have that type",
                    std::any::type_name::<T>(),
                    change.to_value,
                    change.for_element,
                );
            }
        }
    }
}
