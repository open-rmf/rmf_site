/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

#[cfg(feature = "bevy")]
use bevy::prelude::{Component, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum LayerVisibility {
    /// The layer is fully opaque. This is the default for floors when no drawing is
    /// present.
    Opaque,
    /// Make the layer semi-transparent. This is useful for allowing layers
    /// to be visible undearneath them. When a drawing is added to the scene,
    /// the floors will automatically change to Alpha(0.1).
    Alpha(f32),
    /// The layer is fully hidden.
    Hidden,
}

// TODO(MXG): Should this trait be more general?
pub trait VisibilityCycle {
    type Value;
    fn next(&self, transparency: f32) -> Self::Value;
    fn label(&self) -> &'static str;
}

impl LayerVisibility {
    pub fn alpha(&self) -> f32 {
        match self {
            LayerVisibility::Opaque => 1.0,
            LayerVisibility::Alpha(a) => *a,
            LayerVisibility::Hidden => 0.0,
        }
    }

    pub fn is_opaque(&self) -> bool {
        matches!(self, LayerVisibility::Opaque)
    }

    pub fn is_semi_transparent(&self, with_value: f32) -> bool {
        if let LayerVisibility::Alpha(value) = self {
            *value == with_value
        } else {
            false
        }
    }

    pub fn is_floor_general_default(&self) -> bool {
        self.is_semi_transparent(DEFAULT_FLOOR_SEMI_TRANSPARENCY)
    }

    pub fn is_hidden(&self) -> bool {
        matches!(self, LayerVisibility::Hidden)
    }
}

impl VisibilityCycle for LayerVisibility {
    type Value = Self;

    /// Cycle to the next visibility option
    fn next(&self, transparency: f32) -> LayerVisibility {
        match self {
            LayerVisibility::Opaque => LayerVisibility::Alpha(transparency),
            LayerVisibility::Alpha(_) => LayerVisibility::Hidden,
            LayerVisibility::Hidden => LayerVisibility::Opaque,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            LayerVisibility::Opaque => "opaque",
            LayerVisibility::Alpha(_) => "semi-transparent",
            LayerVisibility::Hidden => "hidden",
        }
    }
}

impl VisibilityCycle for Option<LayerVisibility> {
    type Value = Self;
    fn next(&self, transparency: f32) -> Self {
        match self {
            Some(v) => match v {
                LayerVisibility::Hidden => None,
                _ => Some(v.next(transparency)),
            },
            None => Some(LayerVisibility::Opaque),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Some(v) => v.label(),
            None => "global default",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct GlobalFloorVisibility {
    /// The global visibility that will be applied to all floors when a drawing
    /// is not present on the level
    #[serde(
        default = "default_floor_without_drawings_visibility",
        skip_serializing_if = "LayerVisibility::is_opaque"
    )]
    pub without_drawings: LayerVisibility,
    /// The global visibility that will be applied to all floors when at least
    /// one drawing is present on the same level.
    #[serde(
        default = "default_floor_general_visibility",
        skip_serializing_if = "LayerVisibility::is_floor_general_default"
    )]
    pub general: LayerVisibility,
    /// The user's preference for semi-transparency values for floors
    #[serde(
        default = "default_floor_semi_transparency",
        skip_serializing_if = "is_default_floor_semi_transparency"
    )]
    pub preferred_semi_transparency: f32,
}

impl Default for GlobalFloorVisibility {
    fn default() -> Self {
        Self {
            without_drawings: LayerVisibility::Opaque,
            general: LayerVisibility::Alpha(DEFAULT_FLOOR_SEMI_TRANSPARENCY),
            preferred_semi_transparency: DEFAULT_FLOOR_SEMI_TRANSPARENCY,
        }
    }
}

// Semi transparency for floors, less transparent than drawings to make it clear
// where floors have been drawn.
pub const DEFAULT_FLOOR_SEMI_TRANSPARENCY: f32 = 0.7;
fn is_default_floor_semi_transparency(value: &f32) -> bool {
    *value == DEFAULT_FLOOR_SEMI_TRANSPARENCY
}

