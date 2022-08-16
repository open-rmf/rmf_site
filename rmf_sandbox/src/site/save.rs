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

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};
use bevy::{
    ecs::{event::Events, system::SystemState},
    prelude::*,
};
use thiserror::Error as ThisError;

use rmf_site_format::{
    Site,
    SiteProperties,
    Dock,
    Door,
    Drawing,
    Fiducial,
    Floor,
    Lane,
    Level,
    LevelProperties,
    Lift,
    Light,
    Location,
    Measurement,
    Model,
    NavGraph,
    NavGraphProperties,
    PhysicalCamera,
    Wall,
};

use crate::site::*;

pub struct SaveSite {
    pub site: Entity,
    pub to_file: PathBuf
}

#[derive(Component, Debug, Clone, Copy)]
struct SiteID(u32);

#[derive(ThisError, Debug, Clone)]
pub enum SiteGenerationError {
    #[error("the specified entity [{0:?}] does not refer to a site")]
    InvalidSiteEntity(Entity),
    #[error("an object has a reference to an anchor that does not exist")]
    BrokenAnchorReference(Entity),
    #[error("an object has a reference to a level that does not exist")]
    BrokenLevelReference(Entity),
    #[error("level {level} is being referenced by an object in site {site} but does not belong to that site")]
    InvalidLevelReference{site: Entity, level: Entity},
    #[error("anchor {anchor} is being referenced for level {level} but does not belong to that level")]
    InvalidAnchorReference{level: Entity, anchor: Entity},
}

/// Look through all the elements that we will be saving and assign a SiteID
/// component to any elements that do not have one already.
fn assign_site_ids(
    world: &mut World,
    site: Entity,
) -> Result<(), SiteGenerationError> {
    let mut state: SystemState<(
        Query<Entity, Or<(
            With<Anchor>,
            With<Door<Entity>>,
            With<Drawing>,
            With<Fiducial<Entity>>,
            With<Floor<Entity>>,
            With<Light>,
            With<Measurement<Entity>>,
            With<Model>,
            With<PhysicalCamera>,
            With<Wall<Entity>>
        )>>,
        Query<Entity, Or<(
            With<Lane<Entity>>,
            With<Location<Entity>>,
        )>>,
        Query<Entity, With<LevelProperties>>,
        Query<Entity, With<NavGraphProperties>>,
        Query<Entity, With<Lift<Entity>>>,
        Query<&mut NextSiteID>,
        Query<&SiteID>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (
        level_children,
        nav_graph_children,
        levels,
        nav_graphs,
        lifts,
        mut sites,
        site_ids,
        children,
    ) = state.get_mut(world);

    let next_site_id = sites.get_mut(site)
        .map_err(|_| SiteGenerationError::InvalidSiteEntity(site))?;

    let site_children = match children.get(site) {
        Ok(children) => children,
        Err(_) => {
            // The site seems to have no children at all. That's suspicious but
            // not impossible if the site is completely empty. In that case
            // there is no need to assign any SiteIDs
            return Ok(());
        }
    };

    let next_id = move || {
        let next = next_site_id.0;
        next_site_id.0 += 1;
        return next;
    };

    for site_child in site_children {
        if let Ok(level) = levels.get(*site_child) {
            if !site_ids.contains(level) {
                world.entity_mut(level).insert(SiteID(next_id()));
            }

            if let Ok(children) = children.get(level) {
                for child in children {
                    if level_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            world.entity_mut(*child).insert(SiteID(next_id()));
                        }
                    }
                }
            }
        }

        if let Ok(nav_graph) = nav_graphs.get(*site_child) {
            if !site_id.contains(nav_graph) {
                world.entity_mut(nav_graph).insert(SiteID(next_id()));
            }

            if let Ok(children) = children.get(nav_graph) {
                for child in children {
                    if nav_graph_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            world.entity_mut(*child).insert(SiteID(next_id()));
                        }
                    }
                }
            }
        }

        if let Ok(lift) = lifts.get(*site_child) {
            if !site_id.contains(lift) {
                world.entity_mut(lift).insert(SiteID(next_id()));
            }

            if let Ok(children) = children.get(lift) {
                for child in children {
                    if level_children.contains(*child) {
                        if !site_ids.contain(*child) {
                            world.entity_mut(*child).insert(SiteID(next_id()));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn generate_levels(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, Level>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&Anchor, &SiteID, &Parent)>,
        Query<(&Door<Entity>, &SiteID, &Parent)>,
        Query<(&Drawing, &SiteID, &Parent)>,
        Query<(&Fiducial<Entity>, &SiteID, &Parent)>,
        Query<(&Floor<Entity>, &SiteID, &Parent)>,
        Query<(&Light, &SiteID, &Parent)>,
        Query<(&Measurement<Entity>, &SiteID, &Parent)>,
        Query<(&Model, &SiteID, &Parent)>,
        Query<(&PhysicalCamera, &SiteID, &Parent)>,
        Query<(&Wall<Entity>, &SiteID, &Parent)>,
        Query<(&LevelProperties, &SiteID, &Parent)>,
    )> = SystemState::new(world);

    let (
        q_anchors,
        q_doors,
        q_drawings,
        q_fiducials,
        q_floors,
        q_lights,
        q_measurements,
        q_models,
        q_physical_cameras,
        q_walls,
        q_levels,
    ) = state.get(world);

    let mut levels = BTreeMap::new();
    for (properties, level_id, parent) in &q_levels {
        if parent.get() == site {
            levels.insert(level_id.0, Level{
                properties: properties.clone(),
                ..default()
            });
        }
    }

    let get_anchor_id = |entity| {
        let (_, site_id, _) = q_anchors.get(entity).map_err(
            |_| SiteGenerationError::BrokenAnchorReference(entity)
        )?;
        Ok(site_id.0)
    };

    let get_anchor_id_pair = |(left_entity, right_entity)| {
        let left = get_anchor_id(left_entity)?;
        let right = get_anchor_id(right_entity)?;
        Ok((left, right))
    };

    let get_anchor_id_vec = |entities: &Vec<Entity>| {
        let mut anchor_ids = Vec::new();
        anchor_ids.reserve(entities.len());
        for entity in entities {
            let id = get_anchor_id(entity)?;
            anchor_ids.push(id);
        }
        Ok(anchor_ids)
    };

    for (anchor, id, parent) in &q_anchors {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.anchors.insert(id.0, anchor.into());
            }
        }
    }

    for (door, id, parent) in &q_doors {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_pair(door.anchors)?;
                level.doors.insert(id.0, door.to_u32(anchors));
            }
        }
    }

    for (drawing, id, parent) in &q_drawings {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level.id.0) {
                level.drawings.insert(id.0, drawing.clone());
            }
        }
    }

    for (fiducial, id, parent) in &q_fiducials {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchor = get_anchor_id(fiducial.anchor)?;
                level.fiducials.insert(id.0, fiducial.to_u32(anchor));
            }
        }
    }

    for (floor, id, parent) in &q_floors {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_vec(&floor.anchors)?;
                level.floors.insert(id.0, floor.to_u32(anchors));
            }
        }
    }

    for (light, id, parent) in &q_lights {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.lights.insert(id.0, light.clone());
            }
        }
    }

    for (measurement, id, parent) in &q_measurements {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_pair(&measurement.anchors)?;
                level.measurements.insert(id.0, measurement.to_u32(anchors));
            }
        }
    }

    for (model, id, parent) in &q_models {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.models.insert(id.0, model.clone());
            }
        }
    }

    for (physical_camera, id, parent) in &q_physical_cameras {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.physical_cameras.insert(id.0, physical_camera.clone());
            }
        }
    }

    for (wall, id, parent) in &q_walls {
        if let Ok((_, level_id, _)) = q_levels.get(parent.id()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_pair(wall.anchors)?;
                level.walls.insert(id.0, wall.to_u32(anchors));
            }
        }
    }

    return Ok(levels);
}

