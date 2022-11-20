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
pub struct LightBodies {
    /// Visibility group for the point light
    point: Entity,
    /// Visibility group for the spot light
    spot: Entity,
    /// Visibility group for the directional light
    directional: Entity,
}

impl LightBodies {
    fn switch(
        &self,
        kind: &LightKind,
        visibilities: &mut Query<&mut Visibility>,
    ) {
        if let Ok(mut v) = visibilities.get_mut(self.point) {
            v.is_visible = kind.is_point();
        }

        if let Ok(mut v) = visibilities.get_mut(self.spot) {
            v.is_visible = kind.is_spot();
        }

        if let Ok(mut v) = visibilities.get_mut(self.directional) {
            v.is_visible = kind.is_directional();
        }
    }
}

pub fn add_physical_lights(
    mut commands: Commands,
    added: Query<Entity, Added<LightKind>>,
    assets: Res<SiteAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for e in &added {
        // This adds all the extra components provided by all three of the
        // possible light bundles so we can easily switch between the different
        // light types
        let light_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            ..default()
        });
        let bodies = commands
            .entity(e)
            .insert(light_material.clone())
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
                    material: light_material.clone(),
                    ..default()
                }).id();

                let spot = parent
                    .spawn_bundle(SpatialBundle {
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .with_children(|spot| {
                        spot.spawn_bundle(PbrBundle {
                            mesh: assets.spot_light_cover_mesh.clone(),
                            material: assets.physical_camera_material.clone(),
                            ..default()
                        });

                        spot.spawn_bundle(PbrBundle {
                            mesh: assets.spot_light_shine_mesh.clone(),
                            material: light_material.clone(),
                            ..default()
                        });
                    }).id();

                let directional = parent
                        .spawn_bundle(SpatialBundle {
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .with_children(|dir| {
                        dir.spawn_bundle(PbrBundle {
                            mesh: assets.directional_light_cover_mesh.clone(),
                            material: assets.physical_camera_material.clone(),
                            ..default()
                        });

                        dir.spawn_bundle(PbrBundle {
                            mesh: assets.directional_light_shine_mesh.clone(),
                            material: light_material.clone(),
                            ..default()
                        });
                    }).id();

                return LightBodies { point, spot, directional };
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
        &Handle<StandardMaterial>,
        Option<&mut BevyPointLight>,
        Option<&mut BevySpotLight>,
        Option<&mut BevyDirectionalLight>,
    ), Changed<LightKind>>,
    mut visibilities: Query<&mut Visibility>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    for (e, kind, bodies, material, mut b_point, mut b_spot, mut b_dir) in &mut changed {
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

        bodies.switch(kind, &mut visibilities);
        if let Some(m) = material_assets.get_mut(material) {
            m.base_color = kind.color().into();
        } else {
            println!("DEV ERROR: Unable to get material asset for light {e:?}");
        }
    }
}
