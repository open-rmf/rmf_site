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
#[cfg(feature = "bevy")]
use bevy::prelude::{
    Bundle, Component, DirectionalLight as BevyDirectionalLight, PointLight as BevyPointLight,
    SpotLight as BevySpotLight,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Light {
    pub pose: Pose,
    pub kind: LightKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum LightKind {
    Point(PointLight),
    Spot(SpotLight),
    Directional(DirectionalLight),
}

impl Default for LightKind {
    fn default() -> Self {
        PointLight::default().into()
    }
}

#[cfg(feature = "bevy")]
impl LightKind {
    pub fn point(&self) -> Option<&PointLight> {
        match self {
            Self::Point(point) => Some(point),
            _ => None,
        }
    }

    pub fn spot(&self) -> Option<&SpotLight> {
        match self {
            Self::Spot(spot) => Some(spot),
            _ => None,
        }
    }

    pub fn directional(&self) -> Option<&DirectionalLight> {
        match self {
            Self::Directional(dir) => Some(dir),
            _ => None,
        }
    }

    pub fn color(&self) -> [f32; 4] {
        match self {
            Self::Point(l) => l.color,
            Self::Spot(l) => l.color,
            Self::Directional(l) => l.color,
        }
    }

    pub fn intensity(&self) -> Option<f32> {
        match self {
            Self::Point(l) => Some(l.intensity),
            Self::Spot(l) => Some(l.intensity),
            Self::Directional(_) => None,
        }
    }

    pub fn range(&self) -> Option<f32> {
        match self {
            Self::Point(l) => Some(l.range),
            Self::Spot(l) => Some(l.range),
            Self::Directional(_) => None,
        }
    }

    pub fn radius(&self) -> Option<f32> {
        match self {
            Self::Point(l) => Some(l.radius),
            Self::Spot(l) => Some(l.radius),
            Self::Directional(_) => None,
        }
    }

    pub fn enable_shadows(&self) -> bool {
        match self {
            Self::Point(l) => l.enable_shadows,
            Self::Spot(l) => l.enable_shadows,
            Self::Directional(l) => l.enable_shadows,
        }
    }

    pub fn illuminance(&self) -> Option<f32> {
        match self {
            Self::Point(_) => None,
            Self::Spot(_) => None,
            Self::Directional(l) => Some(l.illuminance),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Point(_) => "Point",
            Self::Spot(_) => "Spot",
            Self::Directional(_) => "Directional",
        }
    }

    pub fn is_point(&self) -> bool {
        matches!(self, Self::Point(_))
    }

    pub fn is_spot(&self) -> bool {
        matches!(self, Self::Spot(_))
    }

    pub fn is_directional(&self) -> bool {
        matches!(self, Self::Directional(_))
    }
}

impl From<PointLight> for LightKind {
    fn from(value: PointLight) -> Self {
        LightKind::Point(value)
    }
}

impl From<SpotLight> for LightKind {
    fn from(value: SpotLight) -> Self {
        LightKind::Spot(value)
    }
}

impl From<DirectionalLight> for LightKind {
    fn from(value: DirectionalLight) -> Self {
        LightKind::Directional(value)
    }
}

fn white() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}
fn default_range() -> f32 {
    20.0
}
fn default_radius() -> f32 {
    0.0
}
fn default_illuminance() -> f32 {
    100000.0
}
fn default_intensity() -> f32 {
    800.0
}
fn bool_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    #[serde(default = "white")]
    pub color: [f32; 4],
    #[serde(default = "default_intensity")]
    pub intensity: f32,
    #[serde(default = "default_range")]
    pub range: f32,
    #[serde(default = "default_radius")]
    pub radius: f32,
    #[serde(default = "bool_true")]
    pub enable_shadows: bool,
}

