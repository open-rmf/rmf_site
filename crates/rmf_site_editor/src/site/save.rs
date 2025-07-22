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
    ecs::{event::Events, hierarchy::ChildOf, system::SystemState},
    prelude::*,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::PathBuf,
};
use thiserror::Error as ThisError;

use crate::{
    exit_confirmation::SiteChanged, recency::RecencyRanking, site::*, ExportFormat, Issue,
};
use rmf_site_format::*;
use sdformat_rs::yaserde;

#[derive(Event)]
pub struct SaveSite {
    pub site: Entity,
    pub to_location: PathBuf,
    pub format: ExportFormat,
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
    BrokenLift(Entity),
    #[error(
        "anchor {anchor:?} is being referenced for site {site:?} but does not belong to that site"
    )]
    InvalidAnchorReference { site: SiteID, anchor: SiteID },
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
    let Ok(mut level) = world.get_entity_mut(c.level) else {
        return;
    };
    level.add_children(&[c.drawing]);
}

/// Revert the drawing back to the root so it can continue to be edited.
fn disassemble_edited_drawing(world: &mut World) {
    let Some(c) = world.get_resource::<CurrentEditDrawing>().copied() else {
        return;
    };
    let Some(c) = c.target() else { return };
    let Ok(mut level) = world.get_entity_mut(c.level) else {
        return;
    };
    level.remove_children(&[c.drawing]);
}

fn collect_site_anchors(world: &mut World, site: Entity) -> BTreeMap<SiteID, Anchor> {
    let mut state: SystemState<(Query<&Children>, Query<&Anchor, Without<Pending>>)> =
        SystemState::new(world);

    let mut site_anchors = BTreeMap::new();
    let (q_children, q_anchors) = state.get(world);
    if let Ok(children) = q_children.get(site) {
        for child in children {
            if let Ok(anchor) = q_anchors.get(*child) {
                site_anchors.insert((*child).into(), anchor.clone());
            }
        }
    }

    site_anchors
}

