use std::path::PathBuf;

use bevy::{ecs::event::Events, prelude::*};

use crate::{building_map::BuildingMap, crowd_sim::CrowdSim, spawner::SiteMapRoot};

pub struct SaveLoadPlugin;

pub struct SaveMap(pub PathBuf);

fn save(world: &mut World) {
    let mut save_events = world.resource_mut::<Events<SaveMap>>();
    // if there are multiple save events for whatever reason, just process the last event.
    let path = match save_events.drain().last() {
        Some(SaveMap(path)) => path,
        None => return,
    };

    println!("Saving to {}", path.to_str().unwrap());
    let root_entity = world.resource::<SiteMapRoot>().0;
    let crowd_sim = world.entity(root_entity).get::<CrowdSim>().unwrap();
    let map = BuildingMap {
        crowd_sim: crowd_sim.clone(),
        ..default()
    };
    let f = std::fs::File::create(path).unwrap();
    serde_yaml::to_writer(f, &map).unwrap();
}

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveMap>()
            .add_system(save.exclusive_system());
    }
}
