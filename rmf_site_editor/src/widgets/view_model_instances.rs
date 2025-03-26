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
    interaction::Selection,
    site::{
        scenario::*, Affiliation, CurrentScenario, Delete, Group, InstanceModifier, Members,
        ModelMarker, NameInSite, ScenarioMarker, UpdateInstance, UpdateInstanceEvent,
    },
    widgets::{prelude::*, SelectorWidget},
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, ImageButton, ScrollArea, Ui};
use rmf_site_format::{InstanceMarker, SiteID};
use std::collections::HashMap;

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
        (Entity, &'static NameInSite, &'static Affiliation<Entity>),
        With<ScenarioMarker>,
    >,
    children: Query<'w, 's, &'static Children>,
    current_scenario: ResMut<'w, CurrentScenario>,
    icons: Res<'w, Icons>,
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
    selection: Res<'w, Selection>,
    selector: SelectorWidget<'w, 's>,
    delete: EventWriter<'w, Delete>,
    instance_modifiers:
        Query<'w, 's, (&'static mut InstanceModifier, &'static Affiliation<Entity>)>,
    update_instance: EventWriter<'w, UpdateInstanceEvent>,
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
            let instance_modifier_entities = get_instance_modifier_entities(
                current_scenario_entity,
                &self.children,
                &self.instance_modifiers,
            );
            // Get InstanceModifier components in this scenario
            let scenario_instance_modifiers = instance_modifier_entities.iter().fold(
                HashMap::new(),
                |mut x, (instance_entity, modifier_entity)| {
                    if let Ok((instance, _)) = self.instance_modifiers.get(*modifier_entity) {
                        x.insert(*instance_entity, instance.clone());
                        x
                    } else {
                        x
                    }
                },
            );

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
                            .default_open(self.selection.0.is_some_and(|e| members.contains(&e)))
                            .show(ui, |ui| {
                                for member in members.iter() {
                                    let Ok((instance_entity, instance_name, affiliation)) =
                                        self.model_instances.get_mut(*member)
                                    else {
                                        continue;
                                    };
                                    if affiliation.0.is_some_and(|e| e == desc_entity) {
                                        let scenario_count = count_scenarios(
                                            &self.scenarios,
                                            instance_entity,
                                            &self.children,
                                            &self.instance_modifiers,
                                        );
                                        show_model_instance(
                                            ui,
                                            instance_name,
                                            instance_entity,
                                            &mut self.selector,
                                            &mut self.delete,
                                            &mut self.update_instance,
                                            scenario_instance_modifiers.get(&instance_entity),
                                            current_scenario_entity,
                                            &self.scenarios,
                                            scenario_count,
                                            &self.icons,
                                        );
                                    } else {
                                        unaffiliated_instances.push(instance_entity);
                                    }
                                }
                            });
                    }
                    CollapsingHeader::new("Unaffiliated instances")
                        .default_open(
                            self.selection
                                .0
                                .is_some_and(|e| unaffiliated_instances.contains(&e)),
                        )
                        .show(ui, |ui| {
                            if unaffiliated_instances.is_empty() {
                                ui.label("No orphan model instances.");
                            }
                            for instance_entity in unaffiliated_instances.iter() {
                                if let Ok((_, instance_name, _)) =
                                    self.model_instances.get_mut(*instance_entity)
                                {
                                    let scenario_count = count_scenarios(
                                        &self.scenarios,
                                        *instance_entity,
                                        &self.children,
                                        &self.instance_modifiers,
                                    );
                                    show_model_instance(
                                        ui,
                                        instance_name,
                                        *instance_entity,
                                        &mut self.selector,
                                        &mut self.delete,
                                        &mut self.update_instance,
                                        scenario_instance_modifiers.get(instance_entity),
                                        current_scenario_entity,
                                        &self.scenarios,
                                        scenario_count,
                                        &self.icons,
                                    );
                                }
                            }
                        });
                });
        }
    }
}