fn generate_levels(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<SiteID, Level>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<&Children, With<NameOfSite>>,
        Query<&Anchor>,
        Query<(), With<Group>>,
        Query<(&Edge, Option<&Original<Edge>>, &NameInSite, &DoorType), Without<Pending>>,
        Query<
            (
                &NameInSite,
                &AssetSource,
                &Pose,
                &PixelsPerMeter,
                &PreferredSemiTransparency,
                &Children,
            ),
            (With<DrawingMarker>, Without<Pending>),
        >,
        Query<
            (&Point, Option<&Original<Point>>, &Affiliation),
            (With<FiducialMarker>, Without<Pending>),
        >,
        Query<
            (
                &Path,
                Option<&Original<Path>>,
                &Affiliation,
                &PreferredSemiTransparency,
            ),
            (With<FloorMarker>, Without<Pending>),
        >,
        Query<(&LightKind, &Pose)>,
        Query<
            (&Edge, Option<&Original<Edge>>, &Distance),
            (With<MeasurementMarker>, Without<Pending>),
        >,
        Query<(&NameInSite, &Pose, &PhysicalCameraProperties), Without<Pending>>,
        Query<(&Edge, Option<&Original<Edge>>, &Affiliation), (With<WallMarker>, Without<Pending>)>,
        Query<
            (
                &NameInSite,
                &LevelElevation,
                &GlobalFloorVisibility,
                &GlobalDrawingVisibility,
                &Children,
                Option<&RecencyRanking<FloorMarker>>,
                Option<&RecencyRanking<DrawingMarker>>,
            ),
            Without<Pending>,
        >,
        Query<(&Pose, &NameInSite), With<UserCameraPoseMarker>>,
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
        q_physical_cameras,
        q_walls,
        q_levels,
        q_user_camera_poses,
    ) = state.get(world);

    let validate_anchor = |entity| {
        let _ = q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))?;
        Ok(())
    };

    let validate_group = |entity| {
        q_groups
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAffiliation(entity))
    };

    let validate_edge = |edge: &Edge| {
        validate_anchor(*edge.left())?;
        validate_anchor(*edge.right())
    };

    let validate_path = |path: &Path| {
        for entity in path.0.iter() {
            validate_anchor(**entity)?;
        }
        Ok(())
    };

    let mut levels = BTreeMap::new();
    if let Ok(site_children) = q_site_children.get(site) {
        for c in site_children.iter() {
            if let Ok((
                name,
                elevation,
                floor_vis,
                drawing_vis,
                level_children,
                floor_ranking,
                drawing_ranking,
            )) = q_levels.get(c)
            {
                let mut level = Level::new(
                    LevelProperties {
                        name: name.clone(),
                        elevation: elevation.clone(),
                        global_floor_visibility: floor_vis.clone(),
                        global_drawing_visibility: drawing_vis.clone(),
                    },
                    // TODO(luca) validation for rankings?
                    RankingsInLevel {
                        floors: floor_ranking
                            .map(|r| r.entities().iter().map(|e| (*e).into()).collect())
                            .unwrap_or(Vec::new()),
                        drawings: drawing_ranking
                            .map(|r| r.entities().iter().map(|e| (*e).into()).collect())
                            .unwrap_or(Vec::new()),
                    },
                );
                for c in level_children.iter() {
                    if let Ok(anchor) = q_anchors.get(c) {
                        level.anchors.insert(c.into(), anchor.clone());
                    }
                    if let Ok((edge, o_edge, name, kind)) = q_doors.get(c) {
                        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                        validate_edge(edge)?;
                        level.doors.insert(
                            c.into(),
                            Door {
                                anchors: edge.clone(),
                                name: name.clone(),
                                kind: kind.clone(),
                                marker: DoorMarker,
                            },
                        );
                    }
                    if let Ok((name, source, pose, pixels_per_meter, preferred_alpha, children)) =
                        q_drawings.get(c)
                    {
                        let mut measurements = BTreeMap::new();
                        let mut fiducials = BTreeMap::new();
                        let mut anchors = BTreeMap::new();
                        for e in children.iter() {
                            if let Ok(anchor) = q_anchors.get(e) {
                                anchors.insert(e.into(), anchor.clone());
                            }
                            if let Ok((edge, o_edge, distance)) = q_measurements.get(e) {
                                let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                                validate_edge(edge)?;
                                measurements.insert(
                                    e.into(),
                                    Measurement {
                                        anchors: edge.clone(),
                                        distance: distance.clone(),
                                        marker: MeasurementMarker,
                                    },
                                );
                            }
                            if let Ok((point, o_point, affiliation)) = q_fiducials.get(e) {
                                let point = o_point.map(|x| &x.0).unwrap_or(point);
                                validate_anchor(***point)?;
                                if let Affiliation(Some(e)) = affiliation {
                                    validate_group(**e)?;
                                }
                                fiducials.insert(
                                    e.into(),
                                    Fiducial {
                                        anchor: point.clone(),
                                        affiliation: affiliation.clone(),
                                        marker: FiducialMarker,
                                    },
                                );
                            }
                        }
                        level.drawings.insert(
                            c.into(),
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
                    if let Ok((path, o_path, texture, preferred_alpha)) = q_floors.get(c) {
                        let path = o_path.map(|x| &x.0).unwrap_or(path);
                        validate_path(&path)?;
                        if let Affiliation(Some(e)) = texture {
                            validate_group(**e)?;
                        }

                        level.floors.insert(
                            c.into(),
                            Floor {
                                anchors: path.clone(),
                                texture: texture.clone(),
                                preferred_semi_transparency: preferred_alpha.clone(),
                                marker: FloorMarker,
                            },
                        );
                    }
                    if let Ok((kind, pose)) = q_lights.get(c) {
                        level.lights.insert(
                            c.into(),
                            Light {
                                pose: pose.clone(),
                                kind: kind.clone(),
                            },
                        );
                    }
                    if let Ok((name, pose, properties)) = q_physical_cameras.get(c) {
                        level.physical_cameras.insert(
                            c.into(),
                            PhysicalCamera {
                                name: name.clone(),
                                pose: pose.clone(),
                                properties: properties.clone(),
                                previewable: PreviewableMarker,
                            },
                        );
                    }
                    if let Ok((edge, o_edge, texture)) = q_walls.get(c) {
                        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                        validate_edge(edge)?;
                        if let Affiliation(Some(e)) = texture {
                            validate_group(**e)?;
                        }

                        level.walls.insert(
                            c.into(),
                            Wall {
                                anchors: edge.clone(),
                                texture: texture.clone(),
                                marker: WallMarker,
                            },
                        );
                    }
                    if let Ok((pose, name)) = q_user_camera_poses.get(c) {
                        level.user_camera_poses.insert(
                            c.into(),
                            UserCameraPose {
                                name: name.clone(),
                                pose: pose.clone(),
                                marker: UserCameraPoseMarker,
                            },
                        );
                    }
                }
                levels.insert(c.into(), level);
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
        &'static Edge,
        Option<&'static Original<Edge>>,
        &'static LiftCabin,
        &'static IsStatic,
        &'static InitialLevel,
        &'static ChildOf,
    ),
    Without<Pending>,
>;

fn generate_lifts(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<SiteID, Lift>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<&Anchor, Without<Pending>>,
        QueryLiftDoor,
        Query<(), (With<LevelElevation>, Without<Pending>)>,
        QueryLift,
        Query<Entity, With<CabinAnchorGroup>>,
        Query<&ChildOf, Without<Pending>>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_anchors, q_doors, q_levels, q_lifts, q_cabin_anchor_groups, q_child_of, q_children) =
        state.get(world);

    let mut lifts = BTreeMap::new();

    let is_valid_anchor = |entity| {
        let _ = q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))?;
        Ok(())
    };

    let validate_level = |entity| {
        let _ = q_levels
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenLevelReference(entity))?;
        Ok(SiteID::from(entity))
    };

    let validate_edge = |edge: &Edge| {
        is_valid_anchor(*edge.left())?;
        is_valid_anchor(*edge.right())?;
        Ok(())
    };

    let confirm_entity_parent = |intended_parent, child| {
        if let Ok(actual_parent) = q_child_of.get(child) {
            if actual_parent.parent() == intended_parent {
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
            site: site.into(),
            anchor: anchor.into(),
        })
    };

    let validate_site_anchors = |edge: &Edge| {
        validate_site_anchor(*edge.left())?;
        validate_site_anchor(*edge.right())
    };

    for (lift_entity, name, edge, o_edge, cabin, is_static, initial_level, child_of) in &q_lifts {
        if child_of.parent() != site {
            continue;
        }

        let Ok(children) = q_children.get(lift_entity) else {
            return Err(SiteGenerationError::BrokenLift(lift_entity));
        };

        let Some(anchor_group_entity) =
            children.iter().find(|c| q_cabin_anchor_groups.contains(*c))
        else {
            return Err(SiteGenerationError::BrokenLift(lift_entity));
        };

        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        validate_site_anchors(edge)?;

        let validate_level_door_anchor = |door: Entity, anchor: Entity| {
            if confirm_entity_parent(anchor_group_entity, anchor) {
                return Ok(());
            }

            Err(SiteGenerationError::InvalidLiftDoorReference { door, anchor })
        };

        let validate_level_door_anchors = |door: Entity, edge: &Edge| {
            validate_level_door_anchor(door, *edge.left())?;
            validate_level_door_anchor(door, *edge.right())?;
            validate_edge(edge)
        };

        let mut cabin_anchors = BTreeMap::new();
        let mut cabin_doors = BTreeMap::new();
        for child in children {
            // TODO(luca) this is repeated with the above?
            if let Ok(anchor_group) = q_cabin_anchor_groups.get(*child) {
                if let Ok(anchor_children) = q_children.get(anchor_group) {
                    for anchor_child in anchor_children {
                        if let Ok(anchor) = q_anchors.get(*anchor_child) {
                            cabin_anchors.insert((*anchor_child).into(), anchor.clone());
                        }
                    }
                }
            }

            if let Ok((door_type, edge, o_edge, visits)) = q_doors.get(*child) {
                let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
                validate_level_door_anchors(*child, edge)?;
                cabin_doors.insert(
                    (*child).into(),
                    LiftCabinDoor {
                        kind: door_type.clone(),
                        reference_anchors: edge.clone(),
                        visits: LevelVisits(
                            visits
                                .iter()
                                .map(|level| validate_level(**level))
                                .collect::<Result<_, _>>()?,
                        ),
                        marker: Default::default(),
                    },
                );
            }
        }

        validate_edge(edge)?;
        lifts.insert(
            lift_entity.into(),
            Lift {
                cabin_doors,
                properties: LiftProperties {
                    name: name.clone(),
                    reference_anchors: edge.clone(),
                    cabin: cabin.clone(),
                    is_static: is_static.clone(),
                    initial_level: InitialLevel(
                        initial_level
                            .0
                            .map_or(Ok(None), |level| validate_level(*level).map(|id| Some(id)))?,
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
) -> Result<BTreeMap<SiteID, Fiducial>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(), (With<Anchor>, Without<Pending>)>,
        Query<(), (With<Group>, Without<Pending>)>,
        Query<(&Point, &Affiliation), (With<FiducialMarker>, Without<Pending>)>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_anchor_ids, q_group_ids, q_fiducials, q_children) = state.get(world);

    let Ok(children) = q_children.get(parent) else {
        return Ok(BTreeMap::new());
    };

    let mut fiducials = BTreeMap::new();
    for child in children {
        let Ok((point, affiliation)) = q_fiducials.get(*child) else {
            continue;
        };
        q_anchor_ids
            .get(***point)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(***point))?;
        let affiliation = if let Some(e) = affiliation.0 {
            q_group_ids
                .get(*e)
                .map_err(|_| SiteGenerationError::BrokenAffiliation(*e))?;
            Affiliation(Some(e.into()))
        } else {
            Affiliation(None)
        };

        fiducials.insert(
            (*child).into(),
            Fiducial {
                anchor: point.clone(),
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
) -> Result<BTreeMap<SiteID, FiducialGroup>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<&NameInSite, (With<Group>, With<FiducialMarker>)>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_groups, q_children) = state.get(world);

    let Ok(children) = q_children.get(parent) else {
        return Ok(BTreeMap::new());
    };

    let mut fiducial_groups = BTreeMap::new();
    for child in children {
        let Ok(name) = q_groups.get(*child) else {
            continue;
        };
        fiducial_groups.insert((*child).into(), FiducialGroup::new(name.clone()));
    }

    Ok(fiducial_groups)
}

fn generate_texture_groups(
    world: &mut World,
    parent: Entity,
) -> Result<BTreeMap<SiteID, TextureGroup>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(&NameInSite, &Texture), With<Group>>,
        Query<&Children>,
    )> = SystemState::new(world);

    let (q_groups, q_children) = state.get(world);

    let Ok(children) = q_children.get(parent) else {
        return Ok(BTreeMap::new());
    };

    let mut texture_groups = BTreeMap::new();
    for child in children {
        let Ok((name, texture)) = q_groups.get(*child) else {
            continue;
        };
        texture_groups.insert(
            (*child).into(),
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
) -> Result<BTreeMap<SiteID, NavGraph>, SiteGenerationError> {
    let mut state: SystemState<
        Query<
            (Entity, &NameInSite, &DisplayColor, &ChildOf),
            (With<NavGraphMarker>, Without<Pending>),
        >,
    > = SystemState::new(world);

    let q_nav_graphs = state.get(world);

    let mut nav_graphs = BTreeMap::new();
    for (e, name, color, child_of) in &q_nav_graphs {
        if child_of.parent() != site {
            continue;
        }

        nav_graphs.insert(
            e.into(),
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
) -> Result<BTreeMap<SiteID, Lane>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<
            (
                Entity,
                &Edge,
                Option<&Original<Edge>>,
                &Motion,
                &ReverseLane,
                &AssociatedGraphs,
                &ChildOf,
            ),
            (With<LaneMarker>, Without<Pending>),
        >,
        Query<(), With<Anchor>>,
    )> = SystemState::new(world);

    let (q_lanes, q_anchors) = state.get(world);

    let validate_anchor = |entity| {
        q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))
    };

    let validate_edge = |edge: &Edge| {
        validate_anchor(*edge.left())?;
        validate_anchor(*edge.right())
    };

    let mut lanes = BTreeMap::new();
    for (e, edge, o_edge, forward, reverse, graphs, child_of) in &q_lanes {
        if child_of.parent() != site {
            continue;
        }

        let edge = o_edge.map(|x| &x.0).unwrap_or(edge);
        validate_edge(edge)?;

        lanes.insert(
            e.into(),
            Lane {
                anchors: edge.clone(),
                forward: forward.clone(),
                reverse: reverse.clone(),
                graphs: graphs.clone(),
                marker: LaneMarker,
            },
        );
    }

    Ok(lanes)
}

fn generate_locations(
    world: &mut World,
    site: Entity,
) -> Result<BTreeMap<SiteID, Location>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<
            (
                Entity,
                &Point,
                Option<&Original<Point>>,
                &LocationTags,
                &NameInSite,
                &AssociatedGraphs,
                &ChildOf,
            ),
            Without<Pending>,
        >,
        Query<(), With<Anchor>>,
    )> = SystemState::new(world);

    let (q_locations, q_anchors) = state.get(world);

    let validate_anchor = |entity| {
        q_anchors
            .get(entity)
            .map_err(|_| SiteGenerationError::BrokenAnchorReference(entity))
    };

    let mut locations = BTreeMap::new();
    for (e, point, o_point, tags, name, graphs, child_of) in &q_locations {
        if child_of.parent() != site {
            continue;
        }

        let point = o_point.map(|x| &x.0).unwrap_or(point);
        validate_anchor(***point)?;

        locations.insert(
            e.into(),
            Location {
                anchor: point.clone(),
                tags: tags.clone(),
                name: name.clone(),
                graphs: graphs.clone(),
            },
        );
    }

    Ok(locations)
}

