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

use crate::{
    site::{
        Affiliation, Change, ChangeCurrentScenario, ChangeDefaultScenario, CreateScenario,
        CurrentScenario, DefaultScenario, NameInSite, RemoveScenario, ScenarioModifiers,
    },
    widgets::prelude::*,
    CurrentWorkspace, Icons,
};
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemParam},
    prelude::*,
};
use bevy_egui::egui::{
    Align, Button, CollapsingHeader, Color32, Image, Layout, TextEdit, Ui, Widget,
};
use rmf_site_egui::*;
use std::collections::HashMap;

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
    child_of: Query<'w, 's, &'static ChildOf>,
    scenarios: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static Affiliation<Entity>),
        With<ScenarioModifiers<Entity>>,
    >,
    change_name: EventWriter<'w, Change<NameInSite>>,
    change_current_scenario: EventWriter<'w, ChangeCurrentScenario>,
    change_default_scenario: EventWriter<'w, ChangeDefaultScenario>,
    create_new_scenario: EventWriter<'w, CreateScenario>,
    remove_scenario: EventWriter<'w, RemoveScenario>,
    default_scenario: Res<'w, DefaultScenario>,
    display_scenarios: ResMut<'w, ScenarioDisplay>,
    current_scenario: ResMut<'w, CurrentScenario>,
    current_workspace: Res<'w, CurrentWorkspace>,
    icons: Res<'w, Icons>,
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
        // Current Selection Info
        if let Some(current_scenario_entity) = self.current_scenario.0 {
            if let Ok((_, name, _)) = self.scenarios.get_mut(current_scenario_entity) {
                ui.horizontal(|ui| {
                    ui.label("Selected: ");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button("❌")
                            .on_hover_text("Delete this scenario and all its child scenarios")
                            .clicked()
                        {
                            self.remove_scenario
                                .write(RemoveScenario(current_scenario_entity));
                        }
                        let mut new_name = name.0.clone();
                        if TextEdit::singleline(&mut new_name)
                            .desired_width(ui.available_width())
                            .ui(ui)
                            .changed()
                        {
                            self.change_name
                                .write(Change::new(NameInSite(new_name), current_scenario_entity));
                        }
                    });
                });
            }
        } else {
            ui.label("No scenario selected");
        }

        // Create Scenario
        ui.separator();
        if self.current_scenario.is_none() {
            self.display_scenarios.is_new_scenario_root = true;
        }
        ui.horizontal(|ui| {
            ui.label("Add Scenario: ");
            if ui
                .selectable_label(self.display_scenarios.is_new_scenario_root, "Root")
                .on_hover_text("Add a new root scenario")
                .clicked()
            {
                self.display_scenarios.is_new_scenario_root = true;
            };
            ui.add_enabled_ui(self.current_scenario.is_some(), |ui| {
                if ui
                    .selectable_label(!self.display_scenarios.is_new_scenario_root, "Child")
                    .on_hover_text("Add a new child scenario to the selected scenario")
                    .clicked()
                {
                    self.display_scenarios.is_new_scenario_root = false;
                }
            });
        });
        ui.horizontal(|ui| {
            if ui.add(Button::image(self.icons.add.egui())).clicked() {
                self.create_new_scenario.write(CreateScenario {
                    name: Some(self.display_scenarios.new_scenario_name.clone()),
                    parent: match self.display_scenarios.is_new_scenario_root {
                        true => None,
                        false => self.current_scenario.0,
                    },
                });
            }
            let mut new_name = self.display_scenarios.new_scenario_name.clone();
            if TextEdit::singleline(&mut new_name)
                .desired_width(ui.available_width())
                .ui(ui)
                .on_hover_text("Name for the new scenario")
                .changed()
            {
                self.display_scenarios.new_scenario_name = new_name;
            }
        });

        // Scenario Tree starting from root scenarios
        ui.separator();
        // A version string is used to differentiate scenarios, and to allow
        // egui to distinguish between collapsing headers with the same name

        // Construct scenario children
        let mut scenario_children = HashMap::<Entity, Vec<Entity>>::new();
        for (e, _, parent_scenario) in self.scenarios.iter() {
            if let Some(parent_entity) = parent_scenario.0 {
                if let Some(children) = scenario_children.get_mut(&parent_entity) {
                    children.push(e);
                } else {
                    scenario_children.insert(parent_entity, vec![e]);
                }
            }
        }
        let mut version = 1;
        self.scenarios
            .iter()
            .filter(|(_, _, parent_scenario)| parent_scenario.0.is_none())
            .filter(|(scenario_entity, _, _)| {
                self.current_workspace.root.is_some_and(|e| {
                    self.child_of
                        .get(*scenario_entity)
                        .is_ok_and(|co| e == co.parent())
                })
            })
            .for_each(|(scenario_entity, _, _)| {
                show_scenario_widget(
                    ui,
                    &mut self.change_name,
                    &mut self.change_current_scenario,
                    &mut self.change_default_scenario,
                    &mut self.current_scenario,
                    &self.default_scenario,
                    scenario_entity,
                    vec![version],
                    &scenario_children,
                    &self.scenarios,
                    &self.icons,
                );
                version += 1;
            });
    }
}

