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

use bevy::{
    ecs::{event::Events, system::SystemState},
    prelude::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use thiserror::Error as ThisError;

use crate::site::*;
use rmf_site_format::*;

pub struct SaveSite {
    pub site: Entity,
    pub to_file: Option<PathBuf>,
}

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
    #[error("lift {lift:?} has a reference anchor that does not belong to its site {site:?}")]
    InvalidLiftRefAnchor { site: Entity, lift: Entity },
    #[error(
        "anchor {anchor:?} is being referenced for site {site:?} but does not belong to that site"
    )]
    InvalidAnchorReference { site: Entity, anchor: Entity },
    #[error(
        "lift door {door:?} is referencing an anchor that does not belong to its lift {anchor:?}"
    )]
    InvalidLiftDoorReference { door: Entity, anchor: Entity },
    #[error(
        "door {door:?} is being referenced for level {level:?} but does not belong to that level"
    )]
    InvalidDoorReference { level: Entity, door: Entity },
}

/// Look through all the elements that we will be saving and assign a SiteID
/// component to any elements that do not have one already.
fn assign_site_ids(world: &mut World, site: Entity) -> Result<(), SiteGenerationError> {
    let mut state: SystemState<(
        Commands,
        Query<
            Entity,
            (
                Or<(
                    With<Anchor>,
                    With<DoorType>,
                    With<DrawingMarker>,
                    With<FiducialMarker>,
                    With<FloorMarker>,
                    With<LightKind>,
                    With<MeasurementMarker>,
                    With<ModelMarker>,
                    With<PhysicalCameraProperties>,
                    With<WallMarker>,
                )>,
                Without<Pending>,
            ),
        >,
        Query<Entity, (Or<(With<LaneMarker>, With<LocationTags>)>, Without<Pending>)>,
        Query<Entity, (With<LevelProperties>, Without<Pending>)>,
        Query<Entity, (With<NavGraphProperties>, Without<Pending>)>,
        Query<Entity, (With<LiftCabin<Entity>>, Without<Pending>)>,
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

    let mut next_site_id = sites
        .get_mut(site)
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

fn collect_site_anchors(world: &mut World, site: Entity) -> BTreeMap<u32, Anchor> {
    let mut state: SystemState<(
        Query<&Children>,
        Query<(&SiteID, &Anchor), Without<Pending>>,
    )> = SystemState::new(world);

    let mut site_anchors = BTreeMap::new();
    let (q_children, q_anchors) = state.get(world);
    if let Ok(children) = q_children.get(site) {
        for child in children {
            if let Ok((site_id, anchor)) = q_anchors.get(*child) {
                site_anchors.insert(site_id.0, anchor.clone());
            }
        }
    }

    site_anchors
}

fn generate_levels(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, Level>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&Anchor, &SiteID, &Parent)>,
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &NameInSite,
                &DoorType,
                &SiteID,
                &Parent,
            ),
            Without<Pending>,
        >,
        Query<
            (&AssetSource, &Pose, &PixelsPerMeter, &SiteID, &Parent),
            (With<DrawingMarker>, Without<Pending>),
        >,
        Query<
            (
                &Point<Entity>,
                Option<&Original<Point<Entity>>>,
                &Label,
                &SiteID,
                &Parent,
            ),
            (With<FiducialMarker>, Without<Pending>),
        >,
        Query<
            (
                &Path<Entity>,
                Option<&Original<Path<Entity>>>,
                &Texture,
                &SiteID,
                &Parent,
            ),
            (With<FloorMarker>, Without<Pending>),
        >,
        Query<(&LightKind, &Pose, &SiteID, &Parent)>,
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &Distance,
                &Label,
                &SiteID,
                &Parent,
            ),
            (With<MeasurementMarker>, Without<Pending>),
        >,
        Query<
            (&NameInSite, &Kind, &Pose, &IsStatic, &SiteID, &Parent),
            (With<ModelMarker>, Without<Pending>),
        >,
        Query<
            (
                &NameInSite,
                &Pose,
                &PhysicalCameraProperties,
                &SiteID,
                &Parent,
            ),
            Without<Pending>,
        >,
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &Texture,
                &SiteID,
                &Parent,
            ),
            (With<WallMarker>, Without<Pending>),
        >,
        Query<(&LevelProperties, &SiteID, &Parent), Without<Pending>>,
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
        let (_, site_id, _) = q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))?;
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

    for (anchor, id, parent) in &q_anchors {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.anchors.insert(id.0, anchor.clone());
            }
        }
    }

    for (edge, o_edge, name, kind, id, parent) in &q_doors {
        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_edge(edge)?;
                level.doors.insert(
                    id.0,
                    Door {
                        anchors,
                        name: name.clone(),
                        kind: kind.clone(),
                        marker: DoorMarker,
                    },
                );
            }
        }
    }

    for (source, pose, pixels_per_meter, id, parent) in &q_drawings {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.drawings.insert(
                    id.0,
                    Drawing {
                        source: source.clone(),
                        pose: pose.clone(),
                        pixels_per_meter: pixels_per_meter.clone(),
                        marker: DrawingMarker,
                    },
                );
            }
        }
    }

    for (point, o_point, label, id, parent) in &q_fiducials {
        let point = o_point.map(|x| &x.0).unwrap_or(point);
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchor = Point(get_anchor_id(point.0)?);
                level.fiducials.insert(
                    id.0,
                    Fiducial {
                        anchor,
                        label: label.clone(),
                        marker: FiducialMarker,
                    },
                );
            }
        }
    }

    for (path, o_path, texture, id, parent) in &q_floors {
        let path = o_path.map(|x| &x.0).unwrap_or(path);
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_path(&path)?;
                level.floors.insert(
                    id.0,
                    Floor {
                        anchors,
                        texture: texture.clone(),
                        marker: FloorMarker,
                    },
                );
            }
        }
    }

    for (kind, pose, id, parent) in &q_lights {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.lights.insert(
                    id.0,
                    Light {
                        pose: pose.clone(),
                        kind: kind.clone(),
                    },
                );
            }
        }
    }

    for (edge, o_edge, distance, label, id, parent) in &q_measurements {
        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_edge(edge)?;
                level.measurements.insert(
                    id.0,
                    Measurement {
                        anchors,
                        distance: distance.clone(),
                        label: label.clone(),
                        marker: MeasurementMarker,
                    },
                );
            }
        }
    }

    for (name, kind, pose, is_static, id, parent) in &q_models {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.models.insert(
                    id.0,
                    Model {
                        name: name.clone(),
                        kind: kind.clone(),
                        pose: pose.clone(),
                        is_static: is_static.clone(),
                        marker: ModelMarker,
                    },
                );
            }
        }
    }

    for (name, pose, properties, id, parent) in &q_physical_cameras {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.physical_cameras.insert(
                    id.0,
                    PhysicalCamera {
                        name: name.clone(),
                        pose: pose.clone(),
                        properties: properties.clone(),
                        previewable: PreviewableMarker,
                    },
                );
            }
        }
    }

    for (edge, o_edge, texture, id, parent) in &q_walls {
        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                let anchors = get_anchor_id_edge(edge)?;
                level.walls.insert(
                    id.0,
                    Wall {
                        anchors,
                        texture: texture.clone(),
                        marker: WallMarker,
                    },
                );
            }
        }
    }

    return Ok(levels);
}

