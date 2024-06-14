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

use crate::{recency::RecencyRanking, site::*, ExportFormat};
use rmf_site_format::*;

#[derive(Event)]
pub struct SaveSite {
    pub site: Entity,
    pub to_file: PathBuf,
    pub format: ExportFormat,
}

#[derive(Event)]
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
    #[error("an object has a reference to a group that does not exist")]
    BrokenAffiliation(Entity),
    #[error("an object has a reference to a level that does not exist")]
    BrokenLevelReference(Entity),
    #[error("an object has a reference to a nav graph that does not exist")]
    BrokenNavGraphReference(Entity),
    #[error("an issue has a reference to an object that does not exist")]
    BrokenIssueReference(Entity),
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

/// This is used when a drawing is being edited to fix its parenting before we
/// attempt to save the site.
// TODO(@mxgrey): Remove this when we no longer need to de-parent drawings while
// editing them.
fn assemble_edited_drawing(world: &mut World) {
    let Some(c) = world.get_resource::<CurrentEditDrawing>().copied() else {
        return;
    };
    let Some(c) = c.target() else { return };
    let Some(mut level) = world.get_entity_mut(c.level) else {
        return;
    };
    level.push_children(&[c.drawing]);
}

/// Revert the drawing back to the root so it can continue to be edited.
fn disassemble_edited_drawing(world: &mut World) {
    let Some(c) = world.get_resource::<CurrentEditDrawing>().copied() else {
        return;
    };
    let Some(c) = c.target() else { return };
    let Some(mut level) = world.get_entity_mut(c.level) else {
        return;
    };
    level.remove_children(&[c.drawing]);
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
                    With<FloorMarker>,
                    With<LightKind>,
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
        Query<Entity, (With<LevelElevation>, Without<Pending>)>,
        Query<Entity, (With<LiftCabin<Entity>>, Without<Pending>)>,
        Query<
            Entity,
            (
                Or<(
                    With<Anchor>,
                    With<FiducialMarker>,
                    With<MeasurementMarker>,
                    With<Group>,
                )>,
                Without<Pending>,
            ),
        >,
        Query<(), With<DrawingMarker>>,
        Query<&NextSiteID>,
        Query<&SiteID>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (
        level_children,
        nav_graph_elements,
        levels,
        lifts,
        drawing_children,
        drawings,
        sites,
        site_ids,
        children,
    ) = state.get_mut(world);

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

            if let Ok(current_level_children) = children.get(level) {
                for child in current_level_children {
                    if level_children.contains(*child) {
                        if !site_ids.contains(*child) {
                            new_entities.push(*child);
                        }

                        if drawings.contains(*child) {
                            if let Ok(drawing_children) = children.get(*child) {
                                for child in drawing_children {
                                    if !site_ids.contains(*child) {
                                        new_entities.push(*child);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Ok(e) = drawing_children.get(*site_child) {
            // Sites can contain anchors and fiducials but should not contain
            // measurements, so this query doesn't make perfect sense to use
            // here, but it shouldn't be harmful and it saves us from writing
            // yet another query.
            if !site_ids.contains(e) {
                new_entities.push(e);
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
        Query<&Children, With<NameOfSite>>,
        Query<(&Anchor, &SiteID)>,
        Query<&SiteID, With<Group>>,
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &NameInSite,
                &DoorType,
                &SiteID,
            ),
            Without<Pending>,
        >,
        Query<
            (
                &NameInSite,
                &AssetSource,
                &Pose,
                &PixelsPerMeter,
                &PreferredSemiTransparency,
                &SiteID,
                &Children,
            ),
            (With<DrawingMarker>, Without<Pending>),
        >,
        Query<
            (
                &Point<Entity>,
                Option<&Original<Point<Entity>>>,
                &Affiliation<Entity>,
                &SiteID,
            ),
            (With<FiducialMarker>, Without<Pending>),
        >,
        Query<
            (
                &Path<Entity>,
                Option<&Original<Path<Entity>>>,
                &Affiliation<Entity>,
                &PreferredSemiTransparency,
                &SiteID,
            ),
            (With<FloorMarker>, Without<Pending>),
        >,
        Query<(&LightKind, &Pose, &SiteID)>,
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &Distance,
                &SiteID,
            ),
            (With<MeasurementMarker>, Without<Pending>),
        >,
        Query<
            (&NameInSite, &AssetSource, &Pose, &IsStatic, &Scale, &SiteID),
            (With<ModelMarker>, Without<Pending>),
        >,
        Query<(&NameInSite, &Pose, &PhysicalCameraProperties, &SiteID), Without<Pending>>,
        Query<
            (
                &Edge<Entity>,
                Option<&Original<Edge<Entity>>>,
                &Affiliation<Entity>,
                &SiteID,
            ),
            (With<WallMarker>, Without<Pending>),
        >,
        Query<
            (
                &NameInSite,
                &LevelElevation,
                &GlobalFloorVisibility,
                &GlobalDrawingVisibility,
                &SiteID,
                &Children,
                Option<&RecencyRanking<FloorMarker>>,
                Option<&RecencyRanking<DrawingMarker>>,
            ),
            Without<Pending>,
        >,
        Query<&SiteID>,
        Query<(&Pose, &NameInSite, &SiteID), With<UserCameraPoseMarker>>,
    )> = SystemState::new(world);

    let (
        q_site_children,
        q_anchors,
        q_groups,
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
        q_site_ids,
        q_user_camera_poses,
    ) = state.get(world);

    let get_anchor_id = |entity| {
        let (_, site_id) = q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))?;
        Ok(site_id.0)
    };

    let get_group_id = |entity| {
        q_groups
            .get(entity)
            .map(|id| id.0)
            .map_err(|_| SiteGenerationError::BrokenAffiliation(entity))
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

    let mut levels = BTreeMap::new();
    if let Ok(site_children) = q_site_children.get(site) {
        for c in site_children.iter() {
            if let Ok((
                name,
                elevation,
                floor_vis,
                drawing_vis,
                level_id,
                level_children,
                floor_ranking,
                drawing_ranking,
            )) = q_levels.get(*c)
            {
                let mut level = Level::new(
                    LevelProperties {
                        name: name.clone(),
                        elevation: elevation.clone(),
                        global_floor_visibility: floor_vis.clone(),
                        global_drawing_visibility: drawing_vis.clone(),
                    },
                    RankingsInLevel {
                        floors: floor_ranking
                            .map(|r| r.to_u32(&q_site_ids))
                            .unwrap_or(Vec::new()),
                        drawings: drawing_ranking
                            .map(|r| r.to_u32(&q_site_ids))
                            .unwrap_or(Vec::new()),
                    },
                );
                for c in level_children.iter() {
                    if let Ok((anchor, id)) = q_anchors.get(*c) {
                        level.anchors.insert(id.0, anchor.clone());
                    }
                    if let Ok((edge, o_edge, name, kind, id)) = q_doors.get(*c) {
                        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
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
                    if let Ok((
                        name,
                        source,
                        pose,
                        pixels_per_meter,
                        preferred_alpha,
                        id,
                        children,
                    )) = q_drawings.get(*c)
                    {
                        let mut measurements = BTreeMap::new();
                        let mut fiducials = BTreeMap::new();
                        let mut anchors = BTreeMap::new();
                        for e in children.iter() {
                            if let Ok((anchor, anchor_id)) = q_anchors.get(*e) {
                                anchors.insert(anchor_id.0, anchor.clone());
                            }
                            if let Ok((edge, o_edge, distance, id)) = q_measurements.get(*e) {
                                let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                                let anchors = get_anchor_id_edge(edge)?;
                                measurements.insert(
                                    id.0,
                                    Measurement {
                                        anchors,
                                        distance: distance.clone(),
                                        marker: MeasurementMarker,
                                    },
                                );
                            }
                            if let Ok((point, o_point, affiliation, id)) = q_fiducials.get(*e) {
                                let point = o_point.map(|x| &x.0).unwrap_or(point);
                                let anchor = Point(get_anchor_id(point.0)?);
                                let affiliation = if let Affiliation(Some(e)) = affiliation {
                                    Affiliation(Some(get_group_id(*e)?))
                                } else {
                                    Affiliation(None)
                                };
                                fiducials.insert(
                                    id.0,
                                    Fiducial {
                                        anchor,
                                        affiliation,
                                        marker: FiducialMarker,
                                    },
                                );
                            }
                        }
                        level.drawings.insert(
                            id.0,
                            Drawing {
                                properties: DrawingProperties {
                                    name: name.clone(),
                                    source: source.clone(),
                                    pose: pose.clone(),
                                    pixels_per_meter: pixels_per_meter.clone(),
                                    preferred_semi_transparency: preferred_alpha.clone(),
                                },
                                anchors,
                                fiducials,
                                measurements,
                            },
                        );
                    }
                    if let Ok((path, o_path, texture, preferred_alpha, id)) = q_floors.get(*c) {
                        let path = o_path.map(|x| &x.0).unwrap_or(path);
                        let anchors = get_anchor_id_path(&path)?;
                        let texture = if let Affiliation(Some(e)) = texture {
                            Affiliation(Some(get_group_id(*e)?))
                        } else {
                            Affiliation(None)
                        };

                        level.floors.insert(
                            id.0,
                            Floor {
                                anchors,
                                texture,
                                preferred_semi_transparency: preferred_alpha.clone(),
                                marker: FloorMarker,
                            },
                        );
                    }
                    if let Ok((kind, pose, id)) = q_lights.get(*c) {
                        level.lights.insert(
                            id.0,
                            Light {
                                pose: pose.clone(),
                                kind: kind.clone(),
                            },
                        );
                    }
                    if let Ok((name, source, pose, is_static, scale, id)) = q_models.get(*c) {
                        level.models.insert(
                            id.0,
                            Model {
                                name: name.clone(),
                                source: source.clone(),
                                pose: pose.clone(),
                                is_static: is_static.clone(),
                                scale: scale.clone(),
                                marker: ModelMarker,
                            },
                        );
                    }
                    if let Ok((name, pose, properties, id)) = q_physical_cameras.get(*c) {
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
                    if let Ok((edge, o_edge, texture, id)) = q_walls.get(*c) {
                        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                        let anchors = get_anchor_id_edge(edge)?;
                        let texture = if let Affiliation(Some(e)) = texture {
                            Affiliation(Some(get_group_id(*e)?))
                        } else {
                            Affiliation(None)
                        };

                        level.walls.insert(
                            id.0,
                            Wall {
                                anchors,
                                texture,
                                marker: WallMarker,
                            },
                        );
                    }
                    if let Ok((pose, name, id)) = q_user_camera_poses.get(*c) {
                        level.user_camera_poses.insert(
                            id.0,
                            UserCameraPose {
                                name: name.clone(),
                                pose: pose.clone(),
                                marker: UserCameraPoseMarker,
                            },
                        );
                    }
                }
                levels.insert(level_id.0, level);
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
        Query<&SiteID, (With<LevelElevation>, Without<Pending>)>,
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

fn generate_fiducials(
    world: &mut World,
    parent: Entity,
) -> Result<BTreeMap<u32, Fiducial<u32>>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<&SiteID, (With<Anchor>, Without<Pending>)>,
        Query<&SiteID, (With<Group>, Without<Pending>)>,
        Query<
            (&Point<Entity>, &Affiliation<Entity>, &SiteID),
            (With<FiducialMarker>, Without<Pending>),
        >,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_anchor_ids, q_group_ids, q_fiducials, q_children) = state.get(world);

    let Ok(children) = q_children.get(parent) else {
        return Ok(BTreeMap::new());
    };

    let mut fiducials = BTreeMap::new();
    for child in children {
        let Ok((point, affiliation, site_id)) = q_fiducials.get(*child) else {
            continue;
        };
        let anchor = q_anchor_ids
            .get(point.0)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(point.0))?
            .0;
        let anchor = Point(anchor);
        let affiliation = if let Some(e) = affiliation.0 {
            let group_id = q_group_ids
                .get(e)
                .map_err(|_| SiteGenerationError::BrokenAffiliation(e))?
                .0;
            Affiliation(Some(group_id))
        } else {
            Affiliation(None)
        };

        fiducials.insert(
            site_id.0,
            Fiducial {
                anchor,
                affiliation,
                marker: Default::default(),
            },
        );
    }

    Ok(fiducials)
}

fn generate_fiducial_groups(
    world: &mut World,
    parent: Entity,
) -> Result<BTreeMap<u32, FiducialGroup>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&NameInSite, &SiteID), (With<Group>, With<FiducialMarker>)>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_groups, q_children) = state.get(world);

    let Ok(children) = q_children.get(parent) else {
        return Ok(BTreeMap::new());
    };

    let mut fiducial_groups = BTreeMap::new();
    for child in children {
        let Ok((name, site_id)) = q_groups.get(*child) else {
            continue;
        };
        fiducial_groups.insert(site_id.0, FiducialGroup::new(name.clone()));
    }

    Ok(fiducial_groups)
}

fn generate_texture_groups(
    world: &mut World,
    parent: Entity,
) -> Result<BTreeMap<u32, TextureGroup>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&NameInSite, &Texture, &SiteID), With<Group>>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_groups, q_children) = state.get(world);

    let Ok(children) = q_children.get(parent) else {
        return Ok(BTreeMap::new());
    };

    let mut texture_groups = BTreeMap::new();
    for child in children {
        let Ok((name, texture, site_id)) = q_groups.get(*child) else {
            continue;
        };
        texture_groups.insert(
            site_id.0,
            TextureGroup {
                name: name.clone(),
                texture: texture.clone(),
                group: Default::default(),
            },
        );
    }

    Ok(texture_groups)
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

fn generate_graph_rankings(
    world: &mut World,
    site: Entity,
) -> Result<Vec<u32>, SiteGenerationError> {
    let mut state: SystemState<(Query<&RecencyRanking<NavGraphMarker>>, Query<&SiteID>)> =
        SystemState::new(world);

    let (rankings, site_id) = state.get(world);
    let ranking = match rankings.get(site) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };

    ranking
        .entities()
        .iter()
        .map(|e| {
            site_id
                .get(*e)
                .map(|s| s.0)
                .map_err(|_| SiteGenerationError::BrokenNavGraphReference(*e))
        })
        .collect()
}

fn generate_site_properties(
    world: &mut World,
    site: Entity,
) -> Result<SiteProperties<u32>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(
            &NameOfSite,
            &FilteredIssues<Entity>,
            &FilteredIssueKinds,
            &GeographicComponent,
        )>,
        Query<&SiteID>,
    )> = SystemState::new(world);

    let (q_properties, q_ids) = state.get(world);

    let Ok((name, issues, issue_kinds, geographic_offset)) = q_properties.get(site) else {
        return Err(SiteGenerationError::InvalidSiteEntity(site));
    };

    let mut converted_issues = BTreeSet::new();
    for issue in issues.iter() {
        let mut entities = BTreeSet::new();
        for e in issue.entities.iter() {
            let id = q_ids
                .get(*e)
                .map_err(|_| SiteGenerationError::BrokenIssueReference(*e))?;
            entities.insert(**id);
        }
        converted_issues.insert(IssueKey {
            entities,
            kind: issue.kind.clone(),
        });
    }

    Ok(SiteProperties {
        name: name.clone(),
        geographic_offset: geographic_offset.clone(),
        filtered_issues: FilteredIssues(converted_issues),
        filtered_issue_kinds: issue_kinds.clone(),
    })
}

