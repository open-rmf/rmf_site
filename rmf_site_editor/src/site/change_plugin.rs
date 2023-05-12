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

use crate::site::{SiteState, SiteUpdateLabel};
use crate::AppState;
use bevy::prelude::*;
use std::fmt::Debug;

/// The Change component is used as an event to indicate that the value of a
/// component should change for some entity. Using these events instead of
/// modifying the component directly helps with managing an undo/redo buffer.
#[derive(Debug, Clone)]
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

impl<T: Component + Clone + Debug> Plugin for ChangePlugin<T> {
    fn build(&self, app: &mut App) {
        // TODO(luca) this is duplicated, refactor app states to avoid?
        app.add_event::<Change<T>>()
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::on_update(SiteState::Display)
                    .label(SiteUpdateLabel::ProcessChanges)
                    .with_system(update_changed_values::<T>),
            )
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                    .with_system(update_changed_values::<T>),
            );
    }
}

fn update_changed_values<T: Component + Clone + Debug>(
    mut commands: Commands,
    mut values: Query<&mut T>,
    mut changes: EventReader<Change<T>>,
) {
    for change in changes.iter() {
        if let Ok(mut new_value) = values.get_mut(change.for_element) {
            *new_value = change.to_value.clone();
        } else {
            if change.allow_insert {
                commands
                    .entity(change.for_element)
                    .insert(change.to_value.clone());
            } else {
                println!(
                    "DEV ERROR: Unable to change {} data to {:?} for entity {:?} \
                    because the entity does not have that type",
                    std::any::type_name::<T>(),
                    change.to_value,
                    change.for_element,
                );
            }
        }
    }
}