type QueryLift<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static NameInSite,
        &'static Edge<Entity>,
        Option<&'static Original<Edge<Entity>>>,
        &'static LiftCabin<Entity>,
        &'static IsStatic,
        &'static InitialLevel<Entity>,
        &'static SiteID,
        &'static Parent,
    ),
    Without<Pending>,
>;

fn generate_lifts(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, Lift<u32>>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&SiteID, &Anchor), Without<Pending>>,
        QueryLiftDoor,
        Query<&SiteID, (With<LevelProperties>, Without<Pending>)>,
        QueryLift,
        Query<Entity, With<CabinAnchorGroup>>,
        Query<&Parent, Without<Pending>>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_anchors, q_doors, q_levels, q_lifts, q_cabin_anchor_groups, q_parents, q_children) =
        state.get(world);

    let mut lifts = BTreeMap::new();

    let get_anchor_id = |entity| {
        let (site_id, _) = q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))?;
        Ok(site_id.0)
    };

    let get_level_id = |entity| -> Result<u32, SiteGenerationError> {
        let site_id = q_levels
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenLevelReference(entity))?;
        Ok(site_id.0)
    };

    let get_anchor_id_edge = |edge: &Edge<Entity>| {
        let left = get_anchor_id(edge.left())?;
        let right = get_anchor_id(edge.right())?;
        Ok(Edge::new(left, right))
    };

    let confirm_entity_parent = |intended_parent, child| {
        if let Ok(actual_parent) = q_parents.get(child) {
            if actual_parent.get() == intended_parent {
                return true;
            }
        }

        return false;
    };

    let validate_site_anchor = |anchor| {
        if confirm_entity_parent(site, anchor) {
            return Ok(());
        }

        Err(SiteGenerationError::InvalidAnchorReference { site, anchor })
    };

    let validate_site_anchors = |edge: &Edge<Entity>| {
        validate_site_anchor(edge.left())?;
        validate_site_anchor(edge.right())?;
        Ok(())
    };

    for (lift_entity, name, edge, o_edge, cabin, is_static, initial_level, id, parent) in &q_lifts {
        if parent.get() != site {
            continue;
        }

        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        validate_site_anchors(edge)?;

        let validate_level_door_anchor = |door: Entity, anchor: Entity| {
            if confirm_entity_parent(lift_entity, anchor) {
                return Ok(());
            }

            Err(SiteGenerationError::InvalidLiftDoorReference { door, anchor })
        };

        let validate_level_door_anchors = |door: Entity, edge: &Edge<Entity>| {
            validate_level_door_anchor(door, edge.left())?;
            validate_level_door_anchor(door, edge.right())?;
            get_anchor_id_edge(edge)
        };

        let mut cabin_anchors = BTreeMap::new();
        let mut cabin_doors = BTreeMap::new();
        if let Ok(children) = q_children.get(lift_entity) {
            for child in children {
                if let Ok(anchor_group) = q_cabin_anchor_groups.get(*child) {
                    if let Ok(anchor_children) = q_children.get(anchor_group) {
                        for anchor_child in anchor_children {
                            if let Ok((site_id, anchor)) = q_anchors.get(*anchor_child) {
                                cabin_anchors.insert(site_id.0, anchor.clone());
                            }
                        }
                    }
                }

                if let Ok((site_id, door_type, edge, o_edge, visits)) = q_doors.get(*child) {
                    let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                    cabin_doors.insert(
                        site_id.0,
                        LiftCabinDoor {
                            kind: door_type.clone(),
                            reference_anchors: validate_level_door_anchors(*child, edge)?,
                            visits: LevelVisits(
                                visits
                                    .iter()
                                    .map(|level| get_level_id(*level))
                                    .collect::<Result<_, _>>()?,
                            ),
                            marker: Default::default(),
                        },
                    );
                }
            }
        }

        let reference_anchors = get_anchor_id_edge(edge)?;
        lifts.insert(
            id.0,
            Lift {
                cabin_doors,
                properties: LiftProperties {
                    name: name.clone(),
                    reference_anchors,
                    cabin: cabin.to_u32(&q_doors),
                    is_static: is_static.clone(),
                    initial_level: InitialLevel(
                        initial_level
                            .0
                            .map_or(Ok(None), |level| get_level_id(level).map(|id| Some(id)))?,
                    ),
                },
                cabin_anchors,
            },
        );
    }

    return Ok(lifts);
}

