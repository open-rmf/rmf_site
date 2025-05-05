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

use crate::{interaction::main_view_render_layers, site::CurrentLevel};
use bevy::{
    pbr::CubemapVisibleEntities,
    prelude::{
        DirectionalLight as BevyDirectionalLight, PointLight as BevyPointLight,
        SpotLight as BevySpotLight, *,
    },
    render::{
        primitives::{CubemapFrusta, Frustum},
        view::VisibleEntities,
    },
};
use rmf_site_format::{Category, Light, LightKind, NameInSite, Pose};
use std::collections::{BTreeMap, HashMap};

/// True/false for whether the physical lights of an environment should be
/// rendered.
#[derive(Clone, Copy, Resource)]
pub struct PhysicalLightToggle(pub bool);

impl Default for PhysicalLightToggle {
    fn default() -> Self {
        Self(true)
    }
}

pub fn add_physical_lights(
    mut commands: Commands,
    added: Query<(Entity, Option<&Parent>), Added<LightKind>>,
    physical_light_toggle: Res<PhysicalLightToggle>,
    current_level: Res<CurrentLevel>,
) {
    for (e, parent) in &added {
        // This adds all the extra components provided by all three of the
        // possible light bundles so we can easily switch between the different
        // light types
        commands
            .entity(e)
            .insert(if physical_light_toggle.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            })
            .insert(ViewVisibility::default())
            .insert(InheritedVisibility::default())
            .insert(Frustum::default())
            .insert(VisibleEntities::default())
            .insert(CubemapFrusta::default())
            .insert(CubemapVisibleEntities::default())
            .insert(main_view_render_layers())
            .insert(Category::Light);

        if parent.is_none() {
            if let Some(current_level) = **current_level {
                commands.entity(current_level).add_child(e);
            } else {
                error!("No current level to assign light {e:?}");
            }
        }
    }
}

pub fn update_physical_lights(
    mut commands: Commands,
    mut changed: Query<
        (
            Entity,
            &LightKind,
            Option<&mut BevyPointLight>,
            Option<&mut BevySpotLight>,
            Option<&mut BevyDirectionalLight>,
        ),
        Changed<LightKind>,
    >,
) {
    for (e, kind, mut b_point, mut b_spot, mut b_dir) in &mut changed {
        match kind {
            LightKind::Point(point) => {
                if let Some(b_point) = &mut b_point {
                    **b_point = point.to_bevy();
                } else {
                    commands.entity(e).insert(point.to_bevy());
                }
            }
            LightKind::Spot(spot) => {
                if let Some(b_spot) = &mut b_spot {
                    **b_spot = spot.to_bevy();
                } else {
                    commands.entity(e).insert(spot.to_bevy());
                }
            }
            LightKind::Directional(dir) => {
                if let Some(b_dir) = &mut b_dir {
                    **b_dir = dir.to_bevy();
                } else {
                    commands.entity(e).insert(dir.to_bevy());
                }
            }
        }

        if !kind.is_point() && b_point.is_some() {
            commands.entity(e).remove::<BevyPointLight>();
        }

        if !kind.is_spot() && b_spot.is_some() {
            commands.entity(e).remove::<BevySpotLight>();
        }

        if !kind.is_directional() && b_dir.is_some() {
            commands.entity(e).remove::<BevyDirectionalLight>();
        }
    }
}

pub fn toggle_physical_lights(
    mut physical_lights: Query<&mut Visibility, With<LightKind>>,
    physical_light_toggle: Res<PhysicalLightToggle>,
) {
    if physical_light_toggle.is_changed() {
        for mut v in &mut physical_lights {
            *v = if physical_light_toggle.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Request the light layout to be exported to a file
#[derive(Clone, Event)]
pub struct ExportLights(pub std::path::PathBuf);

pub fn export_lights(
    mut exports: EventReader<ExportLights>,
    lights: Query<(&Pose, &LightKind, &Parent)>,
    levels: Query<&NameInSite>,
) {
    for export in exports.read() {
        let mut lights_per_level: BTreeMap<String, Vec<Light>> = BTreeMap::new();
        for (pose, kind, parent) in &lights {
            if let Ok(name) = levels.get(parent.get()) {
                lights_per_level
                    .entry(name.0.clone())
                    .or_default()
                    .push(Light {
                        pose: pose.clone(),
                        kind: kind.clone(),
                    })
            }
        }

        let out_file = match std::fs::File::create(export.0.clone()) {
            Ok(out_file) => out_file,
            Err(err) => {
                error!(
                    "Failed to create file {:?} for exporting lights: {}",
                    export.0, err,
                );
                continue;
            }
        };

        let mut root: BTreeMap<String, HashMap<String, Vec<Light>>> = BTreeMap::new();
        for (level, lights) in lights_per_level {
            let mut lights_map: HashMap<String, Vec<Light>> = HashMap::new();
            lights_map.insert("lights".to_owned(), lights);
            root.insert(level, lights_map);
        }

        if let Err(err) = serde_yaml::to_writer(out_file, &root) {
            error!("Error while exporting lights: {err}");
        }
    }
}