fn generate_lifts(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, Lift<u32>>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&Anchor, &SiteID)>,
        Query<&SiteID, With<LevelProperties>>,
        Query<(Entity, &Lift<Entity>, &SiteID, &Parent)>,
        Query<&Parent>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (
        q_anchors,
        q_levels,
        q_lifts,
        q_parents,
        q_children,
    ) = state.get(world);

    let mut lifts = BTreeMap::new();

    let get_anchor_id = |entity| {
        let site_id = q_anchors.get(entity).map_err(
            |_| SiteGenerationError::BrokenAnchorReference(entity)
        )?;
        Ok(site_id.0)
    };

    let get_level_id = |entity| {
        let site_id = q_levels.get(entity).map_err(
            |_| SiteGenerationError::BrokenLevelReference(entity)
        )?;
        Ok(site_id.0)
    };

    let get_anchor_id_pair = |(left_entity, right_entity)| {
        let left = get_anchor_id(left_entity)?;
        let right = get_anchor_id(right_entity)?;
        Ok((left, right))
    };

    let confirm_anchor_level = |level, anchor| {
        if let Ok(parent) = q_parents.get(anchor) {
            if parent.get() == level {
                return Ok(())
            }
        }

        Err(SiteGenerationError::InvalidAnchorReference{level, anchor})
    };

    let confirm_anchors_level = |level, (left, right)| {
        confirm_anchor_level(level, left)?;
        confirm_anchor_level(level, right)?;
        if let Ok(parent) = q_parents.get(level) {
            if parent.get() == site {
                Ok(())
            }
        }
        Err(SiteGenerationError::InvalidLevelReference{site, level})
    };

    let get_corrections_map = |entity_map: &BTreeMap<Entity, (Entity, Entity)>| {
        let mut id_map = BTreeMap::new();
        for (level, anchors) in entity_map {
            confirm_anchors_level(*level, *anchors)?;
            let anchors = get_anchor_id_pair(*anchors)?;
            let level = get_level_id(*level)?;
            id_map.insert(level, anchors);
        }
        Ok(id_map)
    };

    for (entity, lift, id, parent) in &q_lifts {
        if parent.get() != site {
            continue;
        }

        if let Ok(canon_level) = q_parents.get(lift.reference_anchors.0) {
            confirm_anchor_level(*canon_level, lift.reference_anchors)?;
        } else {
            return Err(SiteGenerationError::BrokenAnchorReference(lift.reference_anchors.0));
        }

        let mut cabin_anchors = BTreeMap::new();
        if let Ok(children) = q_children.get(entity) {
            for child in children {
                if let Ok((anchor, site_id)) = q_anchors.get(*child) {
                    cabin_anchors.insert(site_id.0, anchor.into());
                }
            }
        }

        let reference_anchors = get_anchor_id_pair(lift.reference_anchors)?;
        let corrections = get_corrections_map(&lift.corrections)?;

        lifts.insert(id.0, lift.to_u32(reference_anchors, corrections, cabin_anchors));
    }

    return Ok(lifts);
}

