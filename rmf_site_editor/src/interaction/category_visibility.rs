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

use crate::site::SiteState;
use std::fmt::Debug;

#[derive(Resource, Clone, Debug)]
pub struct CategoryVisibility<T: Component + Clone + Debug>(pub bool, std::marker::PhantomData<T>);

// Initialized to true (visible) by default
impl<T: Component + Clone + Debug> Default for CategoryVisibility<T> {
    fn default() -> Self {
        Self(true, Default::default())
    }
}

pub struct SetCategoryVisibility<T: Component + Clone + Debug>(
    pub bool,
    std::marker::PhantomData<T>,
);

impl<T: Component + Clone + Debug> From<bool> for SetCategoryVisibility<T> {
    fn from(val: bool) -> Self {
        Self(val, Default::default())
    }
}

#[derive(Default)]
pub struct CategoryVisibilityPlugin<T: Component + Clone + Debug> {
    _ignore: std::marker::PhantomData<T>,
}

impl<T: Component + Clone + Debug> Plugin for CategoryVisibilityPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<SetCategoryVisibility<T>>()
            .init_resource::<CategoryVisibility<T>>()
            // TODO(luca) Check that this is at the right stage
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::on_update(SiteState::Display)
                    .with_system(set_category_visibility::<T>)
                    .with_system(set_category_visibility_for_new_entity::<T>),
            );
    }
}

fn set_category_visibility<T: Component + Clone + Debug>(
    mut events: EventReader<SetCategoryVisibility<T>>,
    mut category_visibility: ResMut<CategoryVisibility<T>>,
    mut visibilities: Query<&mut Visibility, With<T>>,
) {
    if let Some(visibility_event) = events.iter().last() {
        for mut vis in &mut visibilities {
            vis.is_visible = visibility_event.0;
        }
        category_visibility.0 = visibility_event.0;
    }
}

fn set_category_visibility_for_new_entity<T: Component + Clone + Debug>(
    category_visibility: Res<CategoryVisibility<T>>,
    mut visibilities: Query<&mut Visibility, Added<T>>,
) {
    for mut vis in &mut visibilities {
        vis.is_visible = category_visibility.0;
    }
}
