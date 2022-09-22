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
    collections::BTreeMap,
    path::PathBuf,
};
use bevy::{
    ecs::{event::Events, system::SystemState},
    prelude::*,
};
use thiserror::Error as ThisError;

use rmf_site_format::*;
use crate::site::*;

/// The Pending component indicates that an element is not yet ready to be
/// saved to file. We will filter out these elements while assigning SiteIDs,
/// and that will prevent them from being included while collecting elements
/// into the Site data structure.
#[derive(Component, Debug, Clone, Copy)]
pub struct Pending;

pub struct SaveSite {
    pub site: Entity,
    pub to_file: Option<PathBuf>,
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
    #[error("an object has a reference to a door that does not exist")]
    BrokenDoorReference(Entity),
    #[error("level {level:?} is being referenced by an object in site {site:?} but does not belong to that site")]
    InvalidLevelReference{site: Entity, level: Entity},
    #[error("anchor {anchor:?} is being referenced for level {level:?} but does not belong to that level")]
    InvalidAnchorReference{level: Entity, anchor: Entity},
    #[error("door {door:?} is being referenced for level {level:?} but does not belong to that level")]
    InvalidDoorReference{level: Entity, door: Entity}
}

/// Look through all the elements that we will be saving and assign a SiteID
/// component to any elements that do not have one already.
fn assign_site_ids(
    world: &mut World,
    site: Entity,
) -> Result<(), SiteGenerationError> {
    let mut state: SystemState<(
        Commands,
        Query<Entity, (Or<(
            With<Anchor>,
            With<DoorType>,
            With<DrawingMarker>,
            With<FiducialMarker>,
            With<FloorMarker>,
            With<LightType>,
            With<MeasurementMarker>,
            With<ModelMarker>,
            With<PhysicalCameraProperties>,
            With<WallMarker>
        )>, Without<Pending>)>,
        Query<Entity, (Or<(
            With<LaneMarker>,
            With<LocationTags>,
        )>, Without<Pending>)>,
        Query<Entity, (With<LevelProperties>, Without<Pending>)>,
        Query<Entity, (With<NavGraphProperties>, Without<Pending>)>,
        Query<Entity, (With<LiftCabin>, Without<Pending>)>,
        Query<&mut NextSiteID>,
        Query<&SiteID>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (
        mut commands,
        level_children,
        nav_graph_children,
        levels,
        nav_graphs,
        lifts,
        mut sites,
        site_ids,
        children,
    ) = state.get_mut(world);

    let mut next_site_id = sites.get_mut(site)
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

    let mut next_id = move || {
        let next = next_site_id.0;
        next_site_id.as_mut().0 += 1;
        return next;
    };

    for site_child in site_children {
        if let Ok(level) = levels.get(*site_child) {
            if !site_ids.contains(level) {
                commands.entity(level).insert(SiteID(next_id()));
            }

            if let Ok(children) = children.get(level) {
                for child in children {
                    if level_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            commands.entity(*child).insert(SiteID(next_id()));
                        }
                    }
                }
            }
        }

        if let Ok(nav_graph) = nav_graphs.get(*site_child) {
            if !site_ids.contains(nav_graph) {
                commands.entity(nav_graph).insert(SiteID(next_id()));
            }

            if let Ok(children) = children.get(nav_graph) {
                for child in children {
                    if nav_graph_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            commands.entity(*child).insert(SiteID(next_id()));
                        }
                    }
                }
            }
        }

        if let Ok(lift) = lifts.get(*site_child) {
            if !site_ids.contains(lift) {
                commands.entity(lift).insert(SiteID(next_id()));
            }

            if let Ok(children) = children.get(lift) {
                for child in children {
                    if level_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            commands.entity(*child).insert(SiteID(next_id()));
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
        Query<(&Transform, &SiteID, &Parent), With<Anchor>>,
        Query<(&Edge<Entity>, &NameInSite, &DoorType, &SiteID, &Parent)>,
        Query<(&DrawingSource, &Pose, &SiteID, &Parent), With<DrawingMarker>>,
        Query<(&Point<Entity>, &Label, &SiteID, &Parent), With<FiducialMarker>>,
        Query<(&Path<Entity>, &Texture, &SiteID, &Parent), With<FloorMarker>>,
        Query<(&LightType, &Pose, &SiteID, &Parent)>,
        Query<(&Edge<Entity>, &Distance, &Label, &SiteID, &Parent), With<MeasurementMarker>>,
        Query<(&NameInSite, &Label, &Pose, &IsStatic, &SiteID, &Parent), With<ModelMarker>>,
        Query<(&NameInSite, &Pose, &PhysicalCameraProperties, &SiteID, &Parent)>,
        Query<(&Edge<Entity>, &Texture, &SiteID, &Parent), With<WallMarker>>,
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
            levels.insert(level_id.0, Level::new(properties.clone()));
        }
    }

    let get_anchor_id = |entity| {
        let (_, site_id, _) = q_anchors.get(entity).map_err(
            |_| SiteGenerationError::BrokenAnchorReference(entity)
        )?;
        Ok(site_id.0)
    };

    let get_anchor_id_edge = |edge: &Edge<Entity>| {
        let left = get_anchor_id(edge.left())?;
        let right = get_anchor_id(edge.right())?;
        Ok(Edge::new(left, right))
    };

    let get_anchor_id_path = |entities: &Vec<Entity>| {
        let mut anchor_ids = Vec::new();
        anchor_ids.reserve(entities.len());
        for entity in entities {
            let id = get_anchor_id(*entity)?;
            anchor_ids.push(id);
        }
        Ok(Path(anchor_ids))
    };

    for (anchor_tf, id, parent) in &q_anchors {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let p = anchor_tf.translation;
                level.anchors.insert(id.0, (p.x, p.y));
            }
        }
    }

    for (edge, name, kind, id, parent) in &q_doors {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_edge(edge)?;
                level.doors.insert(id.0, Door{
                    anchors,
                    name: name.clone(),
                    kind: kind.clone(),
                    marker: DoorMarker,
                });
            }
        }
    }

    for (source, pose, id, parent) in &q_drawings {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.drawings.insert(id.0, Drawing{
                    source: source.clone(),
                    pose: pose.clone(),
                    marker: DrawingMarker,
                });
            }
        }
    }

    for (point, label, id, parent) in &q_fiducials {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchor = Point(get_anchor_id(point.0)?);
                level.fiducials.insert(id.0, Fiducial{
                    anchor,
                    label: label.clone(),
                    marker: FiducialMarker,
                });
            }
        }
    }

    for (path, texture, id, parent) in &q_floors {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_path(&path)?;
                level.floors.insert(id.0, Floor{
                    anchors,
                    texture: texture.clone(),
                    marker: FloorMarker,
                });
            }
        }
    }

    for (kind, pose, id, parent) in &q_lights {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.lights.insert(id.0, Light{
                    pose: pose.clone(),
                    kind: kind.clone(),
                });
            }
        }
    }

    for (edge, distance, label, id, parent) in &q_measurements {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_edge(edge)?;
                level.measurements.insert(id.0, Measurement{
                    anchors,
                    distance: distance.clone(),
                    label: label.clone(),
                    marker: MeasurementMarker,
                });
            }
        }
    }

    for (name, kind, pose, is_static, id, parent) in &q_models {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.models.insert(id.0, Model{
                    name: name.clone(),
                    kind: kind.clone(),
                    pose: pose.clone(),
                    is_static: is_static.clone(),
                    marker: ModelMarker,
                });
            }
        }
    }

    for (name, pose, properties, id, parent) in &q_physical_cameras {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.physical_cameras.insert(id.0, PhysicalCamera{
                    name: name.clone(),
                    pose: pose.clone(),
                    properties: properties.clone()
                });
            }
        }
    }

    for (edge, texture, id, parent) in &q_walls {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_edge(edge)?;
                level.walls.insert(id.0, Wall{
                    anchors,
                    texture: texture.clone(),
                    marker: WallMarker,
                });
            }
        }
    }

    return Ok(levels);
}

