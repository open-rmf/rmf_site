/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::{interaction::*, layers, site::*};

#[derive(Component, Clone, Copy)]
pub struct Popup {
    regular: f32,
    hovered: f32,
    selected: f32,
}

pub fn add_popups(
    mut commands: Commands,
    new_poppers: Query<Entity, Or<(Added<LocationTags>, Added<FiducialMarker>)>>,
) {
    for e in &new_poppers {
        commands.entity(e).insert(Popup {
            regular: 0.,
            hovered: layers::ZLayer::HoveredLane.to_z(),
            selected: layers::ZLayer::SelectedLane.to_z(),
        });
    }
}

// TODO(@mxgrey): Merge this implementation with the popup implementation for
// lanes at some point.
pub fn update_popups(
    mut objects: Query<
        (&Hovered, &Selected, &Popup, &mut Transform),
        Or<(Changed<Hovered>, Changed<Selected>, Changed<Popup>)>,
    >,
) {
    for (hovered, selected, popup, mut tf) in &mut objects {
        if hovered.is_hovered {
            tf.translation.z = popup.hovered;
        } else if selected.cue() {
            tf.translation.z = popup.selected;
        } else {
            tf.translation.z = popup.regular;
        }
    }
}
