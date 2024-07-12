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
    site::{Change, CurrentScenario, NameInSite, Scenario, ScenarioBundle, ScenarioMarker},
    widgets::prelude::*,
    AppState,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, Ui};

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
    change_name: EventWriter<'w, Change<NameInSite>>,
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
        // Show scenarios window, star
        let mut version = 1;
        self.scenarios
            .iter()
            .filter(|(_, _, scenario)| scenario.parent_scenario.0.is_none())
            .for_each(|(scenario_entity, _, _)| {
                show_scenario_widget(
                    ui,
                    &mut self.commands,
                    &mut self.change_name,
                    &mut self.current_scenario,
                    scenario_entity,
                    vec![version],
                    &self.children,
                    &self.scenarios,
                );
                version += 1;
            });
    }
}

fn show_scenario_widget(
    ui: &mut Ui,
    commands: &mut Commands,
    change_name: &mut EventWriter<Change<NameInSite>>,
    current_scenario: &mut CurrentScenario,
    scenario_entity: Entity,
    scenario_version: Vec<u32>,
    q_children: &Query<&'static Children>,
    q_scenario: &Query<
        (Entity, &'static NameInSite, &'static Scenario<Entity>),
        With<ScenarioMarker>,
    >,
) {
    let (entity, name, scenario) = q_scenario.get(scenario_entity).unwrap();
    let scenario_version_str = scenario_version
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join(".");
    ui.horizontal(|ui| {
        // Selection
        if ui
            .add(bevy_egui::egui::RadioButton::new(
                current_scenario.is_some_and(|e| e == entity),
                "",
            ))
            .clicked()
        {
            //TODO: Replace this with the appropriiate change
            *current_scenario = CurrentScenario(Some(entity));
        }
        // Version and name label
        ui.label(scenario_version_str.clone());
        let mut new_name = name.0.clone();
        if ui.text_edit_singleline(&mut new_name).changed() {
            change_name.send(Change::new(NameInSite(new_name), entity));
        }
    });
    ui.horizontal(|ui| {
        let children = q_children.get(scenario_entity);
        let mut subversion = 1;
        CollapsingHeader::new(format!("Sub-Scenarios {}", scenario_version_str))
            .default_open(false)
            .show(ui, |ui| {
                if let Ok(children) = children {
                    for child in children.iter() {
                        if let Ok(_) = q_scenario.get(*child) {
                            let mut version = scenario_version.clone();
                            version.push(subversion);
                            show_scenario_widget(
                                ui,
                                commands,
                                change_name,
                                current_scenario,
                                *child,
                                version,
                                q_children,
                                q_scenario,
                            );
                            subversion += 1;
                        }
                    }
                } else {
                    ui.label("No sub-scenarios");
                }
            });
        // Add child scenario
        if ui
            .button(" + ")
            .on_hover_text(format!("Add a child scenario to {}", name.0))
            .clicked()
        {
            commands
                .spawn(ScenarioBundle {
                    name: name.clone(),
                    scenario: Scenario::from_parent(entity),
                    marker: ScenarioMarker,
                })
                .set_parent(entity);
        }
    });
}

#[derive(Resource, Default)]
pub struct ScenarioDisplay {
    pub new_root_scenario_name: String,
}
