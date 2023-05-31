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

use bevy::prelude::*;

const DEFAULT_LAYER_SEMI_TRANSPARENCY: f32 = 0.2;

#[derive(Debug, Clone, Copy, Component)]
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
pub trait Cycle {
    type Value;
    fn next(&self) -> Self::Value;
    fn label(&self) -> &'static str;
}

impl LayerVisibility {
    pub fn new_semi_transparent() -> Self {
        LayerVisibility::Alpha(DEFAULT_LAYER_SEMI_TRANSPARENCY)
    }

    pub fn alpha(&self) -> f32 {
        match self {
            LayerVisibility::Opaque => 1.0,
            LayerVisibility::Alpha(a) => *a,
            LayerVisibility::Hidden => 0.0,
        }
    }
}

impl Cycle for LayerVisibility {
    type Value = Self;

    /// Cycle to the next visibility option
    fn next(&self) -> LayerVisibility {
        match self {
            LayerVisibility::Opaque => LayerVisibility::new_semi_transparent(),
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

impl Cycle for Option<LayerVisibility> {
    type Value = Self;
    fn next(&self) -> Self {
        match self {
            Some(v) => match v {
                LayerVisibility::Hidden => None,
                _ => Some(v.next()),
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

impl Default for LayerVisibility {
    fn default() -> Self {
        LayerVisibility::Opaque
    }
}