fn migrate_relative_paths(
    site: Entity,
    new_path: &PathBuf,
    world: &mut World,
    // In((new_path, site)): In<(&PathBuf, Entity)>,
    // mut assets: Query<(Entity, &mut AssetSource)>,
    // mut default_files: Query<&mut DefaultFile>,
    // mut commands: Commands,
    // parents: Query<&Parent>,
) {
    let old_path = if let Some(mut default_file) = world.get_mut::<DefaultFile>(site) {
        let old_path = default_file.0.clone();
        default_file.0 = new_path.clone();
        old_path
    } else {
        world.entity_mut(site).insert(DefaultFile(new_path.clone()));
        // If there was not already a default file then there is no way to
        // migrate relative paths because they had no reference path to actually
        // be relative to.
        return;
    };

    let mut state: SystemState<(Query<(Entity, &mut AssetSource)>, Query<&Parent>)> =
        SystemState::new(world);

    let (mut assets, parents) = state.get_mut(world);

    for (mut e, mut source) in &mut assets {
        let asset_entity = e;
        if !source.is_local_relative() {
            continue;
        }

        loop {
            if e == site {
                if source.migrate_relative_path(&old_path, new_path).is_err() {
                    error!(
                        "Failed to migrate relative path for {asset_entity:?}: {:?}",
                        *source,
                    );
                    break;
                }
            }

            if let Ok(parent) = parents.get(e) {
                e = parent.get();
            } else {
                break;
            }
        }
    }
}

