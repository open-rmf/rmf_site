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

#[cfg(feature = "bevy")]
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The Category component is added to site entities so they can easily express
/// what kind of thing they are, e.g. Anchor, Lane, Model, etc. This should be
/// set by the respective site system that decorates its entities with
/// components, e.g. add_door_visuals, add_lane_visuals, etc.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum Category {
    General,
    Site,
    Anchor,
    Door,
    Wall,
    Floor,
    Level,
    Lane,
    Lift,
    Light,
    Location,
    Measurement,
    Model,
    Camera,
    Drawing,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Site => "Site",
            Self::Anchor => "Anchor",
            Self::Door => "Door",
            Self::Wall => "Wall",
            Self::Floor => "Floor",
            Self::Level => "Level",
            Self::Lane => "Lane",
            Self::Lift => "Lift",
            Self::Light => "Light",
            Self::Location => "Location",
            Self::Measurement => "Measurement",
            Self::Model => "Model",
            Self::Camera => "Camera",
            Self::Drawing => "Drawing",
        }
    }

    /// Returns true if the category of this element has a physical presence.
    /// Returns false if the category of this element is conceptual.
    // TODO(MXG): Consider whether all non-physical entities should automatically
    // be assigned the VisualCue component.
    pub fn is_physical(&self) -> bool {
        match self {
            Self::Door | Self::Wall | Self::Floor | Self::Lift | Self::Model => true,
            // TODO(MXG): Consider whether Light and Camera should be considered physical
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Categorized<T>(pub(crate) BTreeMap<Category, T>);

impl<T> Categorized<T> {
    pub fn new(general: T) -> Self {
        Self(BTreeMap::from([(Category::General, general)]))
    }

    pub fn with_category(mut self, category: Category, value: T) -> Self {
        self.0.insert(category, value);
        self
    }

    pub fn for_general(&self) -> &T {
        self.0.get(&Category::General).unwrap()
    }

    pub fn for_category(&self, category: Category) -> &T {
        match category {
            Category::General => self.0.get(&Category::General).unwrap(),
            category => self.0.get(&category).unwrap_or_else(|| self.for_general()),
        }
    }
}
