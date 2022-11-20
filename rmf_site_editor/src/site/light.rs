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
    pbr::CubemapVisibleEntities,
    render::{
        primitives::{CubemapFrusta, Frustum},
        view::VisibleEntities,
    },
    prelude::{
        *, SpotLight as BevySpotLight,
        PointLight as BevyPointLight,
        DirectionalLight as BevyDirectionalLight,
    }
};
use rmf_site_format::LightKind;
use crate::site::SiteAssets;

#[derive(Clone, Copy, Debug, Component)]
struct LightBodies {
    /// Visibility group for the point light
    point: Entity,
    /// Visibility group for the spot light
    spot: Entity,
    /// Mesh that visualizes the shining part of the spot light.
    /// Changes in color should be applied to this material.
    spot_shine: Entity,
    /// Visibility group for the directional light
    directional: Entity,
    /// Mesh that visualizes the shining part of the directional light.
    /// Changes in color should be applied to this material.
    directional_shine: Entity,
}

impl LightBodies {
    fn switch(&self, kind: &LightKind, )
}

pub fn add_physical_lights(
    mut commands: Commands,
    mut added: Query<Entity, Added<LightKind>>,
    assets: Res<SiteAssets>,
    mut materials: ResMut<Assets<StandardMaterials>>,
    mut visibilities: Query<&mut Visibility>,
) {
    for e in &added {
        // This adds all the extra components provided by all three of the
        // possible light bundles so we can easily switch between the different
        // light types
        let bodies = commands
            .entity(e)
            .insert(Visibility::visible())
            .insert(ComputedVisibility::default())
            .insert(Frustum::default())
            .insert(VisibleEntities::default())
            .insert(CubemapFrusta::default())
            .insert(CubemapVisibleEntities::default())
            .add_children(|parent| {
                let point = parent.spawn_bundle(PbrBundle{
                    mesh: assets.point_light_mesh.clone(),
                    visibility: Visibility { is_visible: false },
                    ..default()
                });

                let spot_cmd = parent.spawn_bundle(SpatialBundle {
                    visibility: Visibility { is_visible: false },
                    ..default()
                });
                let spot = spot_cmd.id();
                let spot_shine = spot_cmd.add_children(|spot| {
                    spot.spawn_bundle(PbrBundle {
                        mesh: assets.spot_light_cover_mesh.clone(),
                        material: assets.physical_camera_material.clone(),
                        ..default()
                    });

                    return spot.spawn_bundle(PbrBundle {
                        mesh: assets.spot_light_shine_mesh.clone(),
                        ..default()
                    }).id();
                });

                let dir_cmd = parent.spawn_bundle(SpatialBundle {
                    visibility: Visibility { is_visible: false },
                    ..default()
                });
                let directional = dir_cmd.id();
                let directional_shine = dir_cmd.add_children(|dir| {
                    dir.spawn_bundle(PbrBundle {
                        mesh: assets.directional_light_cover_mesh.clone(),
                        material: assets.physical_camera_material.clone(),
                        ..default()
                    });

                    return dir.spawn_bundle(PbrBundle {
                        mesh: assets.directional_light_shine_mesh.clone(),
                        ..default()
                    }).id();
                });

                return LightBodies {
                    point, spot, spot_shine, directional, directional_shine
                };
            });

        commands.entity(e).insert(bodies);
    }
}

pub fn update_physical_lights(
    mut commands: Commands,
    mut changed: Query<(
        Entity,
        &LightKind,
        &LightBodies,
        Option<&mut BevyPointLight>,
        Option<&mut BevySpotLight>,
        Option<&mut BevyDirectionalLight>,
    ), Changed<LightKind>>,
) {
    for (e, kind, bodies, mut b_point, mut b_spot, mut b_dir) in &mut changed {
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
