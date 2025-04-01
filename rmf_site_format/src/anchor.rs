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

use crate::{Categorized, Category, Pose};
#[cfg(feature = "bevy")]
use bevy::{
    ecs::{query::QueryEntityError, system::SystemParam},
    prelude::{Component, Entity, GlobalTransform, Parent, Query, Transform},
};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use bevy::utils::tracing::error;

#[derive(Serialize, Deserialize, Clone, Debug)]
// TODO(MXG): Change this to untagged for a cleaner looking format once this
// issue is resolved: https://github.com/ron-rs/ron/issues/217
// #[serde(untagged)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum Anchor {
    Translate2D([f32; 2]),
    CategorizedTranslate2D(Categorized<[f32; 2]>),
    Pose3D(Pose),
}

impl From<[f32; 2]> for Anchor {
    fn from(value: [f32; 2]) -> Self {
        Anchor::Translate2D(value)
    }
}

fn to_slice(p: &[f32; 3]) -> [f32; 2] { [p[0], p[1]] }

impl Anchor {
    pub fn translation_for_category(&self, category: Category) -> [f32; 2] {
        match self {
            Self::Translate2D(v) => *v,
            Self::CategorizedTranslate2D(v) => *v.for_category(category),
            Self::Pose3D(p) => to_slice(&p.trans),
        }
    }

    pub fn is_close(&self, other: &Anchor, dist: f32) -> bool {
        match self {
            Self::Translate2D(p) => {
                let p_left = Vec2::from_array(*p);
                match other {
                    Self::Translate2D(p) => {
                        let p_right = Vec2::from_array(*p);
                        return (p_left - p_right).length() <= dist;
                    }
                    Self::CategorizedTranslate2D(categories) => {
                        for (_, p) in &categories.0 {
                            let p_right = Vec2::from_array(*p);
                            if (p_left - p_right).length() > dist {
                                return false;
                            }
                        }
                        return true;
                    }
                    Self::Pose3D(p) => {
                        let p_right = Vec2::from_array(to_slice(&p.trans));
                        return (p_left - p_right).length() <= dist;
                    }
                }
            }
            Self::CategorizedTranslate2D(left_categories) => match other {
                Self::Translate2D(p) => {
                    let p_left = Vec2::from_array(*left_categories.for_general());
                    let p_right = Vec2::from_array(*p);
                    return (p_left - p_right).length() <= dist;
                }
                Self::CategorizedTranslate2D(right_categories) => {
                    for (category, p) in &left_categories.0 {
                        let p_left = Vec2::from_array(*p);
                        let p_right = Vec2::from_array(*right_categories.for_category(*category));
                        if (p_left - p_right).length() > dist {
                            return false;
                        }
                    }
                    return true;
                }
                Self::Pose3D(p) => {
                    let p_left = Vec2::from_array(*left_categories.for_general());
                    let p_right = Vec2::from_array(to_slice(&p.trans));
                    return (p_left - p_right).length() <= dist;
                }
            },
            Self::Pose3D(p_left) => match other {
                Self::Translate2D(p_right) => {
                    let p_left = Vec3::from_array(p_left.trans);
                    let p_right = Vec3::from_array([p_right[0], p_right[1], 0.0]);
                    return (p_left - p_right).length() <= dist;
                }
                Self::CategorizedTranslate2D(p_right) => {
                    let p_right = p_right.for_general();
                    let p_left = Vec3::from_array(p_left.trans);
                    let p_right = Vec3::from_array([p_right[0], p_right[1], 0.0]);
                    return (p_left - p_right).length() <= dist;
                }
                Self::Pose3D(p_right) => {
                    let p_left = Vec3::from_array(p_left.trans);
                    let p_right = Vec3::from_array(p_right.trans);
                    return (p_left - p_right).length() <= dist;
                }
            },
        }
    }

    #[allow(non_snake_case)]
    pub fn is_3D(&self) -> bool {
        matches!(self, Anchor::Pose3D { .. })
    }
}

#[cfg(feature = "bevy")]
impl Anchor {
    pub fn point(&self, category: Category, tf: &GlobalTransform) -> Vec3 {
        match category {
            Category::General => tf.translation(),
            category => {
                let dp = Vec2::from(self.translation_for_category(category))
                    - Vec2::from(self.translation_for_category(Category::General));
                tf.affine().transform_point3([dp.x, dp.y, 0.0].into())
            }
        }
    }

    pub fn local_transform(&self, category: Category) -> Transform {
        match self {
            Anchor::Translate2D(p) => Transform::from_translation([p[0], p[1], 0.0].into()),
            Anchor::CategorizedTranslate2D(categorized) => {
                let p = categorized.for_category(category);
                Transform::from_translation([p[0], p[1], 0.0].into())
            }
            Anchor::Pose3D(p) => p.transform(),
        }
    }

    pub fn move_to(&mut self, tf: &Transform) {
        match self {
            Anchor::Translate2D(p) => {
                p[0] = tf.translation.x;
                p[1] = tf.translation.y;
            }
            Anchor::CategorizedTranslate2D(categorized) => {
                let delta = tf.translation.truncate()
                    - Vec2::from(*categorized.for_category(Category::General));
                for (_, v) in &mut categorized.0 {
                    *v = (Vec2::from(*v) + delta).into();
                }
            }
            Anchor::Pose3D(p) => {
                p.trans[0] = tf.translation.x;
                p.trans[1] = tf.translation.y;
                p.trans[2] = tf.translation.z;
                p.align_with(tf);
            }
        }
    }
}

#[cfg(feature = "bevy")]
#[derive(SystemParam)]
pub struct AnchorParams<'w, 's> {
    anchors: Query<'w, 's, (&'static Anchor, &'static GlobalTransform)>,
    parents: Query<'w, 's, &'static Parent>,
    global_tfs: Query<'w, 's, &'static GlobalTransform>,
}

#[cfg(feature = "bevy")]
impl<'w, 's> AnchorParams<'w, 's> {
    pub fn point(&self, anchor: Entity, category: Category) -> Result<Vec3, QueryEntityError> {
        let (anchor, tf) = self.anchors.get(anchor)?;
        Ok(anchor.point(category, tf))
    }

    pub fn relative_point(
        &self,
        anchor: Entity,
        category: Category,
        relative_to: Entity,
    ) -> Result<Vec3, QueryEntityError> {
        let (anchor, tf) = self.anchors.get(anchor)?;
        let relative_to_tf = self.global_tfs.get(relative_to)?;
        let global_p = anchor.point(category, tf);
        Ok(relative_to_tf.affine().inverse().transform_point3(global_p))
    }

    pub fn point_in_parent_frame_of(
        &self,
        anchor: Entity,
        category: Category,
        in_parent_frame_of: Entity,
    ) -> Result<Vec3, QueryEntityError> {
        match self.parents.get(in_parent_frame_of) {
            Ok(parent) => self.relative_point(anchor, category, parent.get()),
            Err(_) => self.point(anchor, category),
        }
    }

    pub fn local_transform(
        &self,
        entity: Entity,
        category: Category,
    ) -> Result<Transform, QueryEntityError> {
        let (anchor, _) = self.anchors.get(entity)?;
        Ok(anchor.local_transform(category))
    }
}

#[cfg(feature = "bevy")]
impl From<Anchor> for Transform {
    fn from(anchor: Anchor) -> Self {
        anchor.local_transform(Category::General)
    }
}
