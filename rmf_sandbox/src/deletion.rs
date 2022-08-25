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

use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct DeleteElement(pub Entity);

// TODO(MXG): Use this module to implement the deletion buffer. The role of the
// deletion buffer will be to preserve deleted entities so that they can be
// easily restored if the user wants to undo the deletion.

/// This plugins tries to solve 2 problems when it comes to despawning entities in bevy.
/// 1. Commands are ran at the end of a stage, so it is possible to queue a command that
/// modifies an entity AFTER a despawn command, this will cause a panic in bevy.
/// 2. Some plugins that store entities to be used later may fail to check for the existence
/// of those entities before working their magic, this causes bevy to panic if the entity
/// has already despawned. Particularly, the glb scene loader is guilty of that.
///
/// This plugin offers 2 ways to despawn entities, by using the `Despawn` event or
/// adding `PendingDespawn` to the entity. In order to avoid "use after despawn" of entities,
/// you can add a `DespawnBlocker` component to an entity.

/// Event to despawn an entity. This is a simple wrapper to adding `PendingDespawn` to an entity.
pub struct Despawn(pub Entity);

/// Components tagged with this will not be despawned.
#[derive(Component)]
pub struct DespawnBlocker;

/// Components are tagged with this when they are about to be despawned.
#[derive(Component)]
pub struct PendingDespawn;

fn despawn_system(
    mut commands: Commands,
    mut despawn_reader: EventReader<Despawn>,
    despawn_blocker: Query<&DespawnBlocker>,
    pending_despawn: Query<Entity, With<PendingDespawn>>,
) {
    for e in despawn_reader.iter() {
        if !despawn_blocker.contains(e.0) {
            // can despawn immediately if there is no blockers.
            commands.entity(e.0).despawn_recursive();
        } else {
            // mark for pending despawns.
            commands.entity(e.0).insert(PendingDespawn);
        }
    }

    for e in pending_despawn.iter() {
        if !despawn_blocker.contains(e) {
            commands.entity(e).despawn_recursive();
        }
    }
}

pub struct DeletionPlugin;

impl Plugin for DeletionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<DeleteElement>()
            .add_event::<Despawn>()
            .add_system_to_stage(CoreStage::PostUpdate, despawn_system);
    }
}