type QueryLift<'w, 's> = Query<'w, 's, (
    Entity, &'static NameInSite, &'static Edge<Entity>, &'static LiftCabin,
    &'static LevelDoors<Entity>, &'static Corrections<Entity>,
    &'static IsStatic, &'static SiteID, &'static Parent
)>;

fn generate_lifts(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, Lift<u32>>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&Transform, &SiteID), With<Anchor>>,
        Query<&SiteID, With<DoorType>>,
        Query<&SiteID, With<LevelProperties>>,
        QueryLift,
        Query<&Parent>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (
        q_anchors,
        q_doors,
        q_levels,
        q_lifts,
        q_parents,
        q_children,
    ) = state.get(world);

    let mut lifts = BTreeMap::new();

    let get_anchor_id = |entity| {
        let (_, site_id) = q_anchors.get(entity).map_err(
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

    let get_door_id = |entity| {
        let site_id = q_doors.get(entity).map_err(
            |_| SiteGenerationError::BrokenDoorReference(entity)
        )?;
        Ok(site_id.0)
    };

    let get_anchor_id_edge = |edge: &Edge<Entity>| {
        let left = get_anchor_id(edge.left())?;
        let right = get_anchor_id(edge.right())?;
        Ok(Edge::new(left, right))
    };

    let confirm_entity_level = |level, child| {
        if let Ok(parent) = q_parents.get(child) {
            if parent.get() == level {
                return true;
            }
        }

        return false;
    };

    let confirm_level_on_site = |level| {
        if let Ok(parent) = q_parents.get(level) {
            if parent.get() == site {
                return Ok(());
            }
        }
        Err(SiteGenerationError::InvalidLevelReference{site, level})
    };

    let confirm_anchor_level = |level, anchor| {
        if confirm_entity_level(level, anchor) {
            return Ok(());
        }

        Err(SiteGenerationError::InvalidAnchorReference{level, anchor})
    };

    let confirm_anchors_level = |level, edge: &Edge<Entity>| {
        confirm_anchor_level(level, edge.left())?;
        confirm_anchor_level(level, edge.right())?;
        confirm_level_on_site(level)?;
        Ok(())
    };

    let confirm_door_level = |level, door| {
        if confirm_entity_level(level, door) {
            confirm_level_on_site(level)?;
            return Ok(());
        }

        Err(SiteGenerationError::InvalidDoorReference{level, door})
    };

    let get_corrections_map = |entity_map: &BTreeMap<Entity, Edge<Entity>>| {
        let mut id_map = BTreeMap::new();
        for (level, anchors) in entity_map {
            confirm_anchors_level(*level, anchors)?;
            let anchors = get_anchor_id_edge(anchors)?;
            let level = get_level_id(*level)?;
            id_map.insert(level, anchors);
        }
        Ok(Corrections(id_map))
    };

    for (entity, name, edge, cabin, e_level_doors, corrections, is_static, id, parent) in &q_lifts {
        if parent.get() != site {
            continue;
        }

        if let Ok(canon_level) = q_parents.get(edge.left()) {
            confirm_anchors_level(canon_level.get(), edge)?;
        } else {
            return Err(SiteGenerationError::BrokenAnchorReference(edge.left()));
        }

        let mut cabin_anchors = BTreeMap::new();
        if let Ok(children) = q_children.get(entity) {
            for child in children {
                if let Ok((anchor_tf, site_id)) = q_anchors.get(*child) {
                    let p = anchor_tf.translation;
                    cabin_anchors.insert(site_id.0, [p.x, p.y]);
                }
            }
        }

        let mut level_doors = BTreeMap::new();
        for (level, door) in &e_level_doors.0 {
            confirm_door_level(*level, *door)?;
            let level_id = get_level_id(*level)?;
            let door_id = get_door_id(*door)?;
            level_doors.insert(level_id, door_id);
        }

        let reference_anchors = get_anchor_id_edge(edge)?;
        let corrections = get_corrections_map(&corrections.0)?;
        lifts.insert(id.0, Lift{
            properties: LiftProperties{
                name: name.clone(),
                reference_anchors,
                cabin: cabin.clone(),
                level_doors: LevelDoors(level_doors),
                corrections,
                is_static: is_static.clone(),
            },
            cabin_anchors,
        });
    }

    return Ok(lifts);
}

fn generate_nav_graphs(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, NavGraph>, SiteGenerationError> {
    let state: SystemState<(
        Query<(&NavGraphProperties, &SiteID, &Parent, Option<&Children>)>,
        Query<(&Edge<Entity>, &Motion, &ReverseLane, &SiteID), With<LaneMarker>>,
        Query<(&Point<Entity>, &LocationTags, &SiteID)>,
        Query<&SiteID, With<Anchor>>,
    )> = SystemState::new(world);

    let (
        q_nav_graphs,
        q_lanes,
        q_locations,
        q_anchors,
    ) = state.get(world);

    let get_anchor_id = |entity| {
        let site_id = q_anchors.get(entity).map_err(
            |_| SiteGenerationError::BrokenAnchorReference(entity)
        )?;
        Ok(site_id.0)
    };

    let get_anchor_id_edge = |edge: &Edge<Entity>| {
        let left = get_anchor_id(edge.left())?;
        let right = get_anchor_id(edge.right())?;
        Ok(Edge::new(left, right))
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
                if let Ok((edge, forward, reverse, lane_id)) = q_lanes.get(*child) {
                    let edge = get_anchor_id_edge(edge)?;
                    lanes.insert(lane_id.0, Lane{
                        anchors: edge,
                        forward: forward.clone(),
                        reverse: reverse.clone(),
                        marker: LaneMarker
                    });
                }

                if let Ok((point, tags, location_id)) = q_locations.get(*child) {
                    let anchor = Point(get_anchor_id(point.0)?);
                    locations.insert(location_id.0, Location{
                        anchor,
                        tags: tags.clone()
                    });
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

pub fn save_site(world: &mut World) {
    let save_events: Vec<_> = world.resource_mut::<Events<SaveSite>>().drain().collect();
    for save_event in save_events {
        let path = {
            if let Some(to_file) = save_event.to_file {
                to_file
            } else {
                if let Some(to_file) = world.entity(save_event.site).get::<DefaultFile>() {
                    to_file.0.clone()
                } else {
                    let name = world.entity(save_event.site).get::<SiteProperties>()
                        .map(|site| site.name.clone())
                        .unwrap_or("<invalid site>".to_string());
                    println!("No default save file for {name}, please use [Save As]");
                    continue;
                }
            }
        };

        println!("Saving to {}", path.to_str().unwrap());
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
