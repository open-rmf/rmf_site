use std::collections::HashSet;

use bevy::{ecs::system::SystemParam, prelude::*};

/// Event to despawn an entity.
///
/// Because commands are executed at the end of the stage "at the same time",
/// it creates race conditions when inserting components and despawning entities. In order to
/// avoid the race conditon, despawning should be ran in the PostUpdate stage, this event
/// helps to do that.
pub struct Despawn(pub Entity);

#[derive(Component, Default)]
/// Defer the despawn of an entity until this component is removed.
pub struct DespawnBlocker();

/// Sent when an entity has despawned.
pub struct Despawned(pub Entity);

#[derive(SystemParam)]
pub struct DespawnTracker<'w, 's> {
    despawn_writer: EventWriter<'w, 's, Despawn>,
    despawned_reader: EventReader<'w, 's, Despawned>,
    pub pending: Local<'s, HashSet<Entity>>,
}

impl<'w, 's> DespawnTracker<'w, 's> {
    /// This MUST be called every frame to ensure that despawned events are not
    /// missed.
    /// TODO: Anyway to auto call this every frame?
    pub fn tick(&mut self) {
        for e in self.despawned_reader.iter() {
            self.pending.remove(&e.0);
        }
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.despawn_writer.send(Despawn(entity));
        self.pending.insert(entity);
    }
}

fn despawn_system(
    mut commands: Commands,
    mut despawn_reader: EventReader<Despawn>,
    mut to_despawn: Local<HashSet<Entity>>,
    despawn_blocker: Query<&DespawnBlocker>,
    mut despawned: EventWriter<Despawned>,
) {
    for e in despawn_reader.iter() {
        to_despawn.insert(e.0);
    }

    let mut done: Vec<Entity> = Vec::new();
    for e in to_despawn.iter() {
        if despawn_blocker.contains(*e) {
            continue;
        }
        commands.entity(*e).despawn_recursive();
        despawned.send(Despawned(*e));
        done.push(*e);
        println!("despawned entity {},{}", e.id(), e.generation());
    }
    for e in done {
        to_despawn.remove(&e);
    }
}

pub struct DespawnPlugin;

impl Plugin for DespawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Despawn>()
            .add_event::<Despawned>()
            .add_system_to_stage(CoreStage::PostUpdate, despawn_system);
    }
}
