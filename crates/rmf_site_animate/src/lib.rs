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

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::Quat;
use bevy_reflect::Reflect;
use bevy_render::view::ViewVisibility;
use bevy_time::Time;
use bevy_transform::prelude::*;
use bevy_utils::prelude::*;

/// Marks an entity's visual cue model to spin.
#[derive(Debug, Component, Reflect)]
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

/// Marks an entity's visual cue model to bob.
#[derive(Debug, Component, Reflect)]
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

fn update_spinning_animations(
    mut spinners: Query<(&mut Transform, &Spinning, &ViewVisibility)>,
    now: Res<Time>,
) {
    for (mut tf, spin, visibility) in &mut spinners {
        if **visibility {
            let angle = 2. * std::f32::consts::PI * now.elapsed_secs() / spin.period;
            tf.as_mut().rotation = Quat::from_rotation_z(angle);
        }
    }
}

fn update_bobbing_animations(
    mut bobbers: Query<(&mut Transform, &Bobbing, &ViewVisibility)>,
    now: Res<Time>,
) {
    for (mut tf, bob, visibility) in &mut bobbers {
        if **visibility {
            let theta = 2. * std::f32::consts::PI * now.elapsed_secs() / bob.period;
            let dh = bob.heights.1 - bob.heights.0;
            tf.as_mut().translation[2] = dh * (1. - theta.cos()) / 2.0 + bob.heights.0;
        }
    }
}

/// Plugin for running systems for diferent visual cue animation components.
pub struct VisualCueAnimationsPlugin;

impl Plugin for VisualCueAnimationsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Bobbing>()
            .register_type::<Spinning>()
            .add_systems(
                Update,
                (update_spinning_animations, update_bobbing_animations),
            );
    }
}
