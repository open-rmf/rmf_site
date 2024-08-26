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
    interaction::{Select, Selection},
    site::{
        Category, Change, ChangeCurrentScenario, CurrentScenario, Delete, NameInSite,
        RemoveScenario, Scenario, ScenarioMarker,
    },
    widgets::prelude::*,
    CurrentWorkspace, Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, Color32, ScrollArea, Ui};
use rmf_site_format::{Angle, InstanceMarker, Pose, ScenarioBundle, SiteID};

const INSTANCES_VIEWER_HEIGHT: f32 = 200.0;

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
    instances: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            &'static Category,
            Option<&'static SiteID>,
        ),
        With<InstanceMarker>,
    >,
    selection: Res<'w, Selection>,
    select: EventWriter<'w, Select>,
    delete: EventWriter<'w, Delete>,
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
            if let Ok((_, name, mut scenario)) = self.scenarios.get_mut(current_scenario_entity) {
                ui.horizontal(|ui| {
                    ui.label("Selected: ");
                    let mut new_name = name.0.clone();
                    if ui.text_edit_singleline(&mut new_name).changed() {
                        self.change_name
                            .send(Change::new(NameInSite(new_name), current_scenario_entity));
                    }
                    if ui
                        .button("❌")
                        .on_hover_text("Delete this scenario and all its child scenarios")
                        .clicked()
                    {
                        self.remove_scenario
                            .send(RemoveScenario(current_scenario_entity));
                    }
                });
                ui.label("From Previous:");
                // Added
                collapsing_instance_viewer(
                    &format!("Added: {}", scenario.added_instances.len()),
                    "scenario_added_instances",
                    ui,
                    |ui| {
                        for (entity, pose) in scenario.added_instances.iter() {
                            if let Ok((_, name, category, site_id)) = self.instances.get(*entity) {
                                ui.horizontal(|ui| {
                                    instance_selector(
                                        ui,
                                        name,
                                        site_id,
                                        category,
                                        entity,
                                        &self.selection,
                                        &mut self.select,
                                    );
                                    if ui.button("❌").on_hover_text("Remove instance").clicked() {
                                        self.delete.send(Delete::new(*entity));
                                    }
                                });
                                formatted_pose(ui, pose);
                            } else {
                                warn!("Instance entity {:?} does not exist, or has invalid components", entity);
                            }
                        }
                    },
                );
                // Moved
                let mut undo_moved_ids = Vec::new();
                collapsing_instance_viewer(
                    &format!("Moved: {}", scenario.moved_instances.len()),
                    "scenario_moved_instances",
                    ui,
                    |ui| {
                        for (id, (entity, pose)) in scenario.moved_instances.iter().enumerate() {
                            if let Ok((_, name, category, site_id)) = self.instances.get(*entity) {
                                ui.horizontal(|ui| {
                                    instance_selector(
                                        ui,
                                        name,
                                        site_id,
                                        category,
                                        entity,
                                        &self.selection,
                                        &mut self.select,
                                    );
                                    if ui.button("↩").on_hover_text("Undo move").clicked() {
                                        undo_moved_ids.push(id);
                                    }
                                });
                                formatted_pose(ui, pose);
                            } else {
                                warn!("Instance entity {:?} does not exist, or has invalid components", entity);
                            }
                        }
                    },
                );
                // Removed
                let mut undo_removed_ids = Vec::new();
                collapsing_instance_viewer(
                    &format!("Removed: {}", scenario.removed_instances.len()),
                    "scenario_removed_instances",
                    ui,
                    |ui| {
                        for (id, entity) in scenario.removed_instances.iter().enumerate() {
                            if let Ok((_, name, category, site_id)) = self.instances.get(*entity) {
                                ui.horizontal(|ui| {
                                    instance_selector(
                                        ui,
                                        name,
                                        site_id,
                                        category,
                                        entity,
                                        &self.selection,
                                        &mut self.select,
                                    );
                                    if ui.button("↺").on_hover_text("Restore instance").clicked()
                                    {
                                        undo_removed_ids.push(id);
                                    }
                                });
                            } else {
                                warn!("Instance entity {:?} does not exist, or has invalid components", entity);
                            }
                        }
                    },
                );

                // Trigger an update if the scenario has been modified
                let modified = !undo_removed_ids.is_empty() || !undo_moved_ids.is_empty();
                for id in undo_removed_ids {
                    scenario.removed_instances.remove(id);
                }
                for id in undo_moved_ids {
                    scenario.moved_instances.remove(id);
                }
                if modified {
                    self.change_current_scenario
                        .send(ChangeCurrentScenario(current_scenario_entity));
                }
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
                let mut cmd = self
                    .commands
                    .spawn(ScenarioBundle::<Entity>::from_name_parent(
                        self.display_scenarios.new_scenario_name.clone(),
                        match self.display_scenarios.is_new_scenario_root {
                            true => None,
                            false => self.current_scenario.0,
                        },
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

/// Creates a collasible header exposing a scroll area for viewing instances
fn collapsing_instance_viewer<R>(
    header_name: &str,
    id: &str,
    ui: &mut Ui,
    add_contents: impl FnOnce(&mut Ui) -> R,
) {
    CollapsingHeader::new(header_name)
        .id_source(id)
        .default_open(false)
        .show(ui, |ui| {
            ScrollArea::vertical()
                .max_height(INSTANCES_VIEWER_HEIGHT)
                .show(ui, add_contents);
        });
}

/// Creates a selectable label for an instance
fn instance_selector(
    ui: &mut Ui,
    name: &NameInSite,
    site_id: Option<&SiteID>,
    category: &Category,
    entity: &Entity,
    selection: &Selection,
    select: &mut EventWriter<Select>,
) {
    if ui
        .selectable_label(
            selection.0.is_some_and(|s| s == *entity),
            format!(
                "{} #{}",
                category.label(),
                site_id
                    .map(|s| s.0.to_string())
                    .unwrap_or("unsaved".to_string()),
            ),
        )
        .clicked()
    {
        select.send(Select(Some(*entity)));
    };
    ui.label(format!("[{}]", name.0));
}

/// Creates a formatted label for a pose
fn formatted_pose(ui: &mut Ui, pose: &Pose) {
    ui.colored_label(
        Color32::GRAY,
        format!(
            "[x: {:.3}, y: {:.3}, z: {:.3}, yaw: {:.3}]",
            pose.trans[0],
            pose.trans[1],
            pose.trans[2],
            match pose.rot.yaw() {
                Angle::Rad(r) => r,
                Angle::Deg(d) => d.to_radians(),
            }
        ),
    );
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
