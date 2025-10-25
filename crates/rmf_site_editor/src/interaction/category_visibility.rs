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

use bevy::prelude::*;

use crate::interaction::InteractionState;
use std::fmt::Debug;

#[derive(Resource, Clone, Debug)]
pub struct CategoryVisibility<T: Component + Clone + Debug>(pub bool, std::marker::PhantomData<T>);

impl<T: Component + Clone + Debug> CategoryVisibility<T> {
    pub fn visible(visible: bool) -> Self {
        Self(visible, Default::default())
    }
}

#[derive(Event)]
pub struct SetCategoryVisibility<T: Component + Clone + Debug>(
    pub bool,
    std::marker::PhantomData<T>,
);

impl<T: Component + Clone + Debug> From<bool> for SetCategoryVisibility<T> {
    fn from(val: bool) -> Self {
        Self(val, Default::default())
    }
}

pub struct CategoryVisibilityPlugin<T: Component + Clone + Debug> {
    visible: bool,
    _ignore: std::marker::PhantomData<T>,
}

impl<T: Component + Clone + Debug> CategoryVisibilityPlugin<T> {
    pub fn visible(visible: bool) -> Self {
        Self {
            visible,
            _ignore: Default::default(),
        }
    }
}

impl<T: Component + Clone + Debug> Plugin for CategoryVisibilityPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<SetCategoryVisibility<T>>()
            .insert_resource(CategoryVisibility::<T>::visible(self.visible))
            // TODO(luca) Check that this is at the right stage
            .add_systems(
                Update,
                set_category_visibility::<T>.run_if(in_state(InteractionState::Enable)),
            );
    }
}

fn set_category_visibility<T: Component + Clone + Debug>(
    mut events: EventReader<SetCategoryVisibility<T>>,
    mut category_visibility: ResMut<CategoryVisibility<T>>,
    mut visibilities: Query<&mut Visibility, With<T>>,
) {
    if let Some(visibility_event) = events.read().last() {
        if visibility_event.0 != category_visibility.0 {
            for mut vis in &mut visibilities {
                *vis = if visibility_event.0 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            category_visibility.0 = visibility_event.0;
        }
    }
}
