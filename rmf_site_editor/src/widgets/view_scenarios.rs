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
        AddedInstance, Change, ChangeCurrentScenario, CurrentScenario, Instance, NameInSite,
        RemoveScenario, Scenario, ScenarioMarker,
    },
    widgets::prelude::*,
    CurrentWorkspace, Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Align, Button, CollapsingHeader, Color32, Layout, Ui};
use rmf_site_format::ScenarioBundle;
use std::collections::BTreeMap;

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
    commands: Commands<'w, 's>,
    children: Query<'w, 's, &'static Children>,
    parent: Query<'w, 's, &'static Parent>,
    scenarios: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static mut Scenario<Entity>),
        With<ScenarioMarker>,
    >,
    change_name: EventWriter<'w, Change<NameInSite>>,
    change_current_scenario: EventWriter<'w, ChangeCurrentScenario>,
    remove_scenario: EventWriter<'w, RemoveScenario>,
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
                    let mut new_name = name.0.clone();
                    if ui.text_edit_singleline(&mut new_name).changed() {
                        self.change_name
                            .send(Change::new(NameInSite(new_name), current_scenario_entity));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button("❌")
                            .on_hover_text("Delete this scenario and all its child scenarios")
                            .clicked()
                        {
                            self.remove_scenario
                                .send(RemoveScenario(current_scenario_entity));
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
                let instances: BTreeMap<Entity, Instance> =
                    match self.display_scenarios.is_new_scenario_root {
                        true => BTreeMap::<Entity, Instance>::new(),
                        false => self
                            .current_scenario
                            .0
                            .and_then(|e| self.scenarios.get(e).ok())
                            .map(|(_, _, scenario)| {
                                scenario
                                    .instances
                                    .clone()
                                    .into_iter()
                                    .map(|(e, i)| match i {
                                        Instance::Modified(modified) => (
                                            e,
                                            Instance::Added(AddedInstance {
                                                pose: modified.pose,
                                            }),
                                        ),
                                        _ => (e, i),
                                    })
                                    .collect::<BTreeMap<Entity, Instance>>()
                            })
                            .unwrap_or(BTreeMap::<Entity, Instance>::new()),
                    };

                let mut cmd = self
                    .commands
                    .spawn(ScenarioBundle::<Entity>::from_name_parent(
                        self.display_scenarios.new_scenario_name.clone(),
                        match self.display_scenarios.is_new_scenario_root {
                            true => None,
                            false => self.current_scenario.0,
                        },
                        &instances,
                    ));
                match self.display_scenarios.is_new_scenario_root {
                    true => {
                        if let Some(site_entity) = self.current_workspace.root {
                            cmd.set_parent(site_entity);
                        }
                    }
                    false => {
                        if let Some(current_scenario_entity) = self.current_scenario.0 {
                            cmd.set_parent(current_scenario_entity);
                        }
                    }
                }
                let scenario_entity = cmd.id();
                self.change_current_scenario
                    .send(ChangeCurrentScenario(scenario_entity));
            }
            let mut new_name = self.display_scenarios.new_scenario_name.clone();
            if ui
                .text_edit_singleline(&mut new_name)
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
        let mut version = 1;
        self.scenarios
            .iter()
            .filter(|(_, _, scenario)| scenario.parent_scenario.0.is_none())
            .filter(|(scenario_entity, _, _)| {
                self.current_workspace
                    .root
                    .is_some_and(|e| e == **self.parent.get(*scenario_entity).ok().unwrap())
            })
            .for_each(|(scenario_entity, _, _)| {
                show_scenario_widget(
                    ui,
                    &mut self.change_name,
                    &mut self.change_current_scenario,
                    &mut self.current_scenario,
                    scenario_entity,
                    vec![version],
                    &self.children,
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
    current_scenario: &mut CurrentScenario,
    scenario_entity: Entity,
    scenario_version: Vec<u32>,
    q_children: &Query<&'static Children>,
    q_scenario: &Query<
        (Entity, &'static NameInSite, &'static mut Scenario<Entity>),
        With<ScenarioMarker>,
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
            change_current_scenario.send(ChangeCurrentScenario(entity));
        }
        ui.colored_label(Color32::DARK_GRAY, scenario_version_str.clone());
        let mut new_name = name.0.clone();
        if ui.text_edit_singleline(&mut new_name).changed() {
            change_name.send(Change::new(NameInSite(new_name), entity));
        }
    });

    // Display children recursively
    // The subversion is used as an id_source so that egui does not
    // generate errors when collapsing headers of the same name are created
    let mut subversion = 1;
    let children = q_children.get(scenario_entity);
    CollapsingHeader::new(format!(
        "Child Scenarios:  {}",
        children.map(|c| c.len()).unwrap_or(0)
    ))
    .default_open(true)
    .id_source(scenario_version_str.clone())
    .show(ui, |ui| {
        if let Ok(children) = children {
            for child in children.iter() {
                if let Ok(_) = q_scenario.get(*child) {
                    let mut version = scenario_version.clone();
                    version.push(subversion);
                    show_scenario_widget(
                        ui,
                        change_name,
                        change_current_scenario,
                        current_scenario,
                        *child,
                        version,
                        q_children,
                        q_scenario,
                        icons,
                    );
                    subversion += 1;
                }
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