#[cfg(feature = "bevy")]
impl PointLight {
    pub fn to_bevy(&self) -> BevyPointLight {
        BevyPointLight {
            color: self.color.into(),
            intensity: self.intensity,
            range: self.range,
            radius: self.radius,
            shadows_enabled: self.enable_shadows,
            ..Default::default()
        }
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 800.0,
            range: 20.0,
            radius: 0.0,
            enable_shadows: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct SpotLight {
    #[serde(default = "white")]
    pub color: [f32; 4],
    #[serde(default = "default_intensity")]
    pub intensity: f32,
    #[serde(default = "default_range")]
    pub range: f32,
    #[serde(default = "default_radius")]
    pub radius: f32,
    #[serde(default = "bool_true")]
    pub enable_shadows: bool,
}

#[cfg(feature = "bevy")]
impl SpotLight {
    pub fn to_bevy(&self) -> BevySpotLight {
        BevySpotLight {
            color: self.color.into(),
            intensity: self.intensity,
            range: self.range,
            radius: self.radius,
            shadows_enabled: self.enable_shadows,
            ..Default::default()
        }
    }
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 800.0,
            range: 20.0,
            radius: 0.0,
            enable_shadows: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    #[serde(default = "white")]
    pub color: [f32; 4],
    #[serde(default = "default_illuminance")]
    pub illuminance: f32,
    #[serde(default = "bool_true")]
    pub enable_shadows: bool,
}

#[cfg(feature = "bevy")]
impl DirectionalLight {
    pub fn to_bevy(&self) -> BevyDirectionalLight {
        BevyDirectionalLight {
            color: self.color.into(),
            illuminance: self.illuminance,
            shadows_enabled: self.enable_shadows,
            ..Default::default()
        }
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            illuminance: 100000.0,
            enable_shadows: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallLightKind {
    pub intensity: Option<f32>,
    pub range: Option<f32>,
    pub radius: Option<f32>,
    pub illuminance: Option<f32>,
}

impl RecallLightKind {
    pub fn intensity(&self) -> f32 {
        self.intensity.unwrap_or(default_intensity())
    }
    pub fn range(&self) -> f32 {
        self.range.unwrap_or(default_range())
    }
    pub fn radius(&self) -> f32 {
        self.radius.unwrap_or(default_radius())
    }
    pub fn illuminance(&self) -> f32 {
        self.illuminance.unwrap_or(default_illuminance())
    }
}

#[cfg(feature = "bevy")]
impl RecallLightKind {
    pub fn assume_point(&self, current: &LightKind) -> LightKind {
        PointLight {
            color: current.color(),
            intensity: current.intensity().unwrap_or(self.intensity()),
            range: current.range().unwrap_or(self.range()),
            radius: current.radius().unwrap_or(self.radius()),
            enable_shadows: current.enable_shadows(),
        }
        .into()
    }

    pub fn assume_spot(&self, current: &LightKind) -> LightKind {
        SpotLight {
            color: current.color(),
            intensity: current.intensity().unwrap_or(self.intensity()),
            range: current.range().unwrap_or(self.range()),
            radius: current.radius().unwrap_or(self.radius()),
            enable_shadows: current.enable_shadows(),
        }
        .into()
    }

    pub fn assume_directional(&self, current: &LightKind) -> LightKind {
        DirectionalLight {
            color: current.color(),
            illuminance: current.illuminance().unwrap_or(self.illuminance()),
            enable_shadows: current.enable_shadows(),
        }
        .into()
    }
}

impl Recall for RecallLightKind {
    type Source = LightKind;

    fn remember(&mut self, source: &Self::Source) {
        match source {
            LightKind::Point(light) => {
                self.intensity = Some(light.intensity);
                self.range = Some(light.range);
                self.radius = Some(light.radius);
            }
            LightKind::Spot(light) => {
                self.intensity = Some(light.intensity);
                self.range = Some(light.range);
                self.radius = Some(light.radius);
            }
            LightKind::Directional(light) => {
                self.illuminance = Some(light.illuminance);
            }
        }
    }
}
