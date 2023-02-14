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
use std::{collections::BTreeMap, path::PathBuf};
use thiserror::Error as ThisError;

use crate::site::*;
use crate::AppState;
use rmf_site_format::*;

// TODO(luca) rename site to root
pub struct SaveWorkspace {
    pub site: Entity,
    pub to_file: Option<PathBuf>,
}

pub struct SaveNavGraphs {
    pub site: Entity,
    pub to_file: PathBuf,
}

// TODO(MXG): Change all these errors to use u32 SiteIDs instead of entities
#[derive(ThisError, Debug, Clone)]
pub enum SiteGenerationError {
    #[error("the specified entity [{0:?}] does not refer to a site")]
    InvalidSiteEntity(Entity),
    #[error("an object has a reference to an anchor that does not exist")]
    BrokenAnchorReference(Entity),
    #[error("an object has a reference to a level that does not exist")]
    BrokenLevelReference(Entity),
    #[error("an object has a reference to a nav graph that does not exist")]
    BrokenNavGraphReference(Entity),
    #[error("lift {0} is missing its anchor group")]
    BrokenLift(u32),
    #[error(
        "anchor {anchor:?} is being referenced for site {site:?} but does not belong to that site"
    )]
    InvalidAnchorReference { site: u32, anchor: u32 },
    #[error(
        "lift door {door:?} is referencing an anchor that does not belong to its lift {anchor:?}"
    )]
    InvalidLiftDoorReference { door: Entity, anchor: Entity },
}

/// Look through all the elements that we will be saving and assign a SiteID
/// component to any elements that do not have one already.
fn assign_site_ids(world: &mut World, site: Entity) -> Result<(), SiteGenerationError> {
    let mut state: SystemState<(
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
        Query<
            Entity,
            (
                Or<(With<LaneMarker>, With<LocationTags>, With<NavGraphMarker>)>,
                Without<Pending>,
            ),
        >,
        Query<Entity, (With<LevelProperties>, Without<Pending>)>,
        Query<Entity, (With<LiftCabin<Entity>>, Without<Pending>)>,
        Query<&NextSiteID>,
        Query<&SiteID>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (level_children, nav_graph_elements, levels, lifts, sites, site_ids, children) =
        state.get_mut(world);

    let mut new_entities = Vec::new();

    let site_children = match children.get(site) {
        Ok(children) => children,
        Err(_) => {
            // The site seems to have no children at all. That's suspicious but
            // not impossible if the site is completely empty. In that case
            // there is no need to assign any SiteIDs
            return Ok(());
        }
    };

    for site_child in site_children {
        if let Ok(level) = levels.get(*site_child) {
            if !site_ids.contains(level) {
                new_entities.push(level);
            }

            if let Ok(children) = children.get(level) {
                for child in children {
                    if level_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            new_entities.push(*child);
                        }
                    }
                }
            }
        }

        if let Ok(e) = nav_graph_elements.get(*site_child) {
            if !site_ids.contains(e) {
                new_entities.push(e);
            }
        }

        if let Ok(lift) = lifts.get(*site_child) {
            if !site_ids.contains(lift) {
                new_entities.push(lift);
            }

            if let Ok(children) = children.get(lift) {
                for child in children {
                    if level_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            new_entities.push(*child);
                        }
                    }
                }
            }
        }
    }

    let mut next_site_id = sites
        .get(site)
        .map(|n| n.0)
        .map_err(|_| SiteGenerationError::InvalidSiteEntity(site))?..;
    for e in &new_entities {
        world
            .entity_mut(*e)
            .insert(SiteID(next_site_id.next().unwrap()));
    }

    world
        .entity_mut(site)
        .insert(NextSiteID(next_site_id.next().unwrap()));

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
            (
                &NameInSite,
                &AssetSource,
                &Pose,
                &IsStatic,
                &SiteID,
                &Parent,
            ),
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

    for (name, source, pose, is_static, id, parent) in &q_models {
        if let Ok((_, level_id, _)) = q_levels.get(parent.get()) {
            if let Some(level) = levels.get_mut(&level_id.0) {
                level.models.insert(
                    id.0,
                    Model {
                        name: name.clone(),
                        source: source.clone(),
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
        Query<&SiteID>,
    )> = SystemState::new(world);

    let (
        q_anchors,
        q_doors,
        q_levels,
        q_lifts,
        q_cabin_anchor_groups,
        q_parents,
        q_children,
        q_site_id,
    ) = state.get(world);

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

        Err(SiteGenerationError::InvalidAnchorReference {
            site: q_site_id.get(site).unwrap().0,
            anchor: q_site_id.get(anchor).unwrap().0,
        })
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

        // TODO(MXG): Clean up this spaghetti
        let anchor_group_entity = *match match q_children.get(lift_entity) {
            Ok(children) => children,
            Err(_) => return Err(SiteGenerationError::BrokenLift(id.0)),
        }
        .iter()
        .find(|c| q_cabin_anchor_groups.contains(**c))
        {
            Some(c) => c,
            None => return Err(SiteGenerationError::BrokenLift(id.0)),
        };

        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        validate_site_anchors(edge)?;

        let validate_level_door_anchor = |door: Entity, anchor: Entity| {
            if confirm_entity_parent(anchor_group_entity, anchor) {
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
    let mut state: SystemState<
        Query<
            (&NameInSite, &DisplayColor, &SiteID, &Parent),
            (With<NavGraphMarker>, Without<Pending>),
        >,
    > = SystemState::new(world);

    let q_nav_graphs = state.get(world);

    let mut nav_graphs = BTreeMap::new();
    for (name, color, id, parent) in &q_nav_graphs {
        if parent.get() != site {
            continue;
        }

        nav_graphs.insert(
            id.0,
            NavGraph {
                name: name.clone(),
                color: color.clone(),
                marker: Default::default(),
            },
        );
    }

    return Ok(nav_graphs);
}

fn generate_lanes(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, Lane<u32>>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &Motion,
                &ReverseLane,
                &AssociatedGraphs<Entity>,
                &SiteID,
                &Parent,
            ),
            (With<LaneMarker>, Without<Pending>),
        >,
        Query<&SiteID, With<NavGraphMarker>>,
        Query<&SiteID, With<Anchor>>,
    )> = SystemState::new(world);

    let (q_lanes, q_nav_graphs, q_anchors) = state.get(world);

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

    let mut lanes = BTreeMap::new();
    for (edge, o_edge, forward, reverse, graphs, lane_id, parent) in &q_lanes {
        if parent.get() != site {
            continue;
        }

        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        let edge = get_anchor_id_edge(edge)?;
        let graphs = graphs
            .to_u32(&q_nav_graphs)
            .map_err(|e| SiteGenerationError::BrokenNavGraphReference(e))?;

        lanes.insert(
            lane_id.0,
            Lane {
                anchors: edge.clone(),
                forward: forward.clone(),
                reverse: reverse.clone(),
                graphs,
                marker: LaneMarker,
            },
        );
    }

    Ok(lanes)
}

fn generate_locations(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<u32, Location<u32>>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<
            (
                &Point<Entity>,
                Option<&Original<Point<Entity>>>,
                &LocationTags,
                &NameInSite,
                &AssociatedGraphs<Entity>,
                &SiteID,
                &Parent,
            ),
            Without<Pending>,
        >,
        Query<&SiteID, With<NavGraphMarker>>,
        Query<&SiteID, With<Anchor>>,
    )> = SystemState::new(world);

    let (q_locations, q_nav_graphs, q_anchors) = state.get(world);

    let get_anchor_id = |entity| {
        let site_id = q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))?;
        Ok(site_id.0)
    };

    let mut locations = BTreeMap::new();
    for (point, o_point, tags, name, graphs, location_id, parent) in &q_locations {
        if parent.get() != site {
            continue;
        }

        let point = o_point.map(|x| &x.0).unwrap_or(point);
        let point = get_anchor_id(point.0)?;
        let graphs = graphs
            .to_u32(&q_nav_graphs)
            .map_err(|e| SiteGenerationError::BrokenNavGraphReference(e))?;

        locations.insert(
            location_id.0,
            Location {
                anchor: Point(point),
                tags: tags.clone(),
                name: name.clone(),
                graphs,
            },
        );
    }

    Ok(locations)
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
    let lanes = generate_lanes(world, site)?;
    let locations = generate_locations(world, site)?;

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
        navigation: Navigation {
            guided: Guided {
                graphs: nav_graphs,
                lanes,
                locations,
            },
        },
        // TODO(MXG): Parse agent information once the spec is figured out
        agents: Default::default(),
    });
}

