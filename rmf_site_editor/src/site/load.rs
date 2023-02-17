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

use crate::{site::*, Autoload};
use bevy::{ecs::system::SystemParam, prelude::*, tasks::AsyncComputeTaskPool};
use futures_lite::future;
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error as ThisError;

#[cfg(not(target_arch = "wasm32"))]
use {crate::main_menu::load_site_file, rfd::FileHandle};

/// This component is given to the site to kee ptrack of what file it should be
/// saved to by default.
#[derive(Component, Clone, Debug)]
pub struct DefaultFile(pub PathBuf);

pub struct LoadSite {
    /// The site data to load
    pub site: rmf_site_format::Site,
    /// Should the application switch focus to this new site
    pub focus: bool,
    /// The default file path that should be assigned to the site
    pub default_file: Option<PathBuf>,
}

fn generate_site_entities(commands: &mut Commands, site_data: &rmf_site_format::Site) -> Entity {
    let mut id_to_entity = HashMap::new();
    let mut highest_id = 0_u32;
    let mut consider_id = |consider| {
        if consider > highest_id {
            highest_id = consider;
        }
    };

    let mut binding = commands.spawn(SpatialBundle {
        visibility: Visibility { is_visible: false },
        ..default()
    });
    let mut site = binding
        .insert(Category::Site)
        .insert(site_data.properties.clone())
        .with_children(|site| {
            for (anchor_id, anchor) in &site_data.anchors {
                let anchor_entity = site
                    .spawn(AnchorBundle::new(anchor.clone()))
                    .insert(SiteID(*anchor_id))
                    .id();
                id_to_entity.insert(*anchor_id, anchor_entity);
                consider_id(*anchor_id);
            }

            for (level_id, level_data) in &site_data.levels {
                let level_entity = site
                    .spawn(SpatialBundle {
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .insert(level_data.properties.clone())
                    .insert(SiteID(*level_id))
                    .insert(Category::Level)
                    .with_children(|level| {
                        for (anchor_id, anchor) in &level_data.anchors {
                            let anchor_entity = level
                                .spawn(AnchorBundle::new(anchor.clone()))
                                .insert(SiteID(*anchor_id))
                                .id();
                            id_to_entity.insert(*anchor_id, anchor_entity);
                            consider_id(*anchor_id);
                        }

                        for (door_id, door) in &level_data.doors {
                            let door_entity = level
                                .spawn(door.to_ecs(&id_to_entity))
                                .insert(SiteID(*door_id))
                                .id();
                            id_to_entity.insert(*door_id, door_entity);
                            consider_id(*door_id);
                        }

                        for (drawing_id, drawing) in &level_data.drawings {
                            level.spawn(drawing.clone()).insert(SiteID(*drawing_id));
                            consider_id(*drawing_id);
                        }

                        for (fiducial_id, fiducial) in &level_data.fiducials {
                            level
                                .spawn(fiducial.to_ecs(&id_to_entity))
                                .insert(SiteID(*fiducial_id));
                            consider_id(*fiducial_id);
                        }

                        for (floor_id, floor) in &level_data.floors {
                            level
                                .spawn(floor.to_ecs(&id_to_entity))
                                .insert(SiteID(*floor_id));
                            consider_id(*floor_id);
                        }

                        for (light_id, light) in &level_data.lights {
                            level.spawn(light.clone()).insert(SiteID(*light_id));
                            consider_id(*light_id);
                        }

                        for (measurement_id, measurement) in &level_data.measurements {
                            level
                                .spawn(measurement.to_ecs(&id_to_entity))
                                .insert(SiteID(*measurement_id));
                            consider_id(*measurement_id);
                        }

                        for (model_id, model) in &level_data.models {
                            level.spawn(model.clone()).insert(SiteID(*model_id));
                            consider_id(*model_id);
                        }

                        for (physical_camera_id, physical_camera) in &level_data.physical_cameras {
                            level
                                .spawn(physical_camera.clone())
                                .insert(SiteID(*physical_camera_id));
                            consider_id(*physical_camera_id);
                        }

                        for (wall_id, wall) in &level_data.walls {
                            level
                                .spawn(wall.to_ecs(&id_to_entity))
                                .insert(SiteID(*wall_id));
                            consider_id(*wall_id);
                        }
                    })
                    .id();
                id_to_entity.insert(*level_id, level_entity);
                consider_id(*level_id);
            }

            for (lift_id, lift_data) in &site_data.lifts {
                let lift = site
                    .spawn(SiteID(*lift_id))
                    .insert(Category::Lift)
                    .with_children(|lift| {
                        let lift_entity = lift.parent_entity();
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

                        for (door_id, door) in &lift_data.cabin_doors {
                            let door_entity = lift
                                .spawn(door.to_ecs(&id_to_entity))
                                .insert(Dependents::single(lift_entity))
                                .id();
                            id_to_entity.insert(*door_id, door_entity);
                            consider_id(*door_id);
                        }
                    })
                    .insert(lift_data.properties.to_ecs(&id_to_entity))
                    .id();
                id_to_entity.insert(*lift_id, lift);
                consider_id(*lift_id);
            }

            for (nav_graph_id, nav_graph_data) in &site_data.navigation.guided.graphs {
                let nav_graph = site
                    .spawn(SpatialBundle::default())
                    .insert(nav_graph_data.clone())
                    .insert(SiteID(*nav_graph_id))
                    .id();
                id_to_entity.insert(*nav_graph_id, nav_graph);
                consider_id(*nav_graph_id);
            }

            for (lane_id, lane_data) in &site_data.navigation.guided.lanes {
                let lane = site
                    .spawn(lane_data.to_ecs(&id_to_entity))
                    .insert(SiteID(*lane_id))
                    .id();
                id_to_entity.insert(*lane_id, lane);
                consider_id(*lane_id);
            }

            for (location_id, location_data) in &site_data.navigation.guided.locations {
                let location = site
                    .spawn(location_data.to_ecs(&id_to_entity))
                    .insert(SiteID(*location_id))
                    .id();
                id_to_entity.insert(*location_id, location);
                consider_id(*location_id);
            }
        });

    site.insert(NextSiteID(highest_id + 1));
    let site_id = site.id();

    // Make the lift cabin anchors that are used by doors subordinate
    for (lift_id, lift_data) in &site_data.lifts {
        for (_, door) in &lift_data.cabin_doors {
            for anchor in door.reference_anchors.array() {
                commands
                    .entity(*id_to_entity.get(&anchor).unwrap())
                    .insert(Subordinate(Some(*id_to_entity.get(lift_id).unwrap())));
            }
        }
    }

    return site_id;
}

pub fn load_site(
    mut commands: Commands,
    mut load_sites: EventReader<LoadSite>,
    mut change_current_workspace: EventWriter<ChangeCurrentWorkspace>,
    mut site_display_state: ResMut<State<SiteState>>,
) {
    for cmd in load_sites.iter() {
        let root = generate_site_entities(&mut commands, &cmd.site);
        if let Some(path) = &cmd.default_file {
            commands.entity(root).insert(DefaultFile(path.clone()));
        }

        if cmd.focus {
            change_current_workspace.send(ChangeCurrentWorkspace { root });

            if *site_display_state.current() == SiteState::Off {
                site_display_state.set(SiteState::Display).ok();
            }
        }
    }
}

#[derive(ThisError, Debug, Clone)]
pub enum ImportNavGraphError {
    #[error("The site we are importing into has a broken reference")]
    BrokenSiteReference,
    #[error("The existing site is missing a level name required by the nav graphs: {0}")]
    MissingLevelName(String),
    #[error("The existing site is missing a lift name required by the nav graphs: {0}")]
    MissingLiftName(String),
    #[error("The existing site has a lift without a cabin anchor group: {0}")]
    MissingCabinAnchorGroup(String),
}

pub struct ImportNavGraphs {
    pub into_site: Entity,
    pub from_site: rmf_site_format::Site,
}

#[derive(SystemParam)]
pub struct ImportNavGraphParams<'w, 's> {
    commands: Commands<'w, 's>,
    sites: Query<'w, 's, &'static Children, With<SiteProperties>>,
    levels: Query<
        'w,
        's,
        (
            Entity,
            &'static LevelProperties,
            &'static Parent,
            &'static Children,
        ),
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
    for (e, level, parent, _) in &params.levels {
        if parent.get() != into_site {
            continue;
        }

        level_name_to_entity.insert(level.name.clone(), e);
    }

    let mut lift_name_to_entity = HashMap::new();
    for (e, name, parent, _) in &params.lifts {
        if parent.get() != into_site {
            continue;
        }

        lift_name_to_entity.insert(name.0.clone(), e);
    }

    let mut id_to_entity = HashMap::new();
    for (level_id, level_data) in &from_site_data.levels {
        if let Some(e) = level_name_to_entity.get(&level_data.properties.name) {
            id_to_entity.insert(*level_id, *e);
        } else {
            return Err(ImportNavGraphError::MissingLevelName(
                level_data.properties.name.clone(),
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
                params.commands.entity(anchor_group).add_children(|group| {
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
                params.commands.entity(level_e).add_children(|level| {
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
                params.commands.entity(into_site).add_children(|site| {
                    let e_anchor = site.spawn(AnchorBundle::new(anchor.clone())).id();
                    id_to_entity.insert(*anchor_id, e_anchor);
                });
            }
        }
    }

    for (nav_graph_id, nav_graph_data) in &from_site_data.navigation.guided.graphs {
        params.commands.entity(into_site).add_children(|site| {
            let e = site
                .spawn(SpatialBundle::default())
                .insert(nav_graph_data.clone())
                .id();
            id_to_entity.insert(*nav_graph_id, e);
        });
    }

    for (lane_id, lane_data) in &from_site_data.navigation.guided.lanes {
        params.commands.entity(into_site).add_children(|site| {
            let e = site.spawn(lane_data.to_ecs(&id_to_entity)).id();
            id_to_entity.insert(*lane_id, e);
        });
    }

    for (location_id, location_data) in &from_site_data.navigation.guided.locations {
        params.commands.entity(into_site).add_children(|site| {
            let e = site.spawn(location_data.to_ecs(&id_to_entity)).id();
            id_to_entity.insert(*location_id, e);
        });
    }

    Ok(())
}

pub fn import_nav_graph(
    mut params: ImportNavGraphParams,
    mut import_requests: EventReader<ImportNavGraphs>,
    mut autoload: Option<ResMut<Autoload>>,
    current_workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<SiteProperties>>,
) {
    for r in import_requests.iter() {
        if let Err(err) = generate_imported_nav_graphs(&mut params, r.into_site, &r.from_site) {
            println!("Failed to import nav graph: {err}");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        'import: {
            let autoload = match autoload.as_mut() {
                Some(a) => a,
                None => break 'import,
            };

            if autoload.importing.is_some() {
                break 'import;
            }

            let import = match &autoload.import {
                Some(p) => p,
                None => break 'import,
            };

            let current_site = match current_workspace.to_site(&open_sites) {
                Some(s) => s,
                None => break 'import,
            };

            let file = FileHandle::wrap(import.clone());
            autoload.importing = Some(
                AsyncComputeTaskPool::get()
                    .spawn(async move { load_site_file(&file).await.map(|s| (current_site, s)) }),
            );

            autoload.import = None;
        }

        'importing: {
            let autoload = match autoload.as_mut() {
                Some(a) => a,
                None => break 'importing,
            };

            let task = match &mut autoload.importing {
                Some(t) => t,
                None => break 'importing,
            };

            let result = match future::block_on(future::poll_once(task)) {
                Some(r) => r,
                None => break 'importing,
            };

            autoload.importing = None;

            let (into_site, from_site_data) = match result {
                Some(r) => r,
                None => break 'importing,
            };

            if let Err(err) = generate_imported_nav_graphs(&mut params, into_site, &from_site_data)
            {
                println!("Failed to auto-import nav graph: {err}");
            }
        }
    }
}
