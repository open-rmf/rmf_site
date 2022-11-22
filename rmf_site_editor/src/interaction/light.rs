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
    interaction::{DragPlaneBundle, InteractionAssets, Selectable},
    site::LightKind,
};
use bevy::prelude::*;

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
    fn switch(&self, kind: &LightKind, visibilities: &mut Query<&mut Visibility>) {
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

pub fn add_physical_light_visual_cues(
    mut commands: Commands,
    new_lights: Query<Entity, Added<LightKind>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<InteractionAssets>,
) {
    for e in &new_lights {
        let light_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            ..default()
        });

        let bodies = commands
            .entity(e)
            .insert(light_material.clone())
            .add_children(|parent| {
                let point = parent
                    .spawn_bundle(SpatialBundle {
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .with_children(|point| {
                        point
                            .spawn_bundle(PbrBundle {
                                mesh: assets.point_light_socket_mesh.clone(),
                                material: assets.physical_light_cover_material.clone(),
                                ..default()
                            })
                            .insert(Selectable::new(e))
                            .insert_bundle(DragPlaneBundle::new(e, Vec3::Z));

                        point
                            .spawn_bundle(PbrBundle {
                                mesh: assets.point_light_shine_mesh.clone(),
                                material: light_material.clone(),
                                ..default()
                            })
                            .insert(Selectable::new(e))
                            .insert_bundle(DragPlaneBundle::new(e, Vec3::Z));
                    })
                    .id();

                let spot = parent
                    .spawn_bundle(SpatialBundle {
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .with_children(|spot| {
                        spot.spawn_bundle(PbrBundle {
                            mesh: assets.spot_light_cover_mesh.clone(),
                            material: assets.physical_light_cover_material.clone(),
                            ..default()
                        })
                        .insert(Selectable::new(e))
                        .insert_bundle(DragPlaneBundle::new(e, Vec3::Z));

                        spot.spawn_bundle(PbrBundle {
                            mesh: assets.spot_light_shine_mesh.clone(),
                            material: light_material.clone(),
                            ..default()
                        })
                        .insert(Selectable::new(e))
                        .insert_bundle(DragPlaneBundle::new(e, Vec3::Z));
                    })
                    .id();

                let directional = parent
                    .spawn_bundle(SpatialBundle {
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .with_children(|dir| {
                        dir.spawn_bundle(PbrBundle {
                            mesh: assets.directional_light_cover_mesh.clone(),
                            material: assets.physical_light_cover_material.clone(),
                            ..default()
                        })
                        .insert(Selectable::new(e))
                        .insert_bundle(DragPlaneBundle::new(e, Vec3::Z));

                        dir.spawn_bundle(PbrBundle {
                            mesh: assets.directional_light_shine_mesh.clone(),
                            material: light_material.clone(),
                            ..default()
                        })
                        .insert(Selectable::new(e))
                        .insert_bundle(DragPlaneBundle::new(e, Vec3::Z));
                    })
                    .id();

                return LightBodies {
                    point,
                    spot,
                    directional,
                };
            });

        commands.entity(e).insert(bodies);
    }
}

pub fn update_physical_light_visual_cues(
    changed: Query<(&LightKind, &LightBodies, &Handle<StandardMaterial>)>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut visibilities: Query<&mut Visibility>,
) {
    for (kind, bodies, material) in &changed {
        bodies.switch(kind, &mut visibilities);
        if let Some(m) = material_assets.get_mut(material) {
            m.base_color = kind.color().into();
        } else {
            println!("DEV ERROR: Unable to get material asset for light");
        }
    }
}