pub fn count_scenarios(
    scenarios: &Query<(Entity, &NameInSite, &Affiliation<Entity>), With<ScenarioMarker>>,
    instance: Entity,
    children: &Query<&Children>,
    instance_modifiers: &Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
) -> i32 {
    scenarios.iter().fold(0, |x, (e, _, _)| {
        if find_modifier_for_instance(instance, e, &children, &instance_modifiers)
            .and_then(|modifier_entity| instance_modifiers.get(modifier_entity).ok())
            .is_some_and(|(i, _)| match i {
                InstanceModifier::Hidden => false,
                _ => true,
            })
        {
            x + 1
        } else {
            x
        }
    })
}

/// Show a widget for users to interact with a model instance
fn show_model_instance(
    ui: &mut Ui,
    name: &NameInSite,
    instance: Entity,
    selector: &mut SelectorWidget,
    delete: &mut EventWriter<Delete>,
    update_instance: &mut EventWriter<UpdateInstanceEvent>,
    instance_modifier: Option<&InstanceModifier>,
    scenario: Entity,
    scenarios: &Query<(Entity, &NameInSite, &Affiliation<Entity>), With<ScenarioMarker>>,
    scenario_count: i32,
    icons: &Res<Icons>,
) {
    if let Some(instance_modifier) = instance_modifier {
        ui.horizontal(|ui| {
            // Selector widget
            selector.show_widget(instance, ui);
            // Include/hide model instance
            // Toggle between 3 visibility modes: Include -> Inherited -> Hidden
            // If this is a root scenario, we won't include the Inherited option
            match instance_modifier {
                InstanceModifier::Added(_) => {
                    if ui
                        .add(ImageButton::new(icons.show.egui()))
                        .on_hover_text("Model instance is included in this scenario")
                        .clicked()
                    {
                        // If this is a root scenario, toggle to Hidden modifier
                        // If this is not a root scenario, toggle to Inherited modifier
                        let update = if scenarios.get(scenario).is_ok_and(|(_, _, a)| a.0.is_none())
                        {
                            UpdateInstance::Hide
                        } else {
                            UpdateInstance::ResetVisibility
                        };
                        update_instance.send(UpdateInstanceEvent {
                            scenario,
                            instance,
                            update,
                        });
                    }
                }
                InstanceModifier::Inherited(inherited) => {
                    if inherited.explicit_inclusion {
                        if ui
                            .add(ImageButton::new(icons.show.egui()))
                            .on_hover_text("Model instance is included in this scenario")
                            .clicked()
                        {
                            update_instance.send(UpdateInstanceEvent {
                                scenario,
                                instance,
                                update: UpdateInstance::ResetVisibility,
                            });
                        }
                    } else {
                        if ui
                            .add(ImageButton::new(icons.link.egui()))
                            .on_hover_text(
                                "Model instance visibility is inherited in this scenario",
                            )
                            .clicked()
                        {
                            update_instance.send(UpdateInstanceEvent {
                                scenario,
                                instance,
                                update: UpdateInstance::Hide,
                            });
                        }
                    }
                }
                InstanceModifier::Hidden => {
                    if ui
                        .add(ImageButton::new(icons.hide.egui()))
                        .on_hover_text("Model instance is hidden in this scenario")
                        .clicked()
                    {
                        update_instance.send(UpdateInstanceEvent {
                            scenario,
                            instance,
                            update: UpdateInstance::Include,
                        });
                    }
                }
            }
            // Delete instance from this site (all scenarios)
            if ui
                .add(ImageButton::new(icons.trash.egui()))
                .on_hover_text("Remove instance from all scenarios")
                .clicked()
            {
                delete.send(Delete::new(instance));
            }
            // Name of model instance and scenario count
            ui.label(format!("{}", name.0)).on_hover_text(format!(
                "Instance is included in {} scenarios",
                scenario_count
            ));
        });
    }
}
