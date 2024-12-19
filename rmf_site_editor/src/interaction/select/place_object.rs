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

use crate::{interaction::select::*, site::ModelInstance};
use bevy::ecs::system::SystemParam;

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
    pub place_object_3d: Service<Option<Entity>, ()>,
    pub replace_parent_3d: Service<Option<Entity>, ()>,
    pub hover_service_object_3d: Service<(), (), Hover>,
}

impl ObjectPlacementServices {
    pub fn from_app(app: &mut App) -> Self {
        let hover_service_object_3d = app.spawn_continuous_service(
            Update,
            hover_service::<PlaceObject3dFilter>
                .configure(|config: SystemConfigs| config.in_set(SelectionServiceStages::Hover)),
        );
        let place_object_2d = spawn_place_object_2d_workflow(app);
        let place_object_3d = spawn_place_object_3d_workflow(hover_service_object_3d, app);
        let replace_parent_3d = spawn_replace_parent_3d_workflow(hover_service_object_3d, app);
        Self {
            place_object_2d,
            place_object_3d,
            replace_parent_3d,
            hover_service_object_3d,
        }
    }
}

#[derive(SystemParam)]
pub struct ObjectPlacement<'w, 's> {
    pub services: Res<'w, ObjectPlacementServices>,
    pub commands: Commands<'w, 's>,
}

impl<'w, 's> ObjectPlacement<'w, 's> {
    pub fn place_object_2d(&mut self, object: ModelInstance<Entity>, level: Entity) {
        let state = self
            .commands
            .spawn(SelectorInput(PlaceObject2d { object, level }))
            .id();
        self.send(RunSelector {
            selector: self.services.place_object_2d,
            input: Some(state),
        });
    }

    pub fn place_object_3d(
        &mut self,
        object: PlaceableObject,
        parent: Option<Entity>,
        workspace: Entity,
    ) {
        let state = self
            .commands
            .spawn(SelectorInput(PlaceObject3d {
                object,
                parent,
                workspace,
            }))
            .id();
        self.send(RunSelector {
            selector: self.services.place_object_3d,
            input: Some(state),
        });
    }

    pub fn replace_parent_3d(&mut self, object: Entity, workspace: Entity) {
        let state = self
            .commands
            .spawn(SelectorInput(ReplaceParent3d { object, workspace }))
            .id();
        self.send(RunSelector {
            selector: self.services.replace_parent_3d,
            input: Some(state),
        });
    }

    fn send(&mut self, run: RunSelector) {
        self.commands.add(move |world: &mut World| {
            world.send_event(run);
        });
    }
}
