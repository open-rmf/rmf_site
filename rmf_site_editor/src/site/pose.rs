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
use rmf_site_format::Pose;

pub fn update_transforms_for_changed_poses(
    mut poses: Query<(Entity, &Pose, Option<&mut Transform>), Changed<Pose>>,
    mut commands: Commands,
) {
    for (e, pose, tf) in &mut poses {
        let transform = pose.transform();
        if let Some(mut tf) = tf {
            tf.translation = transform.translation;
            tf.rotation = transform.rotation;
        } else {
            commands
                .entity(e)
                .insert(transform)
                .insert(GlobalTransform::default());
        }
    }
}
