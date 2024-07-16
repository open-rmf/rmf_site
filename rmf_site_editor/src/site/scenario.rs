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

use crate::site::CurrentScenario;
use bevy::prelude::*;
use rmf_site_format::{Group, ModelMarker, NameInSite, Pose};

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

pub fn update_current_scenario(
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    scenarios: Query<Entity, &NameInSite>,
) {
    for ChangeCurrentScenario(new_scenario_entity) in change_current_scenario.read() {
        *current_scenario = CurrentScenario(Some(*new_scenario_entity));
        println!("Changed scenario");
    }
}

pub fn update_scenario_properties(
    current_scenario: Res<CurrentScenario>,
    changed_models: Query<(Entity, &NameInSite, Ref<Pose>), (With<ModelMarker>, Without<Group>)>,
) {
    for (e, name, pose) in changed_models.iter() {
        if pose.is_added() {
            println!("Added: {}", name.0);
        }

        if !pose.is_added() && pose.is_changed() {
            println!("Moved: {}", name.0)
        }
    }
}
