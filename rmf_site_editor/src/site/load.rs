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

use crate::site::*;
use bevy::prelude::*;
use std::{collections::HashMap, path::PathBuf};

/// This component is applied to each site element that gets loaded in order to
/// remember what its original ID within the Site file was.
#[derive(Component, Clone, Copy, Debug)]
pub struct SiteID(pub u32);

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

    let mut site = commands.spawn();
    site.insert_bundle(SpatialBundle {
        visibility: Visibility { is_visible: false },
        ..default()
    })
    .insert(site_data.properties.clone())
    .with_children(|site| {
        for (level_id, level_data) in &site_data.levels {
            let level_entity = site
                .spawn_bundle(SpatialBundle {
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .insert(level_data.properties.clone())
                .insert(SiteID(*level_id))
                .insert(Category("Level".to_string()))
                .with_children(|level| {
                    for (anchor_id, anchor) in &level_data.anchors {
                        let anchor_entity = level
                            .spawn()
                            .insert_bundle(AnchorBundle::new(*anchor))
                            .insert(SiteID(*anchor_id))
                            .id();
                        id_to_entity.insert(*anchor_id, anchor_entity);
                        consider_id(*anchor_id);
                    }

                    for (door_id, door) in &level_data.doors {
                        let door_entity = level
                            .spawn()
                            .insert_bundle(door.to_ecs(&id_to_entity))
                            .insert(SiteID(*door_id))
                            .id();
                        id_to_entity.insert(*door_id, door_entity);
                        consider_id(*door_id);
                    }

                    for (drawing_id, drawing) in &level_data.drawings {
                        level
                            .spawn()
                            .insert_bundle(drawing.clone())
                            .insert(SiteID(*drawing_id));
                        consider_id(*drawing_id);
                    }

                    for (fiducial_id, fiducial) in &level_data.fiducials {
                        level
                            .spawn()
                            .insert_bundle(fiducial.to_ecs(&id_to_entity))
                            .insert(SiteID(*fiducial_id));
                        consider_id(*fiducial_id);
                    }

                    for (floor_id, floor) in &level_data.floors {
                        level
                            .spawn()
                            .insert_bundle(floor.to_ecs(&id_to_entity))
                            .insert(SiteID(*floor_id));
                        consider_id(*floor_id);
                    }

                    for (light_id, light) in &level_data.lights {
                        level
                            .spawn()
                            .insert_bundle(light.clone())
                            .insert(SiteID(*light_id));
                        consider_id(*light_id);
                    }

                    for (measurement_id, measurement) in &level_data.measurements {
                        level
                            .spawn()
                            .insert_bundle(measurement.to_ecs(&id_to_entity))
                            .insert(SiteID(*measurement_id));
                        consider_id(*measurement_id);
                    }

                    for (model_id, model) in &level_data.models {
                        level
                            .spawn()
                            .insert_bundle(model.clone())
                            .insert(SiteID(*model_id));
                        consider_id(*model_id);
                    }

                    for (physical_camera_id, physical_camera) in &level_data.physical_cameras {
                        level
                            .spawn()
                            .insert_bundle(physical_camera.clone())
                            .insert(SiteID(*physical_camera_id));
                        consider_id(*physical_camera_id);
                    }

                    for (wall_id, wall) in &level_data.walls {
                        level
                            .spawn()
                            .insert_bundle(wall.to_ecs(&id_to_entity))
                            .insert(SiteID(*wall_id));
                        consider_id(*wall_id);
                    }
                })
                .id();
            id_to_entity.insert(*level_id, level_entity);
            consider_id(*level_id);
        }

        for (lift_id, lift_data) in &site_data.lifts {
            site.spawn_bundle(SpatialBundle::default())
                .insert_bundle(lift_data.properties.to_ecs(&id_to_entity))
                .insert(SiteID(*lift_id))
                .with_children(|lift| {
                    for (anchor_id, anchor) in &lift_data.cabin_anchors {
                        let anchor_entity = lift
                            .spawn()
                            .insert_bundle(AnchorBundle::new(*anchor))
                            .insert(SiteID(*anchor_id))
                            .id();
                        id_to_entity.insert(*anchor_id, anchor_entity);
                        consider_id(*anchor_id);
                    }
                });
            consider_id(*lift_id);
        }

        for (nav_graph_id, nav_graph_data) in &site_data.nav_graphs {
            site.spawn_bundle(SpatialBundle::default())
                .insert(nav_graph_data.properties.clone())
                .insert(SiteID(*nav_graph_id))
                .with_children(|nav_graph| {
                    for (lane_id, lane) in &nav_graph_data.lanes {
                        nav_graph
                            .spawn()
                            .insert_bundle(lane.to_ecs(&id_to_entity))
                            .insert(SiteID(*lane_id));
                        consider_id(*lane_id);
                    }

                    for (location_id, location) in &nav_graph_data.locations {
                        nav_graph
                            .spawn()
                            .insert_bundle(location.to_ecs(&id_to_entity))
                            .insert(SiteID(*location_id));
                        consider_id(*location_id);
                    }
                });
        }
    });

    site.insert(NextSiteID(highest_id + 1));
    return site.id();
}

pub fn load_site(
    mut commands: Commands,
    mut opened_sites: ResMut<OpenSites>,
    mut load_sites: EventReader<LoadSite>,
    mut change_current_site: EventWriter<ChangeCurrentSite>,
    mut site_display_state: ResMut<State<SiteState>>,
) {
    for cmd in load_sites.iter() {
        let site = generate_site_entities(&mut commands, &cmd.site);
        if let Some(path) = &cmd.default_file {
            commands.entity(site).insert(DefaultFile(path.clone()));
        }
        opened_sites.0.push(site);

        if cmd.focus {
            change_current_site.send(ChangeCurrentSite { site, level: None });

            if *site_display_state.current() == SiteState::Off {
                site_display_state.set(SiteState::Display).ok();
            }
        }
    }
}
