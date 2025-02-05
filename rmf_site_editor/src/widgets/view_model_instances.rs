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
        Affiliation, Category, ChangeCurrentScenario, CurrentScenario, Delete, Group, ModelMarker,
        NameInSite, ResetPose, Scenario, ScenarioMarker,
    },
    widgets::prelude::*,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, Color32, ScrollArea, Ui};
use rmf_site_format::{Angle, InstanceMarker, Pose, SiteID};
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
    scenarios: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static mut Scenario<Entity>),
        With<ScenarioMarker>,
    >,
    change_current_scenario: EventWriter<'w, ChangeCurrentScenario>,
    current_scenario: ResMut<'w, CurrentScenario>,
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
    reset_pose: EventWriter<'w, ResetPose>,
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
        if let Some(current_scenario_entity) = self.current_scenario.0 {
            if let Ok((_, _, mut scenario)) = self.scenarios.get_mut(current_scenario_entity) {
                let current_scenario_instances = scenario.instances.clone();
                let mut non_affiliated_instances = HashSet::<Entity>::new();
                ScrollArea::vertical()
                    .max_height(INSTANCES_VIEWER_HEIGHT)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (desc_entity, desc_name, _) in self.model_descriptions.iter() {
                            CollapsingHeader::new(desc_name.0.clone())
                                .id_source(desc_name.0.clone())
                                .default_open(false)
                                .show(ui, |ui| {
                                    for (
                                        instance_entity,
                                        instance_name,
                                        category,
                                        affiliation,
                                        instance_site_id,
                                    ) in self.model_instances.iter_mut()
                                    {
                                        if affiliation.0.is_some_and(|e| e == desc_entity) {
                                            show_model_instance(
                                                ui,
                                                instance_name,
                                                instance_site_id,
                                                category,
                                                &instance_entity,
                                                &self.selection,
                                                &mut self.select,
                                                &mut self.delete,
                                                &mut self.reset_pose,
                                                &mut scenario.instances,
                                            );
                                        } else {
                                            non_affiliated_instances.insert(instance_entity);
                                        }
                                    }
                                });
                        }
                        CollapsingHeader::new("Non-affiliated instances")
                            .default_open(false)
                            .show(ui, |ui| {
                                if non_affiliated_instances.is_empty() {
                                    ui.label("No orphan model instances.");
                                }
                                for instance_entity in non_affiliated_instances.iter() {
                                    if let Ok((_, instance_name, category, _, instance_site_id)) =
                                        self.model_instances.get_mut(*instance_entity)
                                    {
                                        show_model_instance(
                                            ui,
                                            instance_name,
                                            instance_site_id,
                                            category,
                                            instance_entity,
                                            &self.selection,
                                            &mut self.select,
                                            &mut self.delete,
                                            &mut self.reset_pose,
                                            &mut scenario.instances,
                                        );
                                    }
                                }
                            });
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

/// Show a widget for users to interact with a model instance
fn show_model_instance(
    ui: &mut Ui,
    name: &NameInSite,
    site_id: Option<&SiteID>,
    category: &Category,
    entity: &Entity,
    selection: &Selection,
    select: &mut EventWriter<Select>,
    delete: &mut EventWriter<Delete>,
    reset_pose: &mut EventWriter<ResetPose>,
    instances: &mut HashMap<Entity, ((Pose, bool), bool)>,
) {
    // Instance selector
    ui.horizontal(|ui| {
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
    });

    if let Some(((pose, moved), included)) = instances.get_mut(entity) {
        // Include/hide model instance
        ui.horizontal(|ui| {
            ui.checkbox(included, "Include")
                .on_hover_text("Include this model instance in the current scenario.");

            // Reset instance pose to parent scenario
            ui.add_enabled_ui(*moved, |ui| {
                if ui
                    .button("↩")
                    .on_hover_text("Reset to parent scenario pose")
                    .clicked()
                {
                    reset_pose.send(ResetPose(*entity));
                }
            });
            // Delete instance from this site (all scenarios)
            if ui.button("❌").on_hover_text("Remove instance").clicked() {
                delete.send(Delete::new(*entity));
            }
        });
        // Format instance pose
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
}
