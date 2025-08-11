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
    site::{
        Affiliation, GetModifier, Group, IssueKey, ModelMarker, Modifier, NameInSite,
        ScenarioModifiers, StandardProperty,
    },
    Issue, ValidateWorkspace,
};
use bevy::prelude::*;
use rmf_site_format::Pose;
use uuid::Uuid;

impl StandardProperty for Pose {}

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

/// Unique UUID to identify issue of accidentally moved child instance
pub const ACCIDENTALLY_MOVED_INSTANCE_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x39d33dd7e5f3479a82465d4ec8de0961u128);

pub fn check_for_accidentally_moved_instances(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    get_modifier: GetModifier<Modifier<Pose>>,
    instances: Query<
        (Entity, &NameInSite, &Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
    scenarios: Query<
        (
            &NameInSite,
            &ScenarioModifiers<Entity>,
            &Affiliation<Entity>,
        ),
    >,
) {
    for root in validate_events.read() {
        for (scenario_name, scenario_modifiers, parent_scenario) in scenarios.iter() {
            for (instance_entity, instance_name, _) in instances.iter() {
                // Check if this instance-scenario pair has a Pose modifier
                if let Some(child_modifier) = scenario_modifiers
                    .get(&instance_entity)
                    .and_then(|e| get_modifier.modifiers.get(*e).ok())
                {
                    // Pose modifier exists, check this pose against the parent
                    // scenario's pose for the same instance
                    if let Some(parent_modifier) = parent_scenario
                        .0
                        .and_then(|parent| get_modifier.get(parent, instance_entity))
                    {
                        let child_pose = (**child_modifier).transform().translation;
                        let parent_pose = (**parent_modifier).transform().translation;
                        // If the elements of child and parent poses are very close (< 0.01),
                        // raise issue as the child instance might have been accidentally moved
                        if child_pose.abs_diff_eq(parent_pose, 0.01) {
                            let issue = Issue {
                                key: IssueKey {
                                    entities: [instance_entity].into(),
                                    kind: ACCIDENTALLY_MOVED_INSTANCE_ISSUE_UUID,
                                },
                                brief: format!(
                                    "Model instance {:?} in scenario {:?} is very close to \
                                     its parent scenario pose",
                                    instance_name, scenario_name
                                ),
                                hint: "Model instance pose in scenario {:?} is very close to \
                                    its parent pose. Check that the model instance is meant to \
                                    be moved, otherwise select the model and click Reset Pose."
                                    .to_string(),
                            };
                            let issue_id = commands.spawn(issue).id();
                            commands.entity(**root).add_child(issue_id);
                        }
                    }
                }
            }
        }
    }
}