fn generate_graph_rankings(
    world: &mut World,
    site: Entity,
) -> Result<Vec<SiteID>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<&RecencyRanking<NavGraphMarker>>,
        Query<(), With<NavGraphMarker>>,
    )> = SystemState::new(world);

    let (rankings, nav_graphs) = state.get(world);
    let Ok(ranking) = rankings.get(site) else {
        return Ok(Vec::new());
    };

    ranking
        .entities()
        .iter()
        .map(|e| {
            nav_graphs
                .get(*e)
                .map_err(|_| SiteGenerationError::BrokenNavGraphReference(*e))?;
            Ok((*e).into())
        })
        .collect()
}

fn generate_site_properties(
    world: &mut World,
    site: Entity,
) -> Result<SiteProperties, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(
            &NameOfSite,
            &FilteredIssues,
            &FilteredIssueKinds,
            &GeographicComponent,
        )>,
        Query<&Issue>,
    )> = SystemState::new(world);

    let (q_properties, q_issues) = state.get(world);

    let Ok((name, issues, issue_kinds, geographic_offset)) = q_properties.get(site) else {
        return Err(SiteGenerationError::InvalidSiteEntity(site));
    };

    let mut converted_issues = BTreeSet::new();
    for issue in issues.iter() {
        let mut entities = BTreeSet::new();
        for e in issue.entities.iter() {
            q_issues
                .get(**e)
                .map_err(|_| SiteGenerationError::BrokenIssueReference(**e))?;
            entities.insert((*e).into());
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
    // child_of: Query<&ChildOf>,
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

    let mut state: SystemState<(Query<(Entity, &mut AssetSource)>, Query<&ChildOf>)> =
        SystemState::new(world);

    let (mut assets, child_of) = state.get_mut(world);

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

            if let Ok(child_of) = child_of.get(e) {
                e = child_of.parent();
            } else {
                break;
            }
        }
    }
}

