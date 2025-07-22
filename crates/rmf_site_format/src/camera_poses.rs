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

use crate::{Anchor, Category, NameInSite, Pose, Rotation};
use bevy_ecs::prelude::{Bundle, Component};
use glam::{Affine3A, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Bundle, Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct UserCameraPose {
    pub pose: Pose,
    pub name: NameInSite,
    pub marker: UserCameraPoseMarker,
}

#[derive(Component, Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct UserCameraPoseMarker;

impl UserCameraPose {
    pub fn from_anchors<'a, I>(name: &str, anchors: I) -> Self
    where
        I: Iterator<Item = &'a Anchor>,
    {
        // Center the camera in the centroid of the anchors
        let mut count = 0;
        let mut trans = Vec3::default();
        for anchor in anchors {
            let anchor_trans = anchor.translation_for_category(Category::Level);
            trans[0] = trans[0] + anchor_trans[0];
            trans[1] = trans[1] + anchor_trans[1];
            count += 1;
        }
        if count > 0 {
            trans = trans / count as f32;
        }
        let offset = Vec3::new(-10.0, -10.0, 10.0);
        let affine = Affine3A::look_at_rh(trans + offset, trans, Vec3::Z);
        let rotation = Quat::from_mat3a(&affine.matrix3);
        // TODO(luca) check why the signs are inverted and fix the API call
        let mut rot = rotation.to_array();
        rot[0] = -rot[0];
        rot[1] = -rot[1];
        rot[2] = -rot[2];

        let pose = Pose {
            trans: (trans + offset).into(),
            rot: Rotation::Quat(rot),
        };
        Self {
            pose,
            name: NameInSite(name.into()),
            marker: Default::default(),
        }
    }
}
