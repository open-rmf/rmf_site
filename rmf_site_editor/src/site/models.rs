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

use rmf_site_format::{AssetSource, Scale};
use bevy::{
    prelude::*,
    ecs::system::{Command, EntityCommands},
};

#[derive(Component, Clone, Copy)]
pub struct InScenario(pub Entity);

#[derive(Component, Clone, Copy)]
pub struct ModelSource(Entity);
impl ModelSource {
    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Clone)]
pub struct ModelInstances(Vec<Entity>);
impl ModelInstances {
    pub fn iter(&self) -> impl Iterator<Item=&Entity> {
        self.0.iter()
    }
}

#[derive(Clone, Copy)]
struct ChangeModelSource {
    instance: Entity,
    source: Entity,
}

impl Command for ChangeModelSource {
    fn write(self, world: &mut World) {
        let mut previous_source = None;
        if let Some(mut e) = world.get_entity_mut(self.instance) {
            if let Some(previous) = e.get::<ModelSource>() {
                if previous.0 == self.source {
                    // No need to change anything
                    return;
                }

                previous_source = Some(previous.0);
            }
            e.insert(ModelSource(self.source));
        } else {
            error!(
                "Cannot change model source of instance {:?} because it no \
                longer exists",
                self.instance,
            );
            return;
        }

        if let Some(mut e) = world.get_entity_mut(self.source) {
            if let Some(mut instances) = e.get_mut::<ModelInstances>() {
                instances.0.push(self.instance);
            } else {
                e.insert(ModelInstances(vec![self.instance]));
            }
        } else {
            error!(
                "Cannot change model source of instance {:?} to {:?} because \
                that source no longer exists",
                self.instance,
                self.source,
            );

            if let (Some(mut e), Some(prev)) = (
                world.get_entity_mut(self.instance),
                previous_source
            ) {
                e.insert(ModelSource(prev));
            }
            return;
        }

        if let Some(previous) = previous_source {
            if let Some(mut e) = world.get_entity_mut(previous) {
                if let Some(mut instances) = e.get_mut::<ModelInstances>() {
                    instances.0.retain(|e| *e != self.instance);
                }
            }
        }

        world.send_event(self);
    }
}

pub trait SetModelSourceExt {
    fn set_model_source(&mut self, new_source: Entity) -> &mut Self;
}

impl<'w, 's, 'a> SetModelSourceExt for EntityCommands<'w, 's, 'a> {
    fn set_model_source(&mut self, source: Entity) -> &mut Self {
        let instance = self.id();
        self.commands().add(ChangeModelSource { instance, source });
        self
    }
}

fn handle_changed_model_sources(
    mut commands: Commands,
    mut changed_instances: EventReader<ChangeModelSource>,
    sources: Query<(&AssetSource, &Scale)>,
    changed_sources: Query<
        (&ModelInstances, &AssetSource, &Scale),
        Or<(Changed<AssetSource>, Changed<Scale>)>,
    >,
) {
    for change in changed_instances.iter() {
        let Some(mut instance) = commands.get_entity(change.source) else { continue };
        let Ok((source, scale)) = sources.get(change.source) else { continue };
        instance
            .insert(source.clone())
            .insert(scale.clone());
    }

    for (instances, source, scale) in &changed_sources {
        for instance in instances.iter() {
            let Some(mut instance) = commands.get_entity(*instance) else { continue };
            instance
                .insert(source.clone())
                .insert(scale.clone());
        }
    }
}

pub struct ModelSourcePlugin;
impl Plugin for ModelSourcePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<ChangeModelSource>()
            .add_system(handle_changed_model_sources);
    }
}
