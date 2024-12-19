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
        (
            Entity,
            &mut Selected,
            &mut Hovered,
            &mut Affiliation<Entity>,
            Option<Ref<Tasks<Entity>>>,
        ),
        (With<ModelMarker>, Without<Group>),
    >,
    mut locations: Query<&mut Selected, (With<LocationTags>, Without<ModelMarker>)>,
    mut removed_components: RemovedComponents<Tasks<Entity>>,
) {
    for (instance_entity, mut instance_selected, mut instance_hovered, affiliation, tasks) in
        &mut model_instances
    {
        let mut is_description_selected = false;
        if let Some(description_entity) = affiliation.0 {
            if let Ok((_, description_selected, description_hovered)) =
                model_descriptions.get(description_entity)
            {
                if description_selected.cue() {
                    instance_selected
                        .support_selected
                        .insert(description_entity);
                    is_description_selected = true;
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

        // When an instance is selected, select all locations supporting it
        if let Some(tasks) = tasks {
            // When tasks for an instance have changed, reset all locations from supporting this instance
            if tasks.is_changed() {
                for mut location_selected in locations.iter_mut() {
                    location_selected.support_selected.remove(&instance_entity);
                }
            }

            if let Some(task_location) = tasks.0.first().and_then(|t| t.location()) {
                if let Ok(mut location_selected) = locations.get_mut(task_location.0) {
                    if instance_selected.cue() && !is_description_selected {
                        location_selected.support_selected.insert(instance_entity);
                    } else {
                        location_selected.support_selected.remove(&instance_entity);
                    }
                }
            }
        }
    }

    // When instances are removed, prevent any location from supporting them
    for removed in removed_components.read() {
        for mut location_selected in locations.iter_mut() {
            location_selected.support_selected.remove(&removed);
        }
    }
}
