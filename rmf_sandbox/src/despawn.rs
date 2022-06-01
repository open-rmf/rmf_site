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
use bevy::prelude::*;

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

pub struct DespawnPlugin;

impl Plugin for DespawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Despawn>()
            .add_system_to_stage(CoreStage::PostUpdate, despawn_system);
    }
}