fn show_scenario_widget(
    ui: &mut Ui,
    change_name: &mut EventWriter<Change<NameInSite>>,
    change_current_scenario: &mut EventWriter<ChangeCurrentScenario>,
    change_default_scenario: &mut EventWriter<ChangeDefaultScenario>,
    current_scenario: &mut CurrentScenario,
    default_scenario: &DefaultScenario,
    scenario_entity: Entity,
    scenario_version: Vec<u32>,
    scenario_children: &HashMap<Entity, Vec<Entity>>,
    q_scenario: &Query<
        (Entity, &'static NameInSite, &'static Affiliation<Entity>),
        With<ScenarioModifiers<Entity>>,
    >,
    icons: &Res<Icons>,
) {
    let (entity, name, _) = q_scenario.get(scenario_entity).unwrap();
    let scenario_version_str = scenario_version
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join(".");

    // Scenario version and name, e.g. 1.2.3 My Scenario
    ui.horizontal(|ui| {
        if ui.radio(Some(entity) == **current_scenario, "").clicked() {
            change_current_scenario.write(ChangeCurrentScenario(entity));
        }
        ui.colored_label(Color32::DARK_GRAY, scenario_version_str.clone());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(Image::new(icons.home.egui()));
            let is_default = default_scenario.0.is_some_and(|e| e == entity);
            let mut toggle_default = is_default;
            ui.checkbox(&mut toggle_default, "")
                .on_hover_text("Make this the default scenario");
            if toggle_default && !is_default {
                change_default_scenario.write(ChangeDefaultScenario(Some(entity)));
            } else if is_default && !toggle_default {
                change_default_scenario.write(ChangeDefaultScenario(None));
            }
            let mut new_name = name.0.clone();
            if TextEdit::singleline(&mut new_name)
                .desired_width(ui.available_width())
                .ui(ui)
                .changed()
            {
                change_name.write(Change::new(NameInSite(new_name), entity));
            }
        });
    });

    // Display children recursively
    // The subversion is used as an id_salt so that egui does not
    // generate errors when collapsing headers of the same name are created
    let mut subversion = 1;
    let children = scenario_children.get(&scenario_entity);
    let num_children = children.map(|c| c.len()).unwrap_or(0);
    CollapsingHeader::new(format!("Child Scenarios:  {}", num_children))
        .default_open(true)
        .id_salt(scenario_version_str.clone())
        .show(ui, |ui| {
            if let Some(children) = children {
                for child in children.iter() {
                    let mut version = scenario_version.clone();
                    version.push(subversion);
                    show_scenario_widget(
                        ui,
                        change_name,
                        change_current_scenario,
                        change_default_scenario,
                        current_scenario,
                        default_scenario,
                        *child,
                        version,
                        &scenario_children,
                        q_scenario,
                        icons,
                    );
                    subversion += 1;
                }
            } else {
                ui.label("No Child Scenarios");
            }
        });
}

#[derive(Resource)]
pub struct ScenarioDisplay {
    pub new_scenario_name: String,
    pub is_new_scenario_root: bool,
}

impl Default for ScenarioDisplay {
    fn default() -> Self {
        Self {
            new_scenario_name: "<Unnamed>".to_string(),
            is_new_scenario_root: true,
        }
    }
}
