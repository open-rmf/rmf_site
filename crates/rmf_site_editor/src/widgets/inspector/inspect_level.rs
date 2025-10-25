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
        CurrentScenario, GetModifier, LevelElevation, Modifier, NameInSite, OnLevel, UpdateModifier,
    },
    widgets::{prelude::*, Inspect},
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, Ui};
use rmf_site_egui::{ShareableWidget, WidgetSystem};

#[derive(SystemParam)]
pub struct InspectLevel<'w, 's> {
    commands: Commands<'w, 's>,
    current_scenario: Res<'w, CurrentScenario>,
    get_modifier: GetModifier<'w, 's, Modifier<OnLevel<Entity>>>,
    levels: Query<'w, 's, (Entity, &'static NameInSite), With<LevelElevation>>,
    level_models: Query<'w, 's, (), With<OnLevel<Entity>>>,
}

impl<'w, 's> ShareableWidget for InspectLevel<'w, 's> {}

impl<'w, 's> WidgetSystem<Inspect> for InspectLevel<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        if params.level_models.get(selection).is_err() {
            return;
        }
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
            ui.label("On Level");
            ComboBox::from_id_salt("select_element_level")
                .selected_text(selected_level_name)
                .show_ui(ui, |ui| {
                    for (entity, level_name) in params.levels.iter() {
                        ui.selectable_value(
                            &mut new_level_entity,
                            Some(entity),
                            level_name.0.clone(),
                        );
                    }
                });
        });

        let has_modifier = scenario_modifiers
            .get(&selection)
            .is_some_and(|e| params.get_modifier.modifiers.get(*e).is_ok());
        if has_modifier && parent_scenario.0.is_some() {
            if ui
                .button("Reset level")
                .on_hover_text("Reset to use the same level as the parent scenario")
                .clicked()
            {
                params
                    .commands
                    .trigger(UpdateModifier::<OnLevel<Entity>>::reset(
                        current_scenario_entity,
                        selection,
                    ));
            }
        }

        if new_level_entity != selected_level_entity {
            params
                .commands
                .entity(selection)
                .insert(OnLevel(new_level_entity));
        }
    }
}
