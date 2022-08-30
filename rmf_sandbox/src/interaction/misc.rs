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

use crate::{
    interaction::*,
};
use rmf_site_format::{
    Floor, Wall, Model, Door, Lift, Measurement,
};

// TODO(MXG): Customize the behavior of floor, wall, and model visual cues.
// For now we just use the same interaction behavior for all of them.
#[derive(Component)]
pub struct MiscVisualCue;

pub fn add_misc_visual_cues(
    mut commands: Commands,
    new_entities: Query<Entity, Or<(
        Added<Floor<Entity>>,
        Added<Wall<Entity>>,
        Added<Model>,
        Added<Door<Entity>>,
        Added<Lift<Entity>>,
        Added<Measurement<Entity>>,
    )>>,
) {
    for e in &new_entities {
        commands.entity(e)
            .insert(MiscVisualCue)
            .insert(Selectable::new(e));
    }
}

pub fn update_misc_visual_cues(
    misc: Query<(Entity, &Hovered), (With<MiscVisualCue>, Changed<Hovered>)>,
    removed: RemovedComponents<MiscVisualCue>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
) {
    for (e, hovering) in &misc {
        if hovering.cue() {
            cursor.add_dependent(e, &mut visibility);
        } else {
            cursor.remove_dependent(e, &mut visibility);
        }
    }

    for e in removed.iter() {
        cursor.remove_dependent(e, &mut visibility);
    }
}
