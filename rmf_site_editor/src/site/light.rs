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
    pbr::{
        CubemapVisibleEntities,
    },
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

pub fn add_physical_lights(
    mut commands: Commands,
    mut added: Query<Entity, Added<LightKind>>,
) {
    for e in &added {
        // This adds all the extra components provided by all three of the
        // possible light bundles so we can easily switch between the different
        // light types
        commands
            .entity(e)
            .insert(Visibility::visible())
            .insert(ComputedVisibility::default())
            .insert(Frustum::default())
            .insert(VisibleEntities::default())
            .insert(CubemapFrusta::default())
            .insert(CubemapVisibleEntities::default());
    }
}

pub fn update_physical_lights(
    mut commands: Commands,
    mut changed: Query<(
        Entity,
        &LightKind,
        Option<&mut BevyPointLight>,
        Option<&mut BevySpotLight>,
        Option<&mut BevyDirectionalLight>,
    ), Changed<LightKind>>,
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
