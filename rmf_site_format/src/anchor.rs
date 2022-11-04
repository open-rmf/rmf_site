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

use crate::{Category, Categorized};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(feature = "bevy")]
use bevy::{
    prelude::{Component, Transform, GlobalTransform},
    math::{Vec2, Vec3, Affine3A},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum Anchor {
    Translate2D([f32; 2]),
    CategorizedTranslate2D(Categorized<[f32; 2]>),
}

impl From<[f32; 2]> for Anchor {
    fn from(value: [f32; 2]) -> Self {
        Anchor::Translate2D(value)
    }
}

impl Anchor {
    pub fn translation_for_category(&self, category: Category) -> &[f32; 2] {
        match self {
            Self::Translate2D(v) => v,
            Self::CategorizedTranslate2D(v) => v.for_category(category),
        }
    }
}

#[cfg(feature = "bevy")]
impl Anchor {
    pub fn point(&self, category: Category, tf: &GlobalTransform) -> Vec3 {
        match category {
            Category::General => {
                tf.translation()
            }
            category => {
                let dp = Vec2::from(*self.translation_for_category(category)) - Vec2::from(*self.translation_for_category(Category::General));
                tf.affine().transform_point3([dp.x, dp.y, 0.0].into())
            }
        }
    }

    pub fn relative_transform(&self, category: Category) -> Transform {
        match self {
            Anchor::Translate2D(p) => Transform::from_translation([p[0], p[1], 0.0].into()),
            Anchor::CategorizedTranslate2D(categorized) => {
                let p = categorized.for_category(category);
                Transform::from_translation([p[0], p[1], 0.0].into())
            }
        }
    }
}

#[cfg(feature = "bevy")]
impl From<Anchor> for Transform {
    fn from(anchor: Anchor) -> Self {
        anchor.relative_transform(Category::General)
    }
}