fn default_floor_semi_transparency() -> f32 {
    DEFAULT_FLOOR_SEMI_TRANSPARENCY
}

fn default_floor_without_drawings_visibility() -> LayerVisibility {
    LayerVisibility::Opaque
}

fn default_floor_general_visibility() -> LayerVisibility {
    LayerVisibility::Alpha(DEFAULT_FLOOR_SEMI_TRANSPARENCY)
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct GlobalDrawingVisibility {
    /// The global visibility that will be applied to the bottom-most drawings
    /// in the ranking.
    #[serde(
        default = "default_bottom_drawing_visibility",
        skip_serializing_if = "LayerVisibility::is_opaque"
    )]
    pub bottom: LayerVisibility,
    /// The user's preference for semi-transparency values for bottom drawings
    #[serde(
        default = "default_drawing_semi_transparency",
        skip_serializing_if = "is_default_drawing_semi_transparency"
    )]
    pub preferred_bottom_semi_transparency: f32,
    /// How many of the bottom-most drawings count as being on the bottom
    #[serde(
        default = "default_bottom_drawing_count",
        skip_serializing_if = "is_default_bottom_drawing_count"
    )]
    pub bottom_count: usize,
    /// The global visibility that will be applied to all drawings which are not
    /// on the bottom.
    #[serde(
        default = "default_general_drawing_visibility",
        skip_serializing_if = "is_default_drawing_visibility"
    )]
    pub general: LayerVisibility,
    /// The user's preference for semi-transparency values for general drawings
    #[serde(
        default = "default_drawing_semi_transparency",
        skip_serializing_if = "is_default_drawing_semi_transparency"
    )]
    pub preferred_general_semi_transparency: f32,
}

impl Default for GlobalDrawingVisibility {
    fn default() -> Self {
        Self {
            bottom: LayerVisibility::Opaque,
            preferred_bottom_semi_transparency: DEFAULT_DRAWING_SEMI_TRANSPARENCY,
            bottom_count: 1,
            general: LayerVisibility::Alpha(DEFAULT_DRAWING_SEMI_TRANSPARENCY),
            preferred_general_semi_transparency: DEFAULT_DRAWING_SEMI_TRANSPARENCY,
        }
    }
}

// Semi transparency for drawings, more opaque than floors to make them visible
pub const DEFAULT_DRAWING_SEMI_TRANSPARENCY: f32 = 0.5;
fn is_default_drawing_semi_transparency(value: &f32) -> bool {
    *value == DEFAULT_DRAWING_SEMI_TRANSPARENCY
}

fn default_drawing_semi_transparency() -> f32 {
    DEFAULT_DRAWING_SEMI_TRANSPARENCY
}

fn default_bottom_drawing_visibility() -> LayerVisibility {
    LayerVisibility::Opaque
}

fn default_general_drawing_visibility() -> LayerVisibility {
    LayerVisibility::Alpha(DEFAULT_DRAWING_SEMI_TRANSPARENCY)
}

fn is_default_drawing_visibility(value: &LayerVisibility) -> bool {
    value.is_semi_transparent(DEFAULT_DRAWING_SEMI_TRANSPARENCY)
}

fn default_bottom_drawing_count() -> usize {
    1
}

fn is_default_bottom_drawing_count(value: &usize) -> bool {
    *value == 1
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct PreferredSemiTransparency(pub f32);

impl PreferredSemiTransparency {
    pub fn for_drawing() -> Self {
        Self(DEFAULT_DRAWING_SEMI_TRANSPARENCY)
    }

    pub fn is_default_for_drawing(&self) -> bool {
        self.0 == DEFAULT_DRAWING_SEMI_TRANSPARENCY
    }

    pub fn for_floor() -> Self {
        Self(DEFAULT_FLOOR_SEMI_TRANSPARENCY)
    }

    pub fn is_default_for_floor(&self) -> bool {
        self.0 == DEFAULT_FLOOR_SEMI_TRANSPARENCY
    }
}
