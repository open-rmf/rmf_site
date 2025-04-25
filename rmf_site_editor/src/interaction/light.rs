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
    interaction::{DragPlaneBundle, HeadlightToggle, InteractionAssets},
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
            *v = if kind.is_point() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }

        if let Ok(mut v) = visibilities.get_mut(self.spot) {
            *v = if kind.is_spot() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }

        if let Ok(mut v) = visibilities.get_mut(self.directional) {
            *v = if kind.is_directional() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

pub fn add_physical_light_visual_cues(
    mut commands: Commands,
    new_lights: Query<(Entity, &LightKind), Added<LightKind>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<InteractionAssets>,
    mut headlight_toggle: ResMut<HeadlightToggle>,
) {
    for (e, kind) in &new_lights {
        let light_material = materials.add(StandardMaterial {
            base_color: Color::srgb_from_array(kind.color()),
            unlit: true,
            perceptual_roughness: 0.089,
            ..default()
        });

        if kind.is_directional() {
            headlight_toggle.0 = false;
        }

        let point_visibility = if kind.is_point() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let point = commands
            .spawn((Transform::default(), point_visibility))
            .with_children(|point| {
                point
                    .spawn((
                        Mesh3d(assets.point_light_socket_mesh.clone()),
                        MeshMaterial3d(assets.physical_light_cover_material.clone()),
                        Transform::default(),
                        Visibility::default(),
                    ))
                    .insert(DragPlaneBundle::new(e, Vec3::Z).globally());

                point
                    .spawn((
                        Mesh3d(assets.point_light_shine_mesh.clone()),
                        MeshMaterial3d(light_material.clone()),
                        Transform::default(),
                        Visibility::default(),
                    ))
                    .insert(DragPlaneBundle::new(e, Vec3::Z).globally());
            })
            .id();

        let spot_visibility = if kind.is_point() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let spot = commands
            .spawn((Transform::default(), spot_visibility))
            .with_children(|spot| {
                spot.spawn((
                    Mesh3d(assets.spot_light_cover_mesh.clone()),
                    MeshMaterial3d(assets.physical_light_cover_material.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .insert(DragPlaneBundle::new(e, Vec3::Z).globally());

                spot.spawn((
                    Mesh3d(assets.spot_light_shine_mesh.clone()),
                    MeshMaterial3d(light_material.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .insert(DragPlaneBundle::new(e, Vec3::Z).globally());
            })
            .id();

        let directional_visibility = if kind.is_point() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let directional = commands
            .spawn((Transform::default(), directional_visibility))
            .with_children(|dir| {
                dir.spawn((
                    Mesh3d(assets.directional_light_cover_mesh.clone()),
                    MeshMaterial3d(assets.direction_light_cover_material.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .insert(DragPlaneBundle::new(e, Vec3::Z).globally());

                dir.spawn((
                    Mesh3d(assets.directional_light_shine_mesh.clone()),
                    MeshMaterial3d(light_material.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .insert(DragPlaneBundle::new(e, Vec3::Z).globally());
            })
            .id();

        commands
            .entity(e)
            .insert(MeshMaterial3d(light_material.clone()))
            .insert(LightBodies {
                point,
                spot,
                directional,
            })
            .push_children(&[point, spot, directional]);
    }
}

pub fn update_physical_light_visual_cues(
    changed: Query<
        (&LightKind, &LightBodies, &MeshMaterial3d<StandardMaterial>),
        Changed<LightKind>,
    >,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut visibilities: Query<&mut Visibility>,
    mut headlight_toggle: ResMut<HeadlightToggle>,
) {
    for (kind, bodies, material) in &changed {
        bodies.switch(kind, &mut visibilities);
        if let Some(m) = material_assets.get_mut(material) {
            m.base_color = Color::srgb_from_array(kind.color());
        } else {
            error!("Unable to get material asset for light");
        }

        if kind.is_directional() {
            headlight_toggle.0 = false;
        }
    }
}
