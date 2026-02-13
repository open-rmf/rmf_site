/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use crate::site::{AssetSource, IsStatic, ModelProperty, Scale};
use bevy::{
    ecs::{component::ComponentId, system::EntityCommands},
    prelude::*,
};
use std::collections::HashMap;

/// Function that inserts a default property into an entity
pub type InsertModelPropertyFn = fn(EntityCommands);

pub fn get_insert_model_property_fn<T: Component + Default>() -> InsertModelPropertyFn {
    |mut e_commands| {
        e_commands.insert(T::default());
    }
}

/// Function that removes a property, if it exists, from an entity
pub type RemoveModelPropertyFn = fn(EntityCommands);

pub fn get_remove_model_property_fn<T: Component + Default>() -> RemoveModelPropertyFn {
    |mut e_commands| {
        e_commands.remove::<T>();
    }
}

/// This resource keeps track of all the properties that can be configured for a model description.
#[derive(Resource)]
pub struct ModelPropertyData {
    pub required: HashMap<ComponentId, (String, InsertModelPropertyFn, RemoveModelPropertyFn)>,
    pub optional: HashMap<ComponentId, (String, InsertModelPropertyFn, RemoveModelPropertyFn)>,
}

impl FromWorld for ModelPropertyData {
    fn from_world(world: &mut World) -> Self {
        let mut required = HashMap::new();
        world.register_component::<ModelProperty<AssetSource>>();
        required.insert(
            world
                .components()
                .component_id::<ModelProperty<AssetSource>>()
                .unwrap(),
            (
                "Asset Source".to_string(),
                get_insert_model_property_fn::<ModelProperty<AssetSource>>(),
                get_remove_model_property_fn::<ModelProperty<AssetSource>>(),
            ),
        );
        world.register_component::<ModelProperty<Scale>>();
        required.insert(
            world
                .components()
                .component_id::<ModelProperty<Scale>>()
                .unwrap(),
            (
                "Scale".to_string(),
                get_insert_model_property_fn::<ModelProperty<Scale>>(),
                get_remove_model_property_fn::<ModelProperty<Scale>>(),
            ),
        );
        world.register_component::<ModelProperty<IsStatic>>();
        required.insert(
            world
                .components()
                .component_id::<ModelProperty<IsStatic>>()
                .unwrap(),
            (
                "Is Static".to_string(),
                get_insert_model_property_fn::<IsStatic>(),
                get_remove_model_property_fn::<IsStatic>(),
            ),
        );
        let optional = HashMap::new();
        Self { required, optional }
    }
}