fn generate_nav_graphs(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, NavGraph>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&NavGraphProperties, &SiteID, &Parent, Option<&Children>)>,
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &Motion,
                &ReverseLane,
                &SiteID,
            ),
            (With<LaneMarker>, Without<Pending>),
        >,
        Query<
            (
                &Point<Entity>,
                Option<&Original<Point<Entity>>>,
                &LocationTags,
                &SiteID,
            ),
            Without<Pending>,
        >,
        Query<&SiteID, With<Anchor>>,
    )> = SystemState::new(world);

    let (q_nav_graphs, q_lanes, q_locations, q_anchors) = state.get(world);

    let get_anchor_id = |entity| {
        let site_id = q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))?;
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
                if let Ok((edge, o_edge, forward, reverse, lane_id)) = q_lanes.get(*child) {
                    let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                    let edge = get_anchor_id_edge(edge)?;
                    lanes.insert(
                        lane_id.0,
                        Lane {
                            anchors: edge,
                            forward: forward.clone(),
                            reverse: reverse.clone(),
                            marker: LaneMarker,
                        },
                    );
                }

                if let Ok((point, o_point, tags, location_id)) = q_locations.get(*child) {
                    let point = o_point.map(|x| &x.0).unwrap_or(point);
                    let anchor = Point(get_anchor_id(point.0)?);
                    locations.insert(
                        location_id.0,
                        Location {
                            anchor,
                            tags: tags.clone(),
                        },
                    );
                }
            }
        }

        nav_graphs.insert(
            id.0,
            NavGraph {
                properties: properties.clone(),
                lanes,
                locations,
            },
        );
    }

    return Ok(nav_graphs);
}

pub fn generate_site(
    world: &mut World,
    site: Entity,
) -> Result<rmf_site_format::Site, SiteGenerationError> {
    assign_site_ids(world, site)?;
    let anchors = collect_site_anchors(world, site);
    let levels = generate_levels(world, site)?;
    let lifts = generate_lifts(world, site)?;
    let nav_graphs = generate_nav_graphs(world, site)?;

    let props = match world.get::<SiteProperties>(site) {
        Some(props) => props,
        None => {
            return Err(SiteGenerationError::InvalidSiteEntity(site));
        }
    };

    return Ok(Site {
        format_version: rmf_site_format::SemVer::default(),
        anchors,
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
                    let name = world
                        .entity(save_event.site)
                        .get::<SiteProperties>()
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
            }
            Err(err) => {
                println!("Save failed: {err}");
            }
        }
    }
}
