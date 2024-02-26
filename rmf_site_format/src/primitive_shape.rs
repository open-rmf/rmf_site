/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use crate::Recall;
#[cfg(feature = "bevy")]
use bevy::prelude::{Component, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum PrimitiveShape {
    Box { size: [f32; 3] },
    Cylinder { radius: f32, length: f32 },
    Capsule { radius: f32, length: f32 },
    Sphere { radius: f32 },
}

impl Default for PrimitiveShape {
    fn default() -> Self {
        Self::Box {
            size: [1.0, 1.0, 1.0],
        }
    }
}

impl PrimitiveShape {
    pub fn label(&self) -> String {
        match &self {
            PrimitiveShape::Box { .. } => "Box",
            PrimitiveShape::Cylinder { .. } => "Cylinder",
            PrimitiveShape::Capsule { .. } => "Capsule",
            PrimitiveShape::Sphere { .. } => "Sphere",
        }
        .to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallPrimitiveShape {
    pub box_size: Option<[f32; 3]>,
    pub cylinder_radius: Option<f32>,
    pub cylinder_length: Option<f32>,
    pub capsule_radius: Option<f32>,
    pub capsule_length: Option<f32>,
    pub sphere_radius: Option<f32>,
}

impl Recall for RecallPrimitiveShape {
    type Source = PrimitiveShape;

    fn remember(&mut self, source: &PrimitiveShape) {
        match source {
            PrimitiveShape::Box { size } => {
                self.box_size = Some(*size);
            }
            PrimitiveShape::Cylinder { radius, length } => {
                self.cylinder_radius = Some(*radius);
                self.cylinder_length = Some(*length);
            }
            PrimitiveShape::Capsule { radius, length } => {
                self.capsule_radius = Some(*radius);
                self.capsule_length = Some(*length);
            }
            PrimitiveShape::Sphere { radius } => {
                self.sphere_radius = Some(*radius);
            }
        }
    }
}

impl RecallPrimitiveShape {
    pub fn assume_box(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Box { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Box {
                size: self.box_size.unwrap_or_default(),
            }
        }
    }

    pub fn assume_cylinder(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Cylinder { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Cylinder {
                radius: self.cylinder_radius.unwrap_or_default(),
                length: self.cylinder_length.unwrap_or_default(),
            }
        }
    }

    pub fn assume_capsule(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Capsule { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Capsule {
                radius: self.capsule_radius.unwrap_or_default(),
                length: self.capsule_length.unwrap_or_default(),
            }
        }
    }

    pub fn assume_sphere(&self, current: &PrimitiveShape) -> PrimitiveShape {
        if matches!(current, PrimitiveShape::Sphere { .. }) {
            current.clone()
        } else {
            PrimitiveShape::Sphere {
                radius: self.sphere_radius.unwrap_or_default(),
            }
        }
    }
}
