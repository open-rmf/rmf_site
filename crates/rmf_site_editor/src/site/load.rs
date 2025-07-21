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
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemParam},
    prelude::*,
};
use rmf_site_format::legacy::{building_map::BuildingMap, PortingError};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
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

    /// Create a `LoadSite` instance from raw data and optionally a file name.
    ///
    /// Note that this function can take some time to run if the file data is
    /// large, so it's best to use this in an async context.
    pub fn from_data(data: &Vec<u8>, default_file: Option<PathBuf>) -> Result<Self, LoadSiteError> {
        if let Some(path) = &default_file {
            let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
                return Err(LoadSiteError::IncompatibleFilename(path.clone()));
            };

            // If the default file is specified, we should only try to parse
            // based on formats that match the name, if it's possible to identify
            // one.
            let site = if filename.ends_with(".building.yaml") {
                match BuildingMap::from_bytes(data) {
                    Ok(building) => building
                        .to_site()
                        .map_err(LoadSiteError::LegacyConversion)?,
                    Err(err) => {
                        return Err(LoadSiteError::CorruptedBuildingFile {
                            path: path.clone(),
                            err,
                        });
                    }
                }
            } else if filename.ends_with(".json") {
                Site::from_bytes_json(data)?
            } else if filename.ends_with(".ron") {
                Site::from_bytes_ron(data)
                    .map_err(|err| LoadSiteError::RonParsingError(Box::new(err)))?
            } else {
                return Err(LoadSiteError::UnrecognizedFileType(path.clone()));
            };

            return Ok(Self {
                site,
                focus: false,
                default_file,
            });
        }

        // No file type was indicated, so try parsing the data with each option
        // in order of how likely it will be used
        let site = Site::from_bytes_json(data)
            .map_err(|_| LoadSiteError::UnknownDataFormat)
            .or_else(|_| {
                BuildingMap::from_bytes(data)
                    .map_err(|_| LoadSiteError::UnknownDataFormat)
                    .and_then(|building| {
                        building
                            .to_site()
                            .map_err(|_| LoadSiteError::UnknownDataFormat)
                    })
            })
            .or_else(|_| {
                Site::from_bytes_ron(data).map_err(|_| LoadSiteError::UnknownDataFormat)
            })?;

        Ok(Self {
            site,
            focus: false,
            default_file,
        })
    }
}

