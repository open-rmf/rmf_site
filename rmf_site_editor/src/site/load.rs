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

use crate::{recency::RecencyRanking, site::*, WorkspaceMarker};
use bevy::{ecs::system::SystemParam, prelude::*};
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error as ThisError;

/// This component is given to the site to keep track of what file it should be
/// saved to by default.
#[derive(Component, Clone, Debug, Deref)]
pub struct DefaultFile(pub PathBuf);

#[derive(Event, Clone)]
pub struct LoadSite {
    /// The site data to load
    pub site: rmf_site_format::Site,
    /// Should the application switch focus to this new site
    pub focus: bool,
    /// The default file path that should be assigned to the site
    pub default_file: Option<PathBuf>,
}

impl LoadSite {
    #[allow(non_snake_case)]
    pub fn blank_L1(name: String, default_file: Option<PathBuf>) -> Self {
        Self {
            site: rmf_site_format::Site::blank_L1(name),
            default_file,
            focus: true,
        }
    }
}

#[derive(ThisError, Debug)]
#[error("The site has a broken internal reference: {broken}")]
struct LoadSiteError {
    site: Entity,
    broken: u32,
    // TODO(@mxgrey): reintroduce Backtrack when it's supported on stable
    // backtrace: Backtrace,
}

impl LoadSiteError {
    fn new(site: Entity, broken: u32) -> Self {
        Self { site, broken }
    }
}

trait LoadResult<T> {
    fn for_site(self, site: Entity) -> Result<T, LoadSiteError>;
}

impl<T> LoadResult<T> for Result<T, u32> {
    fn for_site(self, site: Entity) -> Result<T, LoadSiteError> {
        self.map_err(|broken| LoadSiteError::new(site, broken))
    }
}

