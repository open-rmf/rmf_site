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
        Affiliation, CurrentScenario, Delete, DrawingMarker, FloorMarker, LevelElevation,
        LevelProperties, NameInSite, Scenario, ScenarioMarker,
    },
    widgets::{prelude::*, Icons},
    AppState, CurrentWorkspace, RecencyRanking,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, DragValue, ImageButton, Ui};
use std::cmp::{Ordering, Reverse};

/// Add a plugin for viewing and editing a list of all levels
#[derive(Default)]
pub struct ViewScenariosPlugin {}

impl Plugin for ViewScenariosPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenarioDisplay>()
            .add_plugins(PropertiesTilePlugin::<ViewScenarios>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewScenarios<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    scenarios: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static Scenario<Entity>),
        With<ScenarioMarker>,
    >,
    display_scenarios: ResMut<'w, ScenarioDisplay>,
    current_scenario: ResMut<'w, CurrentScenario>,
    commands: Commands<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewScenarios<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        CollapsingHeader::new("Scenarios")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewScenarios<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        // Show scenarios, starting from the root
        let mut scenario_version: Vec<u32> = vec![1];
        self.scenarios
            .iter()
            .filter(|(_, _, scenario)| scenario.parent_scenario.0.is_none())
            .for_each(|(scenario_entity, _, _)| {
                show_scenario_widget(
                    ui,
                    scenario_entity,
                    scenario_version.clone(),
                    &self.children,
                    &self.scenarios,
                );
                scenario_version[0] += 1;
            });
    }
}

fn show_scenario_widget(
    ui: &mut Ui,
    scenario_entity: Entity,
    scenario_version: Vec<u32>,
    q_children: &Query<&'static Children>,
    q_scenario: &Query<
        (Entity, &'static NameInSite, &'static Scenario<Entity>),
        With<ScenarioMarker>,
    >,
) {
    let (entity, name, scenario) = q_scenario.get(scenario_entity).unwrap();
    ui.horizontal(|ui| {
        // Select
        if ui
            .add(bevy_egui::egui::RadioButton::new(false, ""))
            .clicked()
        {
            println!("Select scenario {}", name.0);
        }

        // Add sub scenario
        if ui
            .add(Button::new("+"))
            .on_hover_text(&format!("Add child scenario for {}", name.0))
            .clicked()
        {
            println!("Add child scenario for {}", name.0);
        }

        // Show version
        ui.label(if scenario_version.len() > 1 {
            scenario_version
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(".")
        } else {
            format!("{}.0", scenario_version[0])
        });

        // Renameable label
        let mut new_name = name.0.clone();
        if ui.text_edit_singleline(&mut new_name).changed() {
            println!("Rename scenario {}", name.0);
        }
    });

    CollapsingHeader::new("Properties")
        .default_open(false)
        .show(ui, |ui| {
            ui.label(format!("Added: {}", scenario.added_model_instances.len()));
            ui.label(format!("Moved: {}", scenario.moved_model_instances.len()));
            ui.label(format!(
                "Removed: {}",
                scenario.removed_model_instances.len()
            ));
        });

    CollapsingHeader::new("Sub Scenarios: 0")
        .default_open(false)
        .show(ui, |ui| {});
}

#[derive(Resource)]
pub struct ScenarioDisplay {
    pub new_name: String,
    pub order: Vec<Vec<Entity>>,
}

impl Default for ScenarioDisplay {
    fn default() -> Self {
        Self {
            new_name: "New Scenario".to_string(),
            order: Vec::new(),
        }
    }
}