pub fn generate_site(
    world: &mut World,
    site: Entity,
) -> Result<rmf_site_format::Site, SiteGenerationError> {
    assemble_edited_drawing(world);

    assign_site_ids(world, site)?;
    let anchors = collect_site_anchors(world, site);
    let levels = generate_levels(world, site)?;
    let lifts = generate_lifts(world, site)?;
    let fiducials = generate_fiducials(world, site)?;
    let fiducial_groups = generate_fiducial_groups(world, site)?;
    let textures = generate_texture_groups(world, site)?;
    let nav_graphs = generate_nav_graphs(world, site)?;
    let lanes = generate_lanes(world, site)?;
    let locations = generate_locations(world, site)?;
    let graph_ranking = generate_graph_rankings(world, site)?;
    let properties = generate_site_properties(world, site)?;

    disassemble_edited_drawing(world);
    return Ok(Site {
        format_version: rmf_site_format::SemVer::default(),
        anchors,
        properties,
        levels,
        lifts,
        fiducials,
        fiducial_groups,
        textures,
        navigation: Navigation {
            guided: Guided {
                graphs: nav_graphs,
                ranking: graph_ranking,
                lanes,
                locations,
            },
        },
        // TODO(MXG): Parse agent information once the spec is figured out
        agents: Default::default(),
    });
}