fn generate_site_entities(
    commands: &mut Commands,
    site_data: &rmf_site_format::Site,
) -> Result<Entity, LoadSiteError> {
    let mut id_to_entity = HashMap::new();
    let mut highest_id = 0_u32;
    let mut consider_id = |consider| {
        if consider > highest_id {
            highest_id = consider;
        }
    };

    let site_id = commands
        .spawn(SpatialBundle::HIDDEN_IDENTITY)
        .insert(Category::Site)
        .insert(WorkspaceMarker)
        .id();

    for (anchor_id, anchor) in &site_data.anchors {
        let anchor_entity = commands
            .spawn(AnchorBundle::new(anchor.clone()))
            .insert(SiteID(*anchor_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*anchor_id, anchor_entity);
        consider_id(*anchor_id);
    }

    for (group_id, group) in &site_data.fiducial_groups {
        let group_entity = commands
            .spawn(group.clone())
            .insert(SiteID(*group_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*group_id, group_entity);
        consider_id(*group_id);
    }

    for (group_id, group) in &site_data.textures {
        let group_entity = commands
            .spawn(group.clone())
            .insert(SiteID(*group_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*group_id, group_entity);
        consider_id(*group_id);
    }

    for (model_description_id, model_description) in &site_data.model_descriptions {
        let model_description = commands
            .spawn(model_description.clone())
            .insert(SiteID(*model_description_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*model_description_id, model_description);
        consider_id(*model_description_id);
    }

    let (_, default_scenario) = site_data.scenarios.first_key_value().unwrap();

    for (level_id, level_data) in &site_data.levels {
        let level_entity = commands.spawn(SiteID(*level_id)).set_parent(site_id).id();

        for (anchor_id, anchor) in &level_data.anchors {
            let anchor_entity = commands
                .spawn(AnchorBundle::new(anchor.clone()))
                .insert(SiteID(*anchor_id))
                .set_parent(level_entity)
                .id();
            id_to_entity.insert(*anchor_id, anchor_entity);
            consider_id(*anchor_id);
        }

        for (door_id, door) in &level_data.doors {
            let door_entity = commands
                .spawn(door.convert(&id_to_entity).for_site(site_id)?)
                .insert(SiteID(*door_id))
                .set_parent(level_entity)
                .id();
            id_to_entity.insert(*door_id, door_entity);
            consider_id(*door_id);
        }

        for (drawing_id, drawing) in &level_data.drawings {
            let drawing_entity = commands
                .spawn(DrawingBundle::new(drawing.properties.clone()))
                .insert(SiteID(*drawing_id))
                .set_parent(level_entity)
                .id();

            for (anchor_id, anchor) in &drawing.anchors {
                let anchor_entity = commands
                    .spawn(AnchorBundle::new(anchor.clone()))
                    .insert(SiteID(*anchor_id))
                    .set_parent(drawing_entity)
                    .id();
                id_to_entity.insert(*anchor_id, anchor_entity);
                consider_id(*anchor_id);
            }

            for (fiducial_id, fiducial) in &drawing.fiducials {
                let fiducial_entity = commands
                    .spawn(fiducial.convert(&id_to_entity).for_site(site_id)?)
                    .insert(SiteID(*fiducial_id))
                    .set_parent(drawing_entity)
                    .id();
                id_to_entity.insert(*fiducial_id, fiducial_entity);
                consider_id(*fiducial_id);
            }

            for (measurement_id, measurement) in &drawing.measurements {
                let measurement_entity = commands
                    .spawn(measurement.convert(&id_to_entity).for_site(site_id)?)
                    .insert(SiteID(*measurement_id))
                    .set_parent(drawing_entity)
                    .id();
                id_to_entity.insert(*measurement_id, measurement_entity);
                consider_id(*measurement_id);
            }

            consider_id(*drawing_id);
        }

        for (floor_id, floor) in &level_data.floors {
            commands
                .spawn(floor.convert(&id_to_entity).for_site(site_id)?)
                .insert(SiteID(*floor_id))
                .set_parent(level_entity);
            consider_id(*floor_id);
        }

        for (wall_id, wall) in &level_data.walls {
            commands
                .spawn(wall.convert(&id_to_entity).for_site(site_id)?)
                .insert(SiteID(*wall_id))
                .set_parent(level_entity);
            consider_id(*wall_id);
        }

        commands
            .entity(level_entity)
            .insert(SpatialBundle::HIDDEN_IDENTITY)
            .insert(level_data.properties.clone())
            .insert(Category::Level)
            .with_children(|level| {
                // These don't need a return value so can be wrapped in a with_children
                for (light_id, light) in &level_data.lights {
                    level.spawn(light.clone()).insert(SiteID(*light_id));
                    consider_id(*light_id);
                }

                for (model_instance_id, model_instance) in &default_scenario.model_instances {
                    if model_instance.parent.0 == *level_id {
                        level
                            .spawn(model_instance.clone())
                            .insert(SiteID(*model_instance_id));
                        consider_id(*model_instance_id);
                    }
                }

                for (physical_camera_id, physical_camera) in &level_data.physical_cameras {
                    level
                        .spawn(physical_camera.clone())
                        .insert(SiteID(*physical_camera_id));
                    consider_id(*physical_camera_id);
                }

                for (camera_pose_id, camera_pose) in &level_data.user_camera_poses {
                    level
                        .spawn(camera_pose.clone())
                        .insert(SiteID(*camera_pose_id));
                    consider_id(*camera_pose_id);
                }
            });

        // TODO(MXG): Log when a RecencyRanking fails to load correctly.
        commands
            .entity(level_entity)
            .insert(
                RecencyRanking::<FloorMarker>::from_u32(&level_data.rankings.floors, &id_to_entity)
                    .unwrap_or(RecencyRanking::new()),
            )
            .insert(
                RecencyRanking::<DrawingMarker>::from_u32(
                    &level_data.rankings.drawings,
                    &id_to_entity,
                )
                .unwrap_or(RecencyRanking::new()),
            );
        id_to_entity.insert(*level_id, level_entity);
        consider_id(*level_id);
    }

    for (lift_id, lift_data) in &site_data.lifts {
        let lift_entity = commands.spawn(SiteID(*lift_id)).set_parent(site_id).id();

        commands.entity(lift_entity).with_children(|lift| {
            lift.spawn(SpatialBundle::default())
                .insert(CabinAnchorGroupBundle::default())
                .with_children(|anchor_group| {
                    for (anchor_id, anchor) in &lift_data.cabin_anchors {
                        let anchor_entity = anchor_group
                            .spawn(AnchorBundle::new(anchor.clone()))
                            .insert(SiteID(*anchor_id))
                            .id();
                        id_to_entity.insert(*anchor_id, anchor_entity);
                        consider_id(*anchor_id);
                    }
                });
        });

        for (door_id, door) in &lift_data.cabin_doors {
            let door_entity = commands
                .spawn(door.convert(&id_to_entity).for_site(site_id)?)
                .insert(Dependents::single(lift_entity))
                .set_parent(lift_entity)
                .id();
            id_to_entity.insert(*door_id, door_entity);
            consider_id(*door_id);
        }

        commands.entity(lift_entity).insert(Category::Lift).insert(
            lift_data
                .properties
                .convert(&id_to_entity)
                .for_site(site_id)?,
        );

        id_to_entity.insert(*lift_id, lift_entity);
        consider_id(*lift_id);
    }

    for (fiducial_id, fiducial) in &site_data.fiducials {
        let fiducial_entity = commands
            .spawn(fiducial.convert(&id_to_entity).for_site(site_id)?)
            .insert(SiteID(*fiducial_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*fiducial_id, fiducial_entity);
        consider_id(*fiducial_id);
    }

    for (nav_graph_id, nav_graph_data) in &site_data.navigation.guided.graphs {
        let nav_graph = commands
            .spawn(SpatialBundle::default())
            .insert(nav_graph_data.clone())
            .insert(SiteID(*nav_graph_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*nav_graph_id, nav_graph);
        consider_id(*nav_graph_id);
    }

    for (lane_id, lane_data) in &site_data.navigation.guided.lanes {
        let lane = commands
            .spawn(lane_data.convert(&id_to_entity).for_site(site_id)?)
            .insert(SiteID(*lane_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*lane_id, lane);
        consider_id(*lane_id);
    }

    for (location_id, location_data) in &site_data.navigation.guided.locations {
        let location = commands
            .spawn(location_data.convert(&id_to_entity).for_site(site_id)?)
            .insert(SiteID(*location_id))
            .set_parent(site_id)
            .id();
        id_to_entity.insert(*location_id, location);
        consider_id(*location_id);
    }
    // Properties require the id_to_entity map to be fully populated to load suppressed issues
    commands.entity(site_id).insert(
        site_data
            .properties
            .convert(&id_to_entity)
            .for_site(site_id)?,
    );

    let nav_graph_rankings = match RecencyRanking::<NavGraphMarker>::from_u32(
        &site_data.navigation.guided.ranking,
        &id_to_entity,
    ) {
        Ok(r) => r,
        Err(id) => {
            error!(
                "ERROR: Nav Graph ranking could not load because a graph with \
                id {id} does not exist."
            );
            RecencyRanking::new()
        }
    };

    commands
        .entity(site_id)
        .insert(nav_graph_rankings)
        .insert(NextSiteID(highest_id + 1));

    // Make the lift cabin anchors that are used by doors subordinate
    for (lift_id, lift_data) in &site_data.lifts {
        for (_, door) in &lift_data.cabin_doors {
            for anchor in door.reference_anchors.array() {
                commands
                    .entity(*id_to_entity.get(&anchor).ok_or(anchor).for_site(site_id)?)
                    .insert(Subordinate(Some(
                        *id_to_entity
                            .get(lift_id)
                            .ok_or(*lift_id)
                            .for_site(site_id)?,
                    )));
            }
        }
    }

    return Ok(site_id);
}

pub fn load_site(
    mut commands: Commands,
    mut load_sites: EventReader<LoadSite>,
    mut change_current_site: EventWriter<ChangeCurrentSite>,
) {
    for cmd in load_sites.read() {
        let site = match generate_site_entities(&mut commands, &cmd.site) {
            Ok(site) => site,
            Err(err) => {
                commands.entity(err.site).despawn_recursive();
                error!(
                    "Failed to load the site entities because the file had an \
                    internal inconsistency:\n{err:#?}\n---\nSite Data:\n{:#?}",
                    &cmd.site,
                );
                continue;
            }
        };
        if let Some(path) = &cmd.default_file {
            commands.entity(site).insert(DefaultFile(path.clone()));
        }

        if cmd.focus {
            change_current_site.send(ChangeCurrentSite { site, level: None });
        }
    }
}

#[derive(ThisError, Debug, Clone)]
pub enum ImportNavGraphError {
    #[error("The site we are importing into has a broken reference")]
    BrokenSiteReference,
    #[error("The nav graph that is being imported has a broken reference inside of it")]
    BrokenInternalReference(u32),
    #[error("The existing site is missing a level name required by the nav graphs: {0}")]
    MissingLevelName(String),
    #[error("The existing site is missing a lift name required by the nav graphs: {0}")]
    MissingLiftName(String),
    #[error("The existing site has a lift without a cabin anchor group: {0}")]
    MissingCabinAnchorGroup(String),
}

#[derive(Event)]
pub struct ImportNavGraphs {
    pub into_site: Entity,
    pub from_site: rmf_site_format::Site,
}

#[derive(SystemParam)]
pub struct ImportNavGraphParams<'w, 's> {
    commands: Commands<'w, 's>,
    sites: Query<'w, 's, &'static Children, With<NameOfSite>>,
    levels: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            &'static Parent,
            &'static Children,
        ),
        With<LevelElevation>,
    >,
    lifts: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            &'static Parent,
            &'static Children,
        ),
        With<LiftCabin<Entity>>,
    >,
    cabin_anchor_groups: Query<'w, 's, &'static Children, With<CabinAnchorGroup>>,
    anchors: Query<'w, 's, (Entity, &'static Anchor)>,
}

fn generate_imported_nav_graphs(
    params: &mut ImportNavGraphParams,
    into_site: Entity,
    from_site_data: &rmf_site_format::Site,
) -> Result<(), ImportNavGraphError> {
    let site_children = match params.sites.get(into_site) {
        Ok(c) => c,
        _ => return Err(ImportNavGraphError::BrokenSiteReference),
    };

    let mut level_name_to_entity = HashMap::new();
    for (e, name, parent, _) in &params.levels {
        if parent.get() != into_site {
            continue;
        }

        level_name_to_entity.insert(name.clone().0, e);
    }

    let mut lift_name_to_entity = HashMap::new();
    for (e, name, parent, _) in &params.lifts {
        if parent.get() != into_site {
            continue;
        }

        lift_name_to_entity.insert(name.clone().0, e);
    }

    let mut id_to_entity = HashMap::new();
    for (level_id, level_data) in &from_site_data.levels {
        if let Some(e) = level_name_to_entity.get(&level_data.properties.name.0) {
            id_to_entity.insert(*level_id, *e);
        } else {
            return Err(ImportNavGraphError::MissingLevelName(
                level_data.properties.name.0.clone(),
            ));
        }
    }

    let mut lift_to_anchor_group = HashMap::new();
    for (lift_id, lift_data) in &from_site_data.lifts {
        if let Some(e) = lift_name_to_entity.get(&lift_data.properties.name.0) {
            id_to_entity.insert(*lift_id, *e);
            if let Some(e_group) = params
                .lifts
                .get(*e)
                .unwrap()
                .3
                .iter()
                .find(|child| params.cabin_anchor_groups.contains(**child))
            {
                lift_to_anchor_group.insert(*e, *e_group);
            } else {
                return Err(ImportNavGraphError::MissingCabinAnchorGroup(
                    lift_data.properties.name.0.clone(),
                ));
            }
        } else {
            return Err(ImportNavGraphError::MissingLiftName(
                lift_data.properties.name.0.clone(),
            ));
        }
    }

    let anchor_close_enough = 0.05;
    for (lift_id, lift_data) in &from_site_data.lifts {
        let lift_e = *id_to_entity.get(lift_id).unwrap();
        let anchor_group = *lift_to_anchor_group.get(&lift_e).unwrap();
        let existing_lift_anchors: Vec<(Entity, &Anchor)> = params
            .cabin_anchor_groups
            .get(anchor_group)
            .unwrap()
            .iter()
            .filter_map(|child| params.anchors.get(*child).ok())
            .collect();

        for (anchor_id, anchor) in &lift_data.cabin_anchors {
            let mut already_existing = false;
            for (existing_id, existing_anchor) in &existing_lift_anchors {
                if anchor.is_close(*existing_anchor, anchor_close_enough) {
                    id_to_entity.insert(*anchor_id, *existing_id);
                    already_existing = true;
                    break;
                }
            }
            if !already_existing {
                params.commands.entity(anchor_group).with_children(|group| {
                    let e_anchor = group.spawn(AnchorBundle::new(anchor.clone())).id();
                    id_to_entity.insert(*anchor_id, e_anchor);
                });
            }
        }
    }

    for (level_id, level_data) in &from_site_data.levels {
        let level_e = *id_to_entity.get(level_id).unwrap();
        let existing_level_anchors: Vec<(Entity, &Anchor)> = params
            .levels
            .get(level_e)
            .unwrap()
            .3
            .iter()
            .filter_map(|child| params.anchors.get(*child).ok())
            .collect();
        for (anchor_id, anchor) in &level_data.anchors {
            let mut already_existing = false;
            for (existing_id, existing_anchor) in &existing_level_anchors {
                if anchor.is_close(*existing_anchor, anchor_close_enough) {
                    id_to_entity.insert(*anchor_id, *existing_id);
                    already_existing = true;
                    break;
                }
            }
            if !already_existing {
                params.commands.entity(level_e).with_children(|level| {
                    let e_anchor = level.spawn(AnchorBundle::new(anchor.clone())).id();
                    id_to_entity.insert(*anchor_id, e_anchor);
                });
            }
        }
    }

    {
        let existing_site_anchors: Vec<(Entity, &Anchor)> = site_children
            .iter()
            .filter_map(|child| params.anchors.get(*child).ok())
            .collect();
        for (anchor_id, anchor) in &from_site_data.anchors {
            let mut already_existing = false;
            for (existing_id, existing_anchor) in &existing_site_anchors {
                if anchor.is_close(*existing_anchor, anchor_close_enough) {
                    id_to_entity.insert(*anchor_id, *existing_id);
                    already_existing = true;
                    break;
                }
            }
            if !already_existing {
                params.commands.entity(into_site).with_children(|site| {
                    let e_anchor = site.spawn(AnchorBundle::new(anchor.clone())).id();
                    id_to_entity.insert(*anchor_id, e_anchor);
                });
            }
        }
    }

    for (nav_graph_id, nav_graph_data) in &from_site_data.navigation.guided.graphs {
        params.commands.entity(into_site).with_children(|site| {
            let e = site
                .spawn(SpatialBundle::default())
                .insert(nav_graph_data.clone())
                .id();
            id_to_entity.insert(*nav_graph_id, e);
        });
    }

    for (lane_id, lane_data) in &from_site_data.navigation.guided.lanes {
        let lane_data = lane_data
            .convert(&id_to_entity)
            .map_err(ImportNavGraphError::BrokenInternalReference)?;
        params.commands.entity(into_site).with_children(|site| {
            let e = site.spawn(lane_data).id();
            id_to_entity.insert(*lane_id, e);
        });
    }

    for (location_id, location_data) in &from_site_data.navigation.guided.locations {
        let location_data = location_data
            .convert(&id_to_entity)
            .map_err(ImportNavGraphError::BrokenInternalReference)?;
        params.commands.entity(into_site).with_children(|site| {
            let e = site.spawn(location_data).id();
            id_to_entity.insert(*location_id, e);
        });
    }

    Ok(())
}

pub fn import_nav_graph(
    mut params: ImportNavGraphParams,
    mut import_requests: EventReader<ImportNavGraphs>,
) {
    for r in import_requests.read() {
        if let Err(err) = generate_imported_nav_graphs(&mut params, r.into_site, &r.from_site) {
            error!("Failed to import nav graph: {err}");
        }
    }
}
