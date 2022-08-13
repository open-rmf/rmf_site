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

use crate::*;
use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::{
    prelude::{
        Component, PointLight, SpotLight, DirectionalLight, AmbientLight,
        PointLightBundle, SpotLightBundle, DirectionalLightBundle,
    },
    ecs::system::EntityCommands,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Light {
    pub pose: Pose,
    pub kind: LightType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum LightType {
    PointLight{
        pub color: [f32; 4],
        pub intensity: f32,
        pub range: f32,
        pub radius: f32,
    },
    SpotLight{
        pub color: [f32; 4],
        pub intensity: f32,
        pub range: f32,
        pub radius: f32,
    },
    DirectionalLight{
        pub color: [f32; 4],
        pub illuminance: f32,
    },
    AmbientLight{
        pub color: [f32; 4],
        pub brightness: f32,
    }
}

#[cfg(feature="bevy")]
impl Light {
    pub fn insert_into(&self, commands: &mut EntityCommands) {
        match self.kind {
            LightType::PointLight{color, intensity, range, radius} => {
                commands.insert_bundle(PointLightBundle{
                    transform: self.pose.transform(),
                    point_light: PointLight{
                        color: color.into(),
                        intensity,
                        range,
                        radius,
                        ..default()
                    },
                    ..default()
                });
            },
            LightType::SpotLight{color, intensity, range, radius} => {
                commands.insert_bundle(SpotLightBundle{
                    transform: self.pose.transform(),
                    spot_light: SpotLight{
                        color: color.into(),
                        intensity,
                        range,
                        radius,
                        ..default()
                    },
                    ..default()
                });
            },
            LightType::DirectionalLight{color, illuminance} => {
                commands.insert_bundle(DirectionalLightBundle{
                    transform: self.pose.transform(),
                    directional_light: DirectionalLight{
                        color: color.into(),
                        illuminance,
                        ..default()
                    },
                    ..default()
                });
            },
            LightType::AmbientLight{color, brightness} => {
                commands.insert(AmbientLight{
                    color: color.into(),
                    brightness,
                });
            }
        }
    }
}
