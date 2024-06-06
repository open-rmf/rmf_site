/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use bevy::prelude::{Entity, Resource, World, FromWorld, Plugin, App};
use bevy_infinite_grid::InfiniteGridPlugin;

use crate::shapes::make_infinite_grid;

#[derive(Resource)]
pub struct FloorGrid {
    entity: Entity,
}

impl FloorGrid {
    pub fn get(&self) -> Entity {
        self.entity
    }
}

impl FromWorld for FloorGrid {
    fn from_world(world: &mut World) -> Self {
        let entity = world.spawn(make_infinite_grid(1.0, 100.0, None)).id();
        Self { entity }
    }
}

pub struct FloorGridPlugin;

impl Plugin for FloorGridPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(InfiniteGridPlugin)
            .init_resource::<FloorGrid>();
    }
}
