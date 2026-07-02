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

use crate::{interaction::*, site::*};
use bevy::prelude::*;

pub fn update_model_instance_visual_cues(
    model_descriptions: Query<
        (Entity, &Selected, &Hovered),
        (
            With<ModelMarker>,
            With<Group>,
            Or<(Changed<Hovered>, Changed<Selected>)>,
        ),
    >,
    mut model_instances: Query<
        (&mut Selected, &mut Hovered, &mut Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
) {
    for (mut instance_selected, mut instance_hovered, affiliation) in &mut model_instances {
        if let Some(description_entity) = affiliation.0 {
            if let Ok((_, description_selected, description_hovered)) =
                model_descriptions.get(description_entity)
            {
                if description_selected.cue() {
                    instance_selected
                        .support_selected
                        .insert(description_entity);
                } else {
                    instance_selected
                        .support_selected
                        .remove(&description_entity);
                }
                if description_hovered.cue() {
                    instance_hovered.support_hovering.insert(description_entity);
                } else {
                    instance_hovered
                        .support_hovering
                        .remove(&description_entity);
                }
            }
        }
    }
}
