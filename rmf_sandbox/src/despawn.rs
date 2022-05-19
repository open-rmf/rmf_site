use std::collections::{HashMap, HashSet};

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

#[derive(Default)]
pub struct HandleCounter(usize);

#[derive(Default)]
pub struct DespawnTracker(HashMap<usize, HashSet<Entity>>);

/// A wrapper over the `Despawn` event that tracks `Despawned` events to track when
/// the entities have all been despawned.
/// 
/// The `Despawned` event is being tracked in the `PreUpdate` stage to avoid frame
/// delays as much as possible. Do note that there may be a 1 frame delay if your
/// system also runs in the `PreUpdate` stage.
#[derive(SystemParam)]
pub struct Despawner<'w, 's> {
    despawn_writer: EventWriter<'w, 's, Despawn>,
    handle_counter: ResMut<'w, HandleCounter>,
    tracker: ResMut<'w, DespawnTracker>,
}

impl<'w, 's> Despawner<'w, 's> {
    pub fn despawn<I: IntoIterator<Item = Entity>>(&mut self, entities: I) -> usize {
        let handle = self.handle_counter.0;
        self.handle_counter.0 += 1;
        self.tracker.0.insert(handle, HashSet::new());
        let pending = self.tracker.0.get_mut(&handle).unwrap();
        for e in entities {
            pending.insert(e);
            self.despawn_writer.send(Despawn(e));
        }
        handle
    }

    pub fn is_pending(&self, handle: usize) -> bool {
        self.tracker.0.contains_key(&handle)
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

fn despawn_tracker_system(
    mut tracker: ResMut<DespawnTracker>,
    mut despawned: EventReader<Despawned>,
) {
    for e in despawned.iter() {
        for entities in tracker.0.values_mut() {
            entities.remove(&e.0);
        }
    }
    tracker.0.retain(|_, v| v.len() > 0);
}

pub struct DespawnPlugin;

impl Plugin for DespawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Despawn>()
            .add_event::<Despawned>()
            .init_resource::<HandleCounter>()
            .init_resource::<DespawnTracker>()
            .add_system_to_stage(CoreStage::PreUpdate, despawn_tracker_system)
            .add_system_to_stage(CoreStage::PostUpdate, despawn_system);
    }
}