fn generate_model_descriptions(
    site: Entity,
    world: &mut World,
) -> Result<BTreeMap<SiteID, ModelDescriptionBundle>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<
            (
                &NameInSite,
                &ModelProperty<AssetSource>,
                &ModelProperty<IsStatic>,
                &ModelProperty<Scale>,
            ),
            (With<ModelMarker>, With<Group>, Without<Pending>),
        >,
        Query<&Children>,
    )> = SystemState::new(world);
    let (model_descriptions, children) = state.get(world);

    let mut res = BTreeMap::<SiteID, ModelDescriptionBundle>::new();
    if let Ok(children) = children.get(site) {
        for child in children.iter() {
            if let Ok((name, source, is_static, scale)) = model_descriptions.get(child) {
                let desc_bundle = ModelDescriptionBundle {
                    name: name.clone(),
                    source: source.clone(),
                    is_static: is_static.clone(),
                    scale: scale.clone(),
                    ..Default::default()
                };
                res.insert(child.into(), desc_bundle);
            }
        }
    }
    Ok(res)
}

fn generate_robots(
    site: Entity,
    world: &mut World,
) -> Result<BTreeMap<SiteID, Robot>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<&ModelProperty<Robot>, (With<ModelMarker>, With<Group>, Without<Pending>)>,
        Query<&Children>,
    )> = SystemState::new(world);
    let (robots, children) = state.get(world);

    let mut res = BTreeMap::<SiteID, Robot>::new();
    if let Ok(children) = children.get(site) {
        for child in children.iter() {
            if let Ok(robot_property) = robots.get(child) {
                let mut robot = robot_property.0.clone();
                // Remove any invalid properties
                robot.properties.retain(|k, _| !k.is_empty());
                res.insert(child.into(), robot);
            }
        }
    }
    Ok(res)
}