pub fn save_workspace(world: &mut World) {
    let save_events: Vec<_> = world
        .resource_mut::<Events<SaveWorkspace>>()
        .drain()
        .collect();
    let app_state: AppState = world.resource::<State<AppState>>().current().clone();
    for save_event in save_events {
        let path = {
            if let Some(to_file) = save_event.to_file {
                to_file
            } else {
                if let Some(to_file) = world.entity(save_event.site).get::<DefaultFile>() {
                    to_file.0.clone()
                } else {
                    // TODO(luca) print errors for workcell editor mode as well
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

        println!(
            "Saving to {}",
            path.to_str().unwrap_or("<failed to render??>")
        );
        let f = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(err) => {
                println!("Unable to save file: {err}");
                continue;
            }
        };

        match app_state {
            AppState::SiteEditor => {
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
            AppState::WorkcellEditor => {
                println!("Saving workcell");
            }
            _ => {
                println!("Save failed, no workspace open");
            }
        }
    }
}

pub fn save_nav_graphs(world: &mut World) {
    let save_events: Vec<_> = world
        .resource_mut::<Events<SaveNavGraphs>>()
        .drain()
        .collect();
    for save_event in save_events {
        let path = save_event.to_file;

        let mut site = match generate_site(world, save_event.site) {
            Ok(site) => site,
            Err(err) => {
                println!("Unable to compile site: {err}");
                continue;
            }
        };

        for (name, nav_graph) in legacy::nav_graph::NavGraph::from_site(&site) {
            let mut graph_file = path.clone();
            graph_file.set_file_name(name + ".nav.yaml");
            println!(
                "Saving legacy nav graph to {}",
                graph_file.to_str().unwrap_or("<failed to render??>")
            );
            let f = match std::fs::File::create(graph_file) {
                Ok(f) => f,
                Err(err) => {
                    println!("Unable to save nav graph: {err}");
                    continue;
                }
            };
            if let Err(err) = serde_yaml::to_writer(f, &nav_graph) {
                println!("Failed to save nav graph: {err}");
            }
        }

        // Clear the elements that are not related to nav graphs
        for (_, level) in &mut site.levels {
            level.doors.clear();
            level.drawings.clear();
            level.fiducials.clear();
            level.floors.clear();
            level.lights.clear();
            level.measurements.clear();
            level.models.clear();
            level.walls.clear();
        }

        println!(
            "Saving all site nav graphs to {}",
            path.to_str().unwrap_or("<failed to render??>")
        );
        let f = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(err) => {
                println!("Unable to save file: {err}");
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