#[derive(ThisError, Debug)]
pub enum LoadSiteError {
    #[error("Trying to load a site with an incompatible filename: {0}")]
    IncompatibleFilename(PathBuf),
    #[error("Failed to parse legacy building file named [{path}]: {err}")]
    CorruptedBuildingFile {
        path: PathBuf,
        err: serde_yaml::Error,
    },
    #[error("Failed to convert a legacy building into a site: {0}")]
    LegacyConversion(#[from] PortingError),
    #[error("Failed parsing ron site file: {0}")]
    RonParsingError(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Failed parsing json site file: {0}")]
    JsonParsingError(#[from] serde_json::Error),
    #[error("Unrecognized file type: {0}")]
    UnrecognizedFileType(PathBuf),
    #[error("Cannot determine data format for raw data. It could not be parsed as .building.yaml, .site.json, or .site.ron")]
    UnknownDataFormat,
}

trait LoadResult<T> {
    fn for_site(self, site: Entity) -> Result<T, SiteLoadingError>;
}

impl<T> LoadResult<T> for Result<T, SiteID> {
    fn for_site(self, site: Entity) -> Result<T, SiteLoadingError> {
        self.map_err(|broken| SiteLoadingError::new(site, broken))
    }
}

pub type LoadSiteResult = Result<LoadSite, LoadSiteError>;

#[derive(ThisError, Debug)]
#[error("The site has a broken internal reference: {broken}")]
struct SiteLoadingError {
    site: Entity,
    broken: SiteID,
    // TODO(@mxgrey): reintroduce Backtrack when it's supported on stable
    // backtrace: Backtrace,
}

impl SiteLoadingError {
    fn new(site: Entity, broken: SiteID) -> Self {
        Self { site, broken }
    }
}

fn generate_site_entities(
    commands: &mut Commands,
    model_loader: &mut ModelLoader,
    site_data: &rmf_site_format::Site,
) -> Result<Entity, SiteLoadingError> {
    let mut id_to_entity = HashMap::new();

    let site_id = commands
        .spawn((Transform::IDENTITY, Visibility::Hidden))
        .insert(Category::Site)
        .insert(WorkspaceMarker)
        .id();

    for (anchor_id, anchor) in &site_data.anchors {
        let anchor_entity = commands
            .spawn(AnchorBundle::new(anchor.clone()))
            .insert(ChildOf(site_id))
            .id();
        id_to_entity.insert(*anchor_id, anchor_entity);
    }

    for (group_id, group) in &site_data.fiducial_groups {
        let group_entity = commands.spawn(group.clone()).insert(ChildOf(site_id)).id();
        id_to_entity.insert(*group_id, group_entity);
    }

    for (group_id, group) in &site_data.textures {
        let group_entity = commands.spawn(group.clone()).insert(ChildOf(site_id)).id();
        id_to_entity.insert(*group_id, group_entity);
    }

    for (level_id, level_data) in &site_data.levels {
        let level_entity = commands.spawn(ChildOf(site_id)).id();

        for (anchor_id, anchor) in &level_data.anchors {
            let anchor_entity = commands
                .spawn(AnchorBundle::new(anchor.clone()))
                .insert(ChildOf(level_entity))
                .id();
            id_to_entity.insert(*anchor_id, anchor_entity);
        }

        for (door_id, door) in &level_data.doors {
            let door_entity = commands
                .spawn(door.convert(&id_to_entity).for_site(site_id)?)
                .insert(ChildOf(level_entity))
                .id();
            id_to_entity.insert(*door_id, door_entity);
        }

        for (drawing_id, drawing) in &level_data.drawings {
            let drawing_entity = commands
                .spawn(DrawingBundle::new(drawing.properties.clone()))
                .insert(ChildOf(level_entity))
                .id();

            for (anchor_id, anchor) in &drawing.anchors {
                let anchor_entity = commands
                    .spawn(AnchorBundle::new(anchor.clone()))
                    .insert(ChildOf(drawing_entity))
                    .id();
                id_to_entity.insert(*anchor_id, anchor_entity);
            }

            for (fiducial_id, fiducial) in &drawing.fiducials {
                let fiducial_entity = commands
                    .spawn(fiducial.convert(&id_to_entity).for_site(site_id)?)
                    .insert(ChildOf(drawing_entity))
                    .id();
                id_to_entity.insert(*fiducial_id, fiducial_entity);
            }

            for (measurement_id, measurement) in &drawing.measurements {
                let measurement_entity = commands
                    .spawn(measurement.convert(&id_to_entity).for_site(site_id)?)
                    .insert(ChildOf(drawing_entity))
                    .id();
                id_to_entity.insert(*measurement_id, measurement_entity);
            }
        }

        for (floor_id, floor) in &level_data.floors {
            commands
                .spawn(floor.convert(&id_to_entity).for_site(site_id)?)
                .insert(ChildOf(level_entity));
        }

        for (wall_id, wall) in &level_data.walls {
            commands
                .spawn(wall.convert(&id_to_entity).for_site(site_id)?)
                .insert(ChildOf(level_entity));
        }

        commands
            .entity(level_entity)
            .insert((Transform::IDENTITY, Visibility::Hidden))
            .insert(level_data.properties.clone())
            .insert(Category::Level)
            .with_children(|level| {
                // These don't need a return value so can be wrapped in a with_children
                for (light_id, light) in &level_data.lights {
                    level.spawn(light.clone());
                }

                for (physical_camera_id, physical_camera) in &level_data.physical_cameras {
                    level.spawn(physical_camera.clone());
                }

                for (camera_pose_id, camera_pose) in &level_data.user_camera_poses {
                    level.spawn(camera_pose.clone());
                }
            });

        // TODO(MXG): Log when a RecencyRanking fails to load correctly.
        commands
            .entity(level_entity)
            .insert(
                RecencyRanking::<FloorMarker>::from_site_ids(
                    &level_data.rankings.floors,
                    &id_to_entity,
                )
                .unwrap_or(RecencyRanking::new()),
            )
            .insert(
                RecencyRanking::<DrawingMarker>::from_site_ids(
                    &level_data.rankings.drawings,
                    &id_to_entity,
                )
                .unwrap_or(RecencyRanking::new()),
            );
        id_to_entity.insert(*level_id, level_entity);
    }

    for (lift_id, lift_data) in &site_data.lifts {
        let lift_entity = commands.spawn(ChildOf(site_id)).id();

        commands.entity(lift_entity).with_children(|lift| {
            lift.spawn((Transform::default(), Visibility::default()))
                .insert(CabinAnchorGroupBundle::default())
                .with_children(|anchor_group| {
                    for (anchor_id, anchor) in &lift_data.cabin_anchors {
                        let anchor_entity =
                            anchor_group.spawn(AnchorBundle::new(anchor.clone())).id();
                        id_to_entity.insert(*anchor_id, anchor_entity);
                    }
                });
        });

        for (door_id, door) in &lift_data.cabin_doors {
            let door_entity = commands
                .spawn(door.convert(&id_to_entity).for_site(site_id)?)
                .insert(Dependents::single(lift_entity))
                .insert(ChildOf(lift_entity))
                .id();
            id_to_entity.insert(*door_id, door_entity);
        }

        commands.entity(lift_entity).insert(Category::Lift).insert(
            lift_data
                .properties
                .convert(&id_to_entity)
                .for_site(site_id)?,
        );

        id_to_entity.insert(*lift_id, lift_entity);
    }

    for (fiducial_id, fiducial) in &site_data.fiducials {
        let fiducial_entity = commands
            .spawn(fiducial.convert(&id_to_entity).for_site(site_id)?)
            .insert(ChildOf(site_id))
            .id();
        id_to_entity.insert(*fiducial_id, fiducial_entity);
    }

    for (nav_graph_id, nav_graph_data) in &site_data.navigation.guided.graphs {
        let nav_graph = commands
            .spawn((Transform::default(), Visibility::default()))
            .insert(nav_graph_data.clone())
            .insert(ChildOf(site_id))
            .id();
        id_to_entity.insert(*nav_graph_id, nav_graph);
    }

    for (lane_id, lane_data) in &site_data.navigation.guided.lanes {
        let lane = commands
            .spawn(lane_data.convert(&id_to_entity).for_site(site_id)?)
            .insert(ChildOf(site_id))
            .id();
        id_to_entity.insert(*lane_id, lane);
    }

    for (location_id, location_data) in &site_data.navigation.guided.locations {
        let location = commands
            .spawn(location_data.convert(&id_to_entity).for_site(site_id)?)
            .insert(ChildOf(site_id))
            .id();
        id_to_entity.insert(*location_id, location);
    }
    // Properties require the id_to_entity map to be fully populated to load suppressed issues
    commands.entity(site_id).insert(
        site_data
            .properties
            .convert(&id_to_entity)
            .for_site(site_id)?,
    );

    let mut model_description_dependents = HashMap::<Entity, HashSet<Entity>>::new();
    let mut model_description_to_source = HashMap::<Entity, AssetSource>::new();
    for (model_description_id, model_description) in &site_data.model_descriptions {
        let model_description_entity = commands
            .spawn(model_description.clone())
            .insert(Category::ModelDescription)
            .insert(ChildOf(site_id))
            .id();
        id_to_entity.insert(*model_description_id, model_description_entity);
        model_description_dependents.insert(model_description_entity, HashSet::new());
        model_description_to_source
            .insert(model_description_entity, model_description.source.0.clone());
    }

    for (robot_id, robot_data) in &site_data.robots {
        // Robot IDs are pointing to model description entities
        if let Some(model_description_entity) = id_to_entity
            .get(robot_id)
            .filter(|e| model_description_to_source.contains_key(*e))
        {
            commands
                .entity(*model_description_entity)
                .insert(ModelProperty(robot_data.clone()));
        } else {
            // Robot is affiliated to a non-existent model description,
            // create a description entity for users to modify after loading
            commands
                .spawn(ModelDescriptionBundle::default())
                .insert(Category::ModelDescription)
                .insert(ModelProperty(robot_data.clone()))
                .insert(ChildOf(site_id));
            error!(
                "Robot {} with properties {:?} is pointing to a non-existent \
                model description! Assigning robot to the default model description \
                with an empty asset source.",
                robot_id, robot_data
            );
        };
    }

    for (model_instance_id, parented_model_instance) in &site_data.model_instances {
        let model_instance = parented_model_instance
            .bundle
            .convert(&id_to_entity)
            .for_site(site_id)?;

        // The parent id is invalid, we do not spawn this model instance and generate
        // an error instead
        let parent = id_to_entity
            .get(&parented_model_instance.parent)
            .ok_or_else(|| SiteLoadingError::new(site_id, parented_model_instance.parent))?;

        let model_instance_entity = model_loader
            .spawn_model_instance(*parent, model_instance.clone())
            .insert(Category::Model)
            .id();
        id_to_entity.insert(*model_instance_id, model_instance_entity);

        if let Some(instances) = model_instance
            .description
            .0
            .map(|e| model_description_dependents.get_mut(&e))
            .flatten()
        {
            instances.insert(model_instance_entity);
        } else {
            error!(
                "Model description missing for instance {}. This should \
                not happen, please report this bug to the maintainers of \
                rmf_site_editor.",
                model_instance.name.0,
            );
        }
    }

    for (model_description_entity, dependents) in model_description_dependents {
        commands
            .entity(model_description_entity)
            .insert(Dependents(dependents));
    }

    for (task_id, task_data) in &site_data.tasks {
        let task_entity = commands
            .spawn(task_data.clone())
            .insert(Category::Task)
            .insert(ChildOf(site_id))
            .id();
        id_to_entity.insert(*task_id, task_entity);
    }

    for (scenario_id, scenario_data) in &site_data.scenarios {
        let parent = match scenario_data.properties.parent_scenario.0 {
            Some(parent_id) => *id_to_entity.get(&parent_id).unwrap_or(&site_id),
            None => site_id,
        };
        let scenario = scenario_data.convert(&id_to_entity).for_site(site_id)?;
        let scenario_entity = commands
            .spawn(scenario.properties.clone())
            .insert(ChildOf(parent))
            .id();
        id_to_entity.insert(*scenario_id, scenario_entity);

        // Spawn instance modifier entities
        let mut scenario_modifiers: ScenarioModifiers = ScenarioModifiers::default();
        for (instance_id, instance) in scenario_data.instances.iter() {
            if let Some(instance_entity) = id_to_entity.get(instance_id) {
                if instance.pose.is_some() || instance.visibility.is_some() {
                    let modifier_entity = commands
                        .spawn(Affiliation::affiliated(*instance_entity))
                        .insert(ChildOf(scenario_entity))
                        .id();
                    if let Some(pose) = instance.pose {
                        commands
                            .entity(modifier_entity)
                            .insert(Modifier::<Pose>::new(pose));
                    }
                    if let Some(vis) = instance.visibility {
                        let visibility = if vis {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        };
                        commands
                            .entity(modifier_entity)
                            .insert(Modifier::<Visibility>::new(visibility));
                    }
                    scenario_modifiers.insert(*instance_entity, modifier_entity);
                } else {
                    error!(
                        "Model instance {} does not have all required modifiers in scenario {}!",
                        instance_id, scenario.properties.name.0
                    );
                }
            } else {
                error!(
                    "Model instance {} referenced by scenario {} is missing! This should \
                    not happen, please report this bug to the maintainers of rmf_site_editor.",
                    instance_id, scenario.properties.name.0
                );
            }
        }
        for (task_id, task_data) in scenario_data.tasks.iter() {
            if let Some(task_entity) = id_to_entity.get(task_id) {
                if task_data.inclusion.is_some() || task_data.params.is_some() {
                    let modifier_entity = commands
                        .spawn(Affiliation::affiliated(*task_entity))
                        .insert(ChildOf(scenario_entity))
                        .id();
                    if let Some(inclusion) = task_data.inclusion {
                        commands
                            .entity(modifier_entity)
                            .insert(Modifier::<Inclusion>::new(inclusion));
                    }
                    if let Some(params) = &task_data.params {
                        commands
                            .entity(modifier_entity)
                            .insert(Modifier::<TaskParams>::new(params.clone()));
                    }
                    scenario_modifiers.insert(*task_entity, modifier_entity);
                } else {
                    error!(
                        "Task {} does not have all required modifiers in scenario {}!",
                        task_id, scenario.properties.name.0
                    );
                }
            } else {
                error!(
                    "Task {} referenced by scenario {} is missing! This should \
                    not happen, please report this bug to the maintainers of rmf_site_editor.",
                    task_id, scenario.properties.name.0
                );
            }
        }
        commands.entity(scenario_entity).insert(scenario_modifiers);
    }

    let nav_graph_rankings = match RecencyRanking::<NavGraphMarker>::from_site_ids(
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

    commands.entity(site_id).insert(nav_graph_rankings);

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
    mut model_loader: ModelLoader,
    mut load_sites: EventReader<LoadSite>,
    mut change_current_site: EventWriter<ChangeCurrentSite>,
) {
    for cmd in load_sites.read() {
        let site = match generate_site_entities(&mut commands, &mut model_loader, &cmd.site) {
            Ok(site) => site,
            Err(err) => {
                commands.entity(err.site).despawn();
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
            change_current_site.write(ChangeCurrentSite {
                site,
                level: None,
                scenario: None,
            });
        }
    }
}

#[derive(ThisError, Debug, Clone)]
pub enum ImportNavGraphError {
    #[error("The site we are importing into has a broken reference")]
    BrokenSiteReference,
    #[error("The nav graph that is being imported has a broken reference inside of it")]
    BrokenInternalReference(SiteID),
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
            &'static ChildOf,
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
            &'static ChildOf,
            &'static Children,
        ),
        With<LiftCabin>,
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
    for (e, name, child_of, _) in &params.levels {
        if child_of.parent() != into_site {
            continue;
        }

        level_name_to_entity.insert(name.clone().0, e);
    }

    let mut lift_name_to_entity = HashMap::new();
    for (e, name, child_of, _) in &params.lifts {
        if child_of.parent() != into_site {
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
                .find(|child| params.cabin_anchor_groups.contains(*child))
            {
                lift_to_anchor_group.insert(*e, e_group);
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
            .filter_map(|child| params.anchors.get(child).ok())
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
            .filter_map(|child| params.anchors.get(child).ok())
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
            .filter_map(|child| params.anchors.get(child).ok())
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
                .spawn((Transform::default(), Visibility::default()))
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