fn generate_model_instances(
    site: Entity,
    world: &mut World,
) -> Result<BTreeMap<SiteID, Parented<ModelInstance>>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<&ExportWith, (With<ModelMarker>, With<Group>, Without<Pending>)>,
        Query<
            (Entity, &NameInSite, &Pose, &Affiliation),
            (With<ModelMarker>, Without<Group>, Without<Pending>),
        >,
        Query<Entity, With<LevelElevation>>,
        Query<&ChildOf>,
    )> = SystemState::new(world);
    let (model_descriptions, model_instances, levels, child_of) = state.get(world);

    let mut site_levels_ids = HashSet::<Entity>::new();
    for level_entity in levels.iter() {
        if child_of
            .get(level_entity)
            .is_ok_and(|co| co.parent() == site)
        {
            site_levels_ids.insert(level_entity);
        }
    }
    // Store model instance data in a HashMap for later access with mutable World
    let mut model_instances_data = HashMap::<
        Entity,
        (
            NameInSite,
            Pose,
            SiteID,
            Affiliation,
            HashMap<String, serde_json::Value>,
        ),
    >::new();
    for (instance_entity, instance_name, instance_pose, instance_affiliation) in
        model_instances.iter()
    {
        let Some(level_id) = child_of
            .get(instance_entity)
            .ok()
            .map(|co| site_levels_ids.get(&co.parent()).copied())
            .flatten()
        else {
            error!("Unable to find parent for instance [{}]", instance_name.0);
            continue;
        };
        let description_export = instance_affiliation
            .0
            .and_then(|e| model_descriptions.get(*e).ok());

        model_instances_data.insert(
            instance_entity,
            (
                instance_name.clone(),
                instance_pose.clone(),
                level_id.into(),
                instance_affiliation.clone(),
                description_export
                    .map(|e| e.0.clone())
                    .unwrap_or(HashMap::new()),
            ),
        );
    }

    let mut res = BTreeMap::<SiteID, Parented<ModelInstance>>::new();
    for (entity, (name, pose, level_id, description, description_export)) in
        model_instances_data.into_iter()
    {
        let mut export_data = HashMap::<String, sdformat_rs::XmlElement>::new();
        for (label, value) in description_export.iter() {
            if let Some(data) = world
                .resource_scope::<ExportHandlers, Option<sdformat_rs::XmlElement>>(
                    move |world, mut export_handlers| {
                        if let Some(export_handler) = export_handlers.get_mut(label) {
                            export_handler.export(entity, value.clone(), world)
                        } else {
                            None
                        }
                    },
                )
            {
                export_data.insert(label.clone(), data);
            }
        }
        let model_instance = ModelInstance {
            name,
            pose,
            description,
            export_data: ExportData(export_data),
            ..Default::default()
        };
        res.insert(
            entity.into(),
            Parented {
                parent: level_id.into(),
                bundle: model_instance,
            },
        );
    }
    Ok(res)
}

