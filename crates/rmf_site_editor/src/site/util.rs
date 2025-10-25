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

use crate::site::*;
use crate::CurrentWorkspace;
use bevy::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn line_stroke_transform(p_start: &Vec3, p_end: &Vec3, width: f32) -> Transform {
    let dp = *p_end - *p_start;
    let length = dp.length();

    let yaw = dp.y.atan2(dp.x);
    // TODO(@mxgrey): tilt does not seem to be working as intended. Lanes that
    // connect from floor into lift is sinking into the floor side. Investigate
    // this further to fix the tilt calculation.
    // let tilt = dp.z.atan2(dp.x.abs());
    let tilt = 0.0;
    let center = (*p_start + *p_end) / 2.0;
    Transform {
        translation: Vec3::new(center.x, center.y, center.z),
        rotation: Quat::from_euler(EulerRot::ZYX, yaw, -tilt, 0.),
        scale: Vec3::new(length, width, 1.),
        ..default()
    }
}

pub fn get_current_workspace_path(
    current_workspace: Res<CurrentWorkspace>,
    site_files: Query<&DefaultFile>,
) -> Option<PathBuf> {
    let root_entity = (*current_workspace).root?;
    site_files.get(root_entity).map(|f| f.0.clone()).ok()
}

/// This component indicates what labels are used to refer to the start/left
/// end/right anchors of an edge.
#[derive(Component, Clone, Copy, Debug)]
pub enum EdgeLabels {
    StartEnd,
    LeftRight,
}

impl Default for EdgeLabels {
    fn default() -> Self {
        Self::StartEnd
    }
}

impl EdgeLabels {
    pub fn start(&self) -> &'static str {
        match self {
            Self::StartEnd => "start",
            Self::LeftRight => "left",
        }
    }

    pub fn end(&self) -> &'static str {
        match self {
            Self::StartEnd => "end",
            Self::LeftRight => "right",
        }
    }

    pub fn side(&self, side: Side) -> &'static str {
        match side {
            Side::Left => self.start(),
            Side::Right => self.end(),
        }
    }
}

#[derive(Component, Debug, Default, Clone, Deref, DerefMut)]
pub struct Dependents(pub HashSet<Entity>);

impl Dependents {
    pub fn single(dependent: Entity) -> Self {
        Dependents(HashSet::from_iter([dependent]))
    }
}