fn generate_nav_graphs(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, NavGraph>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&NavGraphProperties, &SiteID, &Parent, Option<&Children>)>,
        Query<(&Lane<Entity>, &SiteID)>,
        Query<(&Location<Entity>, &SiteID)>,
        Query<&SiteID, With<Anchor>>,
    )> = SystemState::new(world);

    let (
        q_nav_graphs,
        q_lanes,
        q_locations,
        q_anchors,
    ) = state.get_mut(world);

    let get_anchor_id = |entity| {
        let site_id = q_anchors.get(entity).map_err(
            |_| SiteGenerationError::BrokenAnchorReference(entity)
        )?;
        Ok(site_id.0)
    };

    let get_anchor_id_pair = |(left, right)| {
        let left = get_anchor_id(left)?;
        let right = get_anchor_id(right)?;
        Ok((left, right))
    };

    let mut nav_graphs = BTreeMap::new();
    for (properties, id, parent, children) in &q_nav_graphs {
        if parent.get() != site {
            continue;
        }

        let mut lanes = BTreeMap::new();
        let mut locations = BTreeMap::new();
        if let Some(children) = children {
            for child in children {
                if let Ok((lane, lane_id)) = q_lanes.get(*child) {
                    let anchors = get_anchor_id_pair(lane.anchors);
                    lanes.insert(lane_id.0, lane.to_u32(anchors));
                }

                if let Ok((location, location_id)) = q_locations.get(*child) {
                    let anchor = get_anchor_id(location.anchor)?;
                    locations.insert(location_id.0, location.to_u32(anchor));
                }
            }
        }

        nav_graphs.insert(
            id.0,
            NavGraph{
                properties: properties.clone(),
                lanes,
                locations,
            }
        );
    }

    return Ok(nav_graphs);
}

pub fn generate_site(
    world: &mut World,
    site: Entity,
) -> Result<rmf_site_format::Site, SiteGenerationError> {
    assign_site_ids(world, site)?;
    let levels = generate_levels(world, site)?;
    let lifts = generate_lifts(world, site)?;
    let nav_graphs = generate_nav_graphs(world, site)?;

    let props = match world.get::<SiteProperties>(site) {
        Some(props) => props,
        None => {
            return Err(SiteGenerationError::InvalidSiteEntity(site));
        }
    };

    return Ok(Site{
        format_version: rmf_site_format::SemVer::default(),
        properties: props.clone(),
        levels,
        lifts,
        nav_graphs,
        // TODO(MXG): Parse agent information once the spec is figured out
        agents: Default::default(),
    });
}

pub fn save(world: &mut World) {
    let mut save_events = world.resource_mut::<Events<SaveSite>>();
    for save_event in save_events.drain() {
        println!("Saving to {}", save_event.to_file.to_str().unwrap());
        let f = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(err) => {
                println!("Unable to save file: {err}");
                continue;
            }
        };

        let site = match generate_site(world, save_event.site) {
            Ok(site) => site,
            Err(err) => {
                println!("Unable to compile site: {err}");
                continue;
            }
        };

        match site.to_writer(f) {
            Ok(()) => {
                println!("Save successful");
            },
            Err(err) => {
                println!("Save failed: {err}");
            }
        }
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveSite>()
            .add_system(save.exclusive_system());
    }
}