fn generate_scenarios(
    site: Entity,
    world: &mut World,
) -> Result<BTreeMap<SiteID, Scenario>, SiteGenerationError> {
    let mut state: SystemState<(
        Query<(Entity, &NameInSite, &Affiliation), With<ScenarioMarker>>,
        Query<(
            Option<&Modifier<Pose>>,
            Option<&Modifier<Visibility>>,
            &Affiliation,
        )>,
        Query<Entity, With<InstanceMarker>>,
        Query<(
            Option<&Modifier<Inclusion>>,
            Option<&Modifier<TaskParams>>,
            &Affiliation,
        )>,
        Query<Entity, (With<Task>, Without<Pending>)>,
        Query<&Children>,
    )> = SystemState::new(world);
    let (scenarios, instance_modifiers, instances, task_modifiers, tasks, children) =
        state.get(world);
    let mut res = BTreeMap::<SiteID, Scenario>::new();

    if let Ok(site_children) = children.get(site) {
        for site_child in site_children.iter() {
            if let Ok((entity, ..)) = scenarios.get(site_child) {
                let mut queue = vec![entity];

                while let Some(scenario) = queue.pop() {
                    let mut scenario_instance_modifiers = Vec::new();
                    let mut scenario_task_modifiers = Vec::new();
                    if let Ok(scenario_children) = children.get(scenario) {
                        for scenario_child in scenario_children.iter() {
                            if scenarios.contains(scenario_child) {
                                queue.push(scenario_child);
                            } else if instance_modifiers
                                .get(scenario_child)
                                .is_ok_and(|(p, v, _)| p.is_some() || v.is_some())
                            {
                                scenario_instance_modifiers.push(scenario_child);
                            } else if task_modifiers
                                .get(scenario_child)
                                .is_ok_and(|(i, p, _)| i.is_some() || p.is_some())
                            {
                                scenario_task_modifiers.push(scenario_child);
                            }
                        }
                    }

                    if let Ok((entity, name, parent_scenario)) = scenarios.get(scenario) {
                        res.insert(
                            entity.into(),
                            Scenario {
                                instances: scenario_instance_modifiers
                                    .iter()
                                    .filter_map(|child_entity| {
                                        instance_modifiers.get(*child_entity).ok()
                                    })
                                    .filter_map(|(pose, visibility, affiliation)| {
                                        Some((
                                            affiliation.0.and_then(|e| {
                                                instances.get(*e).ok().map(|e| e.into())
                                            })?,
                                            InstanceModifier {
                                                pose: pose.map(|p| **p),
                                                visibility: visibility.map(|v| match **v {
                                                    Visibility::Hidden => false,
                                                    _ => true,
                                                }),
                                            },
                                        ))
                                    })
                                    .collect(),
                                tasks: scenario_task_modifiers
                                    .iter()
                                    .filter_map(|child_entity| {
                                        task_modifiers.get(*child_entity).ok()
                                    })
                                    .filter_map(|(inclusion, task_params, affiliation)| {
                                        Some((
                                            affiliation.0.and_then(|e| {
                                                tasks.get(*e).ok().map(|e| e.into())
                                            })?,
                                            TaskModifier {
                                                inclusion: inclusion.map(|i| **i),
                                                params: task_params.map(|p| (**p).clone()),
                                            },
                                        ))
                                    })
                                    .collect(),
                                properties: ScenarioBundle {
                                    name: name.clone(),
                                    // TODO(luca) validate parent scenario
                                    parent_scenario: parent_scenario.clone(),
                                    marker: ScenarioMarker,
                                },
                            },
                        );
                    }
                }
            }
        }
    }
    info!("Added scenarios: {:?}", res.len());
    Ok(res)
}

