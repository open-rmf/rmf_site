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
        Affiliation, ChangeCurrentScenario, CurrentScenario, Delete, Group, Instance, Members,
        ModelMarker, NameInSite, Scenario, ScenarioMarker, UpdateInstance, UpdateInstanceType,
    },
    widgets::{prelude::*, SelectorWidget},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, Color32, ScrollArea, Ui};
use rmf_site_format::{Angle, InstanceMarker, SiteID};
use std::collections::BTreeMap;

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
    members: Query<'w, 's, &'static Members>,
    model_descriptions: Query<
        'w,
        's,
        (Entity, &'static NameInSite, Option<&'static SiteID>),
        (With<ModelMarker>, With<Group>),
    >,
    model_instances: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static Affiliation<Entity>),
        With<InstanceMarker>,
    >,
    selector: SelectorWidget<'w, 's>,
    delete: EventWriter<'w, Delete>,
    update_instance: EventWriter<'w, UpdateInstance>,
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
                let mut unaffiliated_instances = Vec::<Entity>::new();
                ScrollArea::vertical()
                    .max_height(INSTANCES_VIEWER_HEIGHT)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (desc_entity, desc_name, _) in self.model_descriptions.iter() {
                            let Ok(members) = self.members.get(desc_entity) else {
                                continue;
                            };
                            CollapsingHeader::new(desc_name.0.clone())
                                .id_source(desc_name.0.clone())
                                // TODO(@xiyuoh) true if model is selected
                                .default_open(false)
                                .show(ui, |ui| {
                                    for member in members.iter() {
                                        let Ok((instance_entity, instance_name, affiliation)) =
                                            self.model_instances.get_mut(*member)
                                        else {
                                            continue;
                                        };
                                        if affiliation.0.is_some_and(|e| e == desc_entity) {
                                            show_model_instance(
                                                ui,
                                                instance_name,
                                                instance_entity,
                                                &mut self.selector,
                                                &mut self.delete,
                                                &mut self.update_instance,
                                                &mut scenario.instances,
                                            );
                                        } else {
                                            unaffiliated_instances.push(instance_entity);
                                        }
                                    }
                                });
                        }
                        CollapsingHeader::new("Unaffiliated instances")
                            // TODO(@xiyuoh) true if model is selected
                            .default_open(false)
                            .show(ui, |ui| {
                                if unaffiliated_instances.is_empty() {
                                    ui.label("No orphan model instances.");
                                }
                                for instance_entity in unaffiliated_instances.iter() {
                                    if let Ok((_, instance_name, _)) =
                                        self.model_instances.get_mut(*instance_entity)
                                    {
                                        show_model_instance(
                                            ui,
                                            instance_name,
                                            *instance_entity,
                                            &mut self.selector,
                                            &mut self.delete,
                                            &mut self.update_instance,
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
    entity: Entity,
    selector: &mut SelectorWidget,
    delete: &mut EventWriter<Delete>,
    update_instance: &mut EventWriter<UpdateInstance>,
    instances: &mut BTreeMap<Entity, Instance>,
) {
    // Instance selector
    ui.horizontal(|ui| {
        selector.show_widget(entity, ui);
        ui.label(format!("{}", name.0));
    });

    if let Some(instance) = instances.get_mut(&entity) {
        let (mut included, pose) = match instance {
            Instance::Added(added) => (true, added.pose.clone()),
            Instance::Modified(modified) => (true, modified.pose.clone()),
            Instance::Hidden(hidden) => (false, hidden.pose.clone()),
        };

        // Include/hide model instance
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut included, "Include")
                .on_hover_text("Include/Hide this model instance in the current scenario")
                .changed()
            {
                if included {
                    update_instance.send(UpdateInstance {
                        entity,
                        update_type: UpdateInstanceType::Include,
                    });
                } else {
                    update_instance.send(UpdateInstance {
                        entity,
                        update_type: UpdateInstanceType::Hide,
                    });
                }
            }

            // Reset instance pose to parent scenario
            ui.add_enabled_ui(
                match instance {
                    Instance::Modified(_) => true,
                    _ => false,
                },
                |ui| {
                    if ui
                        .button("↩")
                        .on_hover_text("Reset to parent scenario pose")
                        .clicked()
                    {
                        update_instance.send(UpdateInstance {
                            entity,
                            update_type: UpdateInstanceType::ResetPose,
                        });
                    }
                },
            );
            // Delete instance from this site (all scenarios)
            if ui
                .button("❌")
                .on_hover_text("Remove instance from all scenarios")
                .clicked()
            {
                delete.send(Delete::new(entity));
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
