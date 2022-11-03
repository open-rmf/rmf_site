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

/// The Category component is added to site entities so they can easily express
/// what kind of thing they are, e.g. Anchor, Lane, Model, etc. This should be
/// set by the respective site system that decorates its entities with
/// components, e.g. add_door_visuals, add_lane_visuals, etc.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum Category {
    General,
    Anchor,
    Door,
    Wall,
    Floor,
    Level,
    Lane,
    Lift,
    Measurement,
    Model,
    Camera,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Anchor => "Anchor",
            Self::Door => "Door",
            Self::Wall => "Wall",
            Self::Floor => "Floor",
            Self::Level => "Level",
            Self::Lane => "Lane",
            Self::Lift => "Lift",
            Self::Measurement => "Measurement",
            Self::Model => "Model",
            Self::Camera => "Camera",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct Categorized<T>(BTreeMap<Category, T>);

impl<T> Categorized<T> {
    pub fn new(general: T) -> Self {
        Self(BTreeMap::from([(Category::General, general)]))
    }

    pub fn with_category(mut self, category: Category, value: T) -> Self {
        self.0.insert(category, value);
        self
    }

    pub fn for_category(&self, category: Category) -> &T {
        match category {
            Category::General => self.0.get(&Category::General).unwrap(),
            category => self.0.get(&category).unwrap_or_else(
                || self.0.get(&Category::General).unwrap()
            ),
        }
    }
}