fn generate_tasks(
    site: Entity,
    world: &mut World,
) -> Result<BTreeMap<SiteID, Task>, SiteGenerationError> {
    let mut state: SystemState<(Query<&Task, Without<Pending>>, Query<&Children>)> =
        SystemState::new(world);
    let (tasks, children) = state.get(world);
    let mut res = BTreeMap::<SiteID, Task>::new();
    if let Ok(children) = children.get(site) {
        for child in children.iter() {
            if let Ok(task) = tasks.get(child) {
                res.insert(child.into(), task.clone());
            }
        }
    }
    Ok(res)
}

pub fn generate_site(
    world: &mut World,
    site: Entity,
) -> Result<rmf_site_format::Site, SiteGenerationError> {
    assemble_edited_drawing(world);

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
    let model_descriptions = generate_model_descriptions(site, world)?;
    let robots = generate_robots(site, world)?;
    let model_instances = generate_model_instances(site, world)?;
    let scenarios = generate_scenarios(site, world)?;
    let tasks = generate_tasks(site, world)?;

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
        model_descriptions,
        robots,
        model_instances,
        scenarios,
        tasks,
    });
}

pub fn save_site(world: &mut World) {
    let save_events: Vec<_> = world.resource_mut::<Events<SaveSite>>().drain().collect();
    for save_event in save_events {
        let mut new_path = dbg!(save_event.to_location);
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
                    new_path = path_str.replace(".building.yaml", ".site.json").into();
                } else if path_str.ends_with(".site.ron") {
                    // Noop, we allow .site.ron to remain as-is
                } else if !path_str.ends_with(".site.json") {
                    info!("Appending .site.json to {}", new_path.display());
                    new_path = new_path.with_extension("site.json");
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
                            continue;
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
                            continue;
                        }
                    }
                }

                // Indicate that the site has not changed since the last save.
                // Note that we will need to change this logic when we start
                // supporting multiple sites being open in one app.
                world.resource_mut::<SiteChanged>().0 = false;
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
                if !new_path.exists() {
                    if let Err(e) = std::fs::create_dir_all(&new_path) {
                        error!("Unable to create folder {}: {e}", new_path.display());
                        continue;
                    }
                } else {
                    if !new_path.is_dir() {
                        error!("SDF can only be exported to a folder");
                        continue;
                    }
                }
                let mut sdf_path = new_path.clone();
                sdf_path.push(&site.properties.name.0);
                sdf_path.set_extension("world");
                let f = match std::fs::File::create(&sdf_path) {
                    Ok(f) => f,
                    Err(err) => {
                        error!("Unable to save file {}: {err}", sdf_path.display());
                        continue;
                    }
                };

                let mut meshes_dir = new_path.clone();
                meshes_dir.push("meshes");
                if let Err(e) = std::fs::create_dir_all(&meshes_dir) {
                    error!("Unable to create folder {}: {e}", meshes_dir.display());
                    continue;
                }
                if let Err(e) = collect_site_meshes(world, save_event.site, &meshes_dir) {
                    error!("Unable to collect site meshes: {e}");
                    continue;
                }

                migrate_relative_paths(save_event.site, &sdf_path, world);
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
            }
            ExportFormat::NavGraph => {
                let site = match generate_site(world, save_event.site) {
                    Ok(site) => site,
                    Err(err) => {
                        error!("Unable to compile site: {err}");
                        continue;
                    }
                };

                dbg!(&new_path);
                for (name, nav_graph) in legacy::nav_graph::NavGraph::from_site(&site) {
                    let graph_file = new_path.clone().join(name + ".nav.yaml");
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

                info!(
                    "Saving all site nav graphs to {}",
                    new_path.to_str().unwrap_or("<failed to render??>")
                );
            }
        }
    }
}
