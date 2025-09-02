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

use crate::{
    interaction::{spawn_place_object_2d_workflow, PlaceObject2d},
    site::{CurrentLevel, ModelInstance},
};
use bevy::{
    app::{App, Plugin},
    ecs::{
        prelude::*,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use bevy_impulse::{testing::Resource, Service};
use rmf_site_picking::{RunSelector, SelectionNodeResult, SelectorInput};
use tracing::warn;

#[derive(Default)]
pub struct ObjectPlacementPlugin {}

impl Plugin for ObjectPlacementPlugin {
    fn build(&self, app: &mut App) {
        let services = ObjectPlacementServices::from_app(app);
        app.insert_resource(services);
    }
}

#[derive(Resource, Clone, Copy)]
pub struct ObjectPlacementServices {
    pub place_object_2d: Service<Option<Entity>, ()>,
    pub find_placement_2d: Service<(), Transform>,
    pub on_key_code_2d: Service<KeyCode, SelectionNodeResult>,
}

impl ObjectPlacementServices {
    pub fn from_app(app: &mut App) -> Self {
        spawn_place_object_2d_workflow(app)
    }
}

#[derive(SystemParam)]
pub struct ObjectPlacement<'w, 's> {
    pub services: Res<'w, ObjectPlacementServices>,
    pub commands: Commands<'w, 's>,
    current_level: Res<'w, CurrentLevel>,
}

impl<'w, 's> ObjectPlacement<'w, 's> {
    pub fn place_object_2d(&mut self, object: ModelInstance<Entity>) {
        let Some(level) = self.current_level.0 else {
            warn!("Unble to create [object:?] outside a level");
            return;
        };
        let state = self
            .commands
            .spawn(SelectorInput(PlaceObject2d { object, level }))
            .id();
        self.send(RunSelector {
            selector: self.services.place_object_2d,
            input: Some(state),
        });
    }

    fn send(&mut self, run: RunSelector) {
        self.commands.queue(move |world: &mut World| {
            world.send_event(run);
        });
    }
}

/// Trait to be implemented to allow placing models with commands
pub trait ObjectPlacementExt<'w, 's> {
    fn place_object_2d(&mut self, object: ModelInstance<Entity>);
}

impl<'w, 's> ObjectPlacementExt<'w, 's> for Commands<'w, 's> {
    fn place_object_2d(&mut self, object: ModelInstance<Entity>) {
        self.queue(ObjectPlaceCommand(object));
    }
}

#[derive(Deref, DerefMut)]
pub struct ObjectPlaceCommand(ModelInstance<Entity>);

impl Command for ObjectPlaceCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<ObjectPlacement> = SystemState::new(world);
        let mut placement = system_state.get_mut(world);
        placement.place_object_2d(self.0);
        system_state.apply(world);
    }
}
