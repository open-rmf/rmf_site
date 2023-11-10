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

use bevy::prelude::*;

/// Used to mark halo meshes so their rotations can be animated
#[derive(Debug, Component)]
pub struct Spinning {
    period: f32,
}

impl Spinning {
    pub fn new(period: f32) -> Self {
        Self { period }
    }
}

impl Default for Spinning {
    fn default() -> Self {
        Self { period: 2. }
    }
}

#[derive(Debug, Component)]
pub struct Bobbing {
    period: f32,
    heights: (f32, f32),
}

impl Bobbing {
    #[allow(dead_code)]
    pub fn between(h_min: f32, h_max: f32) -> Self {
        Self {
            heights: (h_min, h_max),
            ..default()
        }
    }
}

impl Default for Bobbing {
    fn default() -> Self {
        Self {
            period: 2.,
            heights: (0., 0.2),
        }
    }
}

impl From<(f32, f32)> for Bobbing {
    fn from(heights: (f32, f32)) -> Self {
        Self {
            heights,
            ..default()
        }
    }
}

pub fn set_bobbing(
    entity: Entity,
    min_height: f32,
    max_height: f32,
    q_bobbing: &mut Query<&mut Bobbing>,
) {
    if let Some(mut b) = q_bobbing.get_mut(entity).ok() {
        b.heights = (min_height, max_height);
    }
}

pub fn update_spinning_animations(
    mut spinners: Query<(&mut Transform, &Spinning, &ComputedVisibility)>,
    now: Res<Time>,
) {
    for (mut tf, spin, visibility) in &mut spinners {
        if visibility.is_visible_in_view() {
            let angle = 2. * std::f32::consts::PI * now.elapsed_seconds() / spin.period;
            tf.as_mut().rotation = Quat::from_rotation_z(angle);
        }
    }
}

pub fn update_bobbing_animations(
    mut bobbers: Query<(&mut Transform, &Bobbing, &ComputedVisibility)>,
    now: Res<Time>,
) {
    for (mut tf, bob, visibility) in &mut bobbers {
        if visibility.is_visible_in_view() {
            let theta = 2. * std::f32::consts::PI * now.elapsed_seconds() / bob.period;
            let dh = bob.heights.1 - bob.heights.0;
            tf.as_mut().translation[2] = dh * (1. - theta.cos()) / 2.0 + bob.heights.0;
        }
    }
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_spinning_animations, update_bobbing_animations),
        );
    }
}