pub fn save_site(world: &mut World) {
    let save_events: Vec<_> = world.resource_mut::<Events<SaveSite>>().drain().collect();
    for save_event in save_events {
        let mut new_path = save_event.to_file;
        let path_str = match new_path.to_str() {
            Some(s) => s,
            None => {
                error!("Unable to save file: Invalid path [{new_path:?}]");
                continue;
            }
        };
        match save_event.format {
            ExportFormat::Default => {
                if path_str.ends_with(".building.yaml") {
                    warn!("Detected old file format, converting to new format");
                    new_path = path_str.replace(".building.yaml", ".site.ron").into();
                } else if path_str.ends_with(".site.json") {
                    // Noop
                } else if !path_str.ends_with(".site.ron") {
                    info!("Appending .site.ron to {}", new_path.display());
                    new_path = new_path.with_extension("site.ron");
                }
                info!("Saving to {}", new_path.display());
                let f = match std::fs::File::create(new_path.clone()) {
                    Ok(f) => f,
                    Err(err) => {
                        error!("Unable to save file: {err}");
                        continue;
                    }
                };

                let old_default_path = world.get::<DefaultFile>(save_event.site).cloned();
                migrate_relative_paths(save_event.site, &new_path, world);

                let site = match generate_site(world, save_event.site) {
                    Ok(site) => site,
                    Err(err) => {
                        error!("Unable to compile site: {err}");
                        continue;
                    }
                };

                if new_path.extension().is_some_and(|e| e == "json") {
                    match site.to_writer_json(f) {
                        Ok(()) => {
                            info!("Save successful");
                        }
                        Err(err) => {
                            if let Some(old_default_path) = old_default_path {
                                world.entity_mut(save_event.site).insert(old_default_path);
                            }
                            error!("Save failed: {err}");
                        }
                    }
                } else {
                    match site.to_writer_ron(f) {
                        Ok(()) => {
                            info!("Save successful");
                        }
                        Err(err) => {
                            if let Some(old_default_path) = old_default_path {
                                world.entity_mut(save_event.site).insert(old_default_path);
                            }
                            error!("Save failed: {err}");
                        }
                    }
                }
            }
            ExportFormat::Sdf => {
                // TODO(luca) reduce code duplication with default exporting

                // Make sure to generate the site before anything else, because
                // generating the site will ensure that all items are assigned a
                // SiteID, and the SDF export process will not work correctly if
                // any are unassigned.
                let site = match generate_site(world, save_event.site) {
                    Ok(site) => site,
                    Err(err) => {
                        error!("Unable to compile site: {err}");
                        continue;
                    }
                };

                info!("Saving to {}", new_path.display());
                let Some(parent_folder) = new_path.parent() else {
                    error!("Unable to save SDF. Please select a save path that has a parent directory.");
                    continue;
                };
                if !parent_folder.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent_folder) {
                        error!("Unable to create folder {}: {e}", parent_folder.display());
                        continue;
                    }
                }
                let f = match std::fs::File::create(&new_path) {
                    Ok(f) => f,
                    Err(err) => {
                        error!("Unable to save file {}: {err}", new_path.display());
                        continue;
                    }
                };

                let mut meshes_dir = PathBuf::from(parent_folder);
                meshes_dir.push("meshes");
                if let Err(e) = std::fs::create_dir_all(&meshes_dir) {
                    error!("Unable to create folder {}: {e}", meshes_dir.display());
                    continue;
                }
                if let Err(e) = collect_site_meshes(world, save_event.site, &meshes_dir) {
                    error!("Unable to collect site meshes: {e}");
                    continue;
                }

                migrate_relative_paths(save_event.site, &new_path, world);
                let graphs = legacy::nav_graph::NavGraph::from_site(&site);
                let sdf = match site.to_sdf() {
                    Ok(sdf) => sdf,
                    Err(err) => {
                        error!("Unable to convert site to sdf: {err}");
                        continue;
                    }
                };
                let config = yaserde::ser::Config {
                    perform_indent: true,
                    write_document_declaration: true,
                    ..Default::default()
                };
                if let Err(e) = yaserde::ser::serialize_with_writer(&sdf, f, &config) {
                    error!("Failed serializing site to sdf: {e}");
                    continue;
                }
                let mut navgraph_dir = PathBuf::from(parent_folder);
                navgraph_dir.push("nav_graphs");
                if let Err(e) = std::fs::create_dir_all(&navgraph_dir) {
                    error!("Unable to create folder {}: {e}", navgraph_dir.display());
                    continue;
                }
                for (name, graph) in &graphs {
                    let mut graph_file = navgraph_dir.clone();
                    graph_file.push(name.to_owned() + ".yaml");
                    info!(
                        "Saving legacy nav graph to {}",
                        graph_file.to_str().unwrap_or("<failed to render??>")
                    );
                    let f = match std::fs::File::create(graph_file) {
                        Ok(f) => f,
                        Err(err) => {
                            error!("Unable to save nav graph: {err}");
                            continue;
                        }
                    };
                    if let Err(err) = serde_yaml::to_writer(f, &graph) {
                        error!("Failed to save nav graph: {err}");
                    }
                }
            }
            ExportFormat::Urdf => {
                warn!("Site exporting to Urdf is not supported.");
                continue;
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
                error!("Unable to compile site: {err}");
                continue;
            }
        };

        for (name, nav_graph) in legacy::nav_graph::NavGraph::from_site(&site) {
            let mut graph_file = path.clone();
            graph_file.set_file_name(name + ".nav.yaml");
            info!(
                "Saving legacy nav graph to {}",
                graph_file.to_str().unwrap_or("<failed to render??>")
            );
            let f = match std::fs::File::create(graph_file) {
                Ok(f) => f,
                Err(err) => {
                    error!("Unable to save nav graph: {err}");
                    continue;
                }
            };
            if let Err(err) = serde_yaml::to_writer(f, &nav_graph) {
                error!("Failed to save nav graph: {err}");
            }
        }

        // Clear the elements that are not related to nav graphs
        for (_, level) in &mut site.levels {
            level.doors.clear();
            level.drawings.clear();
            level.floors.clear();
            level.lights.clear();
            level.models.clear();
            level.walls.clear();
        }

        info!(
            "Saving all site nav graphs to {}",
            path.to_str().unwrap_or("<failed to render??>")
        );
        let f = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(err) => {
                error!("Unable to save file: {err}");
                continue;
            }
        };

        match site.to_writer_ron(f) {
            Ok(()) => {
                info!("Save successful");
            }
            Err(err) => {
                error!("Save failed: {err}");
            }
        }
    }
}
