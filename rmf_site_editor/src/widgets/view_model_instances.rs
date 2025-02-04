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
    interaction::{Select, Selection},
    site::{
        Affiliation, Category, Change, ChangeCurrentScenario, CurrentScenario, Delete, Group,
        ModelMarker, NameInSite, Scenario, ScenarioMarker,
    },
    widgets::prelude::*,
    CurrentWorkspace, Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Align, Button, CollapsingHeader, Color32, Layout, ScrollArea, Ui};
use rmf_site_format::{Angle, InstanceMarker, Pose, ScenarioBundle, SiteID};
use std::collections::{HashMap, HashSet};

const INSTANCES_VIEWER_HEIGHT: f32 = 200.0;

/// Add a plugin for viewing and editing a list of all levels
#[derive(Default)]
pub struct ViewModelInstancesPlugin {}

impl Plugin for ViewModelInstancesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PropertiesTilePlugin::<ViewModelInstances>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewModelInstances<'w, 's> {
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
    current_scenario: ResMut<'w, CurrentScenario>,
    current_workspace: Res<'w, CurrentWorkspace>,
    model_descriptions: Query<
        'w,
        's,
        (Entity, &'static NameInSite, Option<&'static SiteID>),
        (With<ModelMarker>, With<Group>),
    >,
    model_instances: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            &'static Category,
            &'static Affiliation<Entity>,
            Option<&'static SiteID>,
        ),
        With<InstanceMarker>,
    >,
    selection: Res<'w, Selection>,
    select: EventWriter<'w, Select>,
    delete: EventWriter<'w, Delete>,
    icons: Res<'w, Icons>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewModelInstances<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        CollapsingHeader::new("Models")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewModelInstances<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        // some label here maybe
        if let Some(current_scenario_entity) = self.current_scenario.0 {
            if let Ok((_, name, mut scenario)) = self.scenarios.get_mut(current_scenario_entity) {
                ui.horizontal(|ui| {
                    ui.label("Selected: ");
                    let mut new_name = name.0.clone();
                    if ui.text_edit_singleline(&mut new_name).changed() {
                        self.change_name
                            .send(Change::new(NameInSite(new_name), current_scenario_entity));
                    }
                });
                let mut current_scenario_instances = scenario.instances.clone();
                let mut non_affiliated_instances = HashSet::<Entity>::new();
                // let mut hide_instance_ids = Vec::new();
                ScrollArea::vertical()
                    .max_height(INSTANCES_VIEWER_HEIGHT)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Loop through every model description
                        for (desc_entity, desc_name, desc_site_id) in self.model_descriptions.iter()
                        {
                            CollapsingHeader::new(desc_name.0.clone())
                                .id_source(desc_name.0.clone())
                                .default_open(false)
                                .show(ui, |ui| {
                                    // Loop through every model instance
                                    for (
                                        instance_entity,
                                        instance_name,
                                        category,
                                        affiliation,
                                        instance_site_id,
                                    ) in self.model_instances.iter_mut()
                                    {
                                        if affiliation.0.is_some_and(|e| e == desc_entity) {
                                            ui.horizontal(|ui| {
                                                instance_selector(
                                                    ui,
                                                    instance_name,
                                                    instance_site_id,
                                                    category,
                                                    &instance_entity,
                                                    &self.selection,
                                                    &mut self.select,
                                                );
                                            });
                                            if let Some((pose, included)) =
                                                scenario.instances.get_mut(&instance_entity)
                                            {
                                                //
                                                ui.horizontal(|ui| {
                                                    ui.checkbox(included, "Include");
                                                    // .on_hover_text("Include this model instance in the current scenario.");
                                                    if ui
                                                        .button("❌")
                                                        .on_hover_text("Remove instance")
                                                        .clicked()
                                                    {
                                                        self.delete
                                                            .send(Delete::new(instance_entity));
                                                    }
                                                });
                                                formatted_pose(ui, pose);
                                            }
                                        } else {
                                            non_affiliated_instances.insert(instance_entity);
                                        }
                                    }
                                });
                        }
                        // TODO(@xiyuoh) add a single collapsing header for all the non affiliated instances
                    });
                // Update visibility by triggering ChangeCurrentScenario event
                if scenario.instances != current_scenario_instances {
                    self.change_current_scenario
                        .send(ChangeCurrentScenario(current_scenario_entity));
                }
            }
        }
    }
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
        select.send(Select::new(Some(*entity)));
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
