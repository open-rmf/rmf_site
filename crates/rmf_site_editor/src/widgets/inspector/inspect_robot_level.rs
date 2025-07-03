/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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
        CurrentScenario, GetModifier, LevelElevation, Modifier, NameInSite, RobotLevel,
        UpdateModifier, UpdateModifierEvent,
    },
    widgets::{prelude::*, Inspect},
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, Ui};

#[derive(SystemParam)]
pub struct InspectRobotLevel<'w, 's> {
    current_scenario: Res<'w, CurrentScenario>,
    get_modifier: GetModifier<'w, 's, Modifier<RobotLevel<Entity>>>,
    levels: Query<'w, 's, (Entity, &'static NameInSite), With<LevelElevation>>,
    update_modifier: EventWriter<'w, UpdateModifierEvent<RobotLevel<Entity>>>,
}

impl<'w, 's> ShareableWidget for InspectRobotLevel<'w, 's> {}

impl<'w, 's> WidgetSystem<Inspect> for InspectRobotLevel<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Some(current_scenario_entity) = params.current_scenario.0 else {
            return;
        };
        let Ok((scenario_modifiers, parent_scenario)) =
            params.get_modifier.scenarios.get(current_scenario_entity)
        else {
            return;
        };

        let level_modifier = params.get_modifier.get(current_scenario_entity, selection);
        let (selected_level_entity, selected_level_name) = match level_modifier
            .and_then(|m| m.0)
            .and_then(|e| params.levels.get(e).ok())
        {
            Some((level_entity, name)) => (Some(level_entity), name.0.clone()),
            None => (None, "No level assigned".to_string()),
        };

        let mut new_level_entity = selected_level_entity;
        ui.horizontal(|ui| {
            ui.label("Robot Level");
            ComboBox::from_id_salt("select_robot_level")
                .selected_text(selected_level_name)
                .show_ui(ui, |ui| {
                    for (entity, level_name) in params.levels.iter() {
                        ui.selectable_value(
                            &mut new_level_entity,
                            Some(entity),
                            level_name.0.clone(),
                        );
                    }
                    // Add an option to remove selected robot from any level
                    ui.selectable_value(
                        &mut new_level_entity,
                        None,
                        "Remove from all levels".to_string(),
                    );
                });
        });

        let has_modifier = scenario_modifiers
            .get(&selection)
            .is_some_and(|e| params.get_modifier.modifiers.get(*e).is_ok());
        if has_modifier && parent_scenario.0.is_some() {
            if ui
                .button("Reset level")
                .on_hover_text("Reset to parent scenario level")
                .clicked()
            {
                params.update_modifier.write(UpdateModifierEvent::new(
                    current_scenario_entity,
                    selection,
                    UpdateModifier::Reset,
                ));
            }
        }

        if new_level_entity != selected_level_entity {
            params.update_modifier.write(UpdateModifierEvent::new(
                current_scenario_entity,
                selection,
                UpdateModifier::Modify(RobotLevel(new_level_entity)),
            ));
        }
    }
}
