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

use crate::{
    site::*,
};
use bevy::prelude::*;
use std::collections::HashMap;

/// This component is applied to each site element that gets loaded in order to
/// remember what its original ID within the Site file was.
#[derive(Component, Clone, Copy, Debug)]
pub struct SiteID(pub u32);

pub struct LoadSite(pub rmf_site_format::Site);

pub struct LoadedSites(pub Vec<Entity>);

fn generate_site_entities(
    commands: &mut Commands,
    site_data: &rmf_site_format::Site,
) -> Entity {
    let mut id_to_entity = HashMap::new();
    let site = commands
        .spawn_bundle(SpatialBundle{
            visibility: Visibility { is_visible: false },
            ..default()
        })
        .insert(site_data.properties)
        .with_children(|site| {
            for (level_id, level_data) in &site_data.levels {
                let level_entity = site.spawn_bundle(SpatialBundle{
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .insert(level_data.properties)
                .insert(SiteID(*level_id))
                .with_children(|level| {
                    for (anchor_id, anchor) in &level_data.anchors {
                        let anchor_entity = level
                            .spawn()
                            .insert(Anchor(anchor.0, anchor.1))
                            .insert(AnchorDependents::default())
                            .insert(SiteID(*anchor_id))
                            .id();
                        id_to_entity.insert(*anchor_id, anchor_entity);
                    }

                    for (door_id, door) in &level_data.doors {
                        let door_entity = level
                            .spawn()
                            .insert(door.to_ecs(&id_to_entity))
                            .insert(SiteID(*door_id))
                            .id();
                        id_to_entity.insert(*door_id, door_entity);
                    }

                    for (drawing_id, drawing) in &level_data.drawings {
                        level
                            .spawn()
                            .insert(drawing.clone())
                            .insert(SiteID(*drawing_id));
                    }

                    for (fiducial_id, fiducial) in &level_data.fiducials {
                        level
                            .spawn()
                            .insert(fiducial.to_ecs(&id_to_entity))
                            .insert(SiteID(*fiducial_id));
                    }

                    for (floor_id, floor) in &level_data.floors {
                        level
                            .spawn()
                            .insert(floor.to_ecs(&id_to_entity))
                            .insert(SiteID(*floor_id));
                    }

                    for (light_id, light) in &level_data.lights {
                        level
                            .spawn()
                            .insert(light.clone())
                            .insert(SiteID(*light_id));
                    }

                    for (measurement_id, measurement) in &level_data.measurements {
                        level
                            .spawn()
                            .insert(measurement.to_ecs(&id_to_entity))
                            .insert(SiteID(*measurement_id));
                    }

                    for (model_id, model) in &level_data.models {
                        level
                            .spawn()
                            .insert(model.clone())
                            .insert(SiteID(*model_id));
                    }

                    for (physical_camera_id, physical_camera) in &level_data.physical_cameras {
                        level
                            .spawn()
                            .insert(physical_camera.clone())
                            .insert(SiteID(*physical_camera_id));
                    }

                    for (wall_id, wall) in &level_data.walls {
                        level
                            .spawn()
                            .insert(wall.to_ecs(&id_to_entity))
                            .insert(SiteID(*wall_id));
                    }
                }).id();
                id_to_entity.insert(*level_id, level_entity);
            }

            for (lift_id, lift_data) in &site_data.lifts {
                site.spawn_bundle(SpatialBundle::default())
                    .insert(lift_data.to_ecs(&id_to_entity))
                    .insert(SiteID(lift_id))
                    .with_children(|lift| {
                        for (anchor_id, anchor) in &lift_data.cabin_anchors {
                            let anchor_entity = lift
                                .spawn()
                                .insert(Anchor(anchor.0, anchor.1))
                                .insert(AnchorDependents::default())
                                .insert(SiteID(*anchor_id))
                                .id();
                            id_to_entity.insert(*anchor_id, anchor_entity);
                        }
                    });
            }
        }).id();

    return site;
}

pub fn load_site(
    mut commands: Commands,
    mut loaded_sites: ResMut<LoadedSites>,
    mut loading_sites: EventReader<LoadSite>,
) {
    for site_to_load in loading_sites.iter() {
        loaded_sites.0.push(generate_site_entities(&mut commands, &site_to_load.0));
    }
}
