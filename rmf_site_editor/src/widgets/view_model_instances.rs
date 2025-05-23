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
        Affiliation, CurrentScenario, Delete, GetModifier, Group, InstanceModifier, Members,
        ModelMarker, Modifier, NameInSite, Property, ScenarioMarker, ScenarioModifiers,
        UpdateInstance, UpdateInstanceEvent,
    },
    widgets::{prelude::*, SelectorWidget},
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, ImageButton, ScrollArea, Ui};
use rmf_site_format::{InstanceMarker, SiteID};

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
        (
            Entity,
            &'static ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
        With<ScenarioMarker>,
    >,
    current_scenario: ResMut<'w, CurrentScenario>,
    get_modifier: GetModifier<'w, 's, InstanceModifier>,
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
                            .id_salt(desc_name.0.clone())
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
                                            &self.get_modifier,
                                        );
                                        show_model_instance(
                                            ui,
                                            instance_name,
                                            instance_entity,
                                            &mut self.selector,
                                            &mut self.delete,
                                            &mut self.update_instance,
                                            &self.get_modifier,
                                            current_scenario_entity,
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
                                        &self.get_modifier,
                                    );
                                    show_model_instance(
                                        ui,
                                        instance_name,
                                        *instance_entity,
                                        &mut self.selector,
                                        &mut self.delete,
                                        &mut self.update_instance,
                                        &self.get_modifier,
                                        current_scenario_entity,
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

fn check_instance_modifier_inclusion(
    instance_modifier: &InstanceModifier,
    instance_entity: Entity,
    scenario_entity: Entity,
    get_modifier: &GetModifier<InstanceModifier>,
) -> bool {
    let visibility: Visibility = instance_modifier
        .get()
        .or_else(|| {
            instance_modifier.retrieve_inherited(instance_entity, scenario_entity, get_modifier)
        })
        .unwrap_or(Visibility::get_fallback(instance_entity, scenario_entity));
    match visibility {
        Visibility::Hidden => false,
        _ => true,
    }
}

/// Count the number of scenarios a model instance is included in
pub fn count_scenarios(
    scenarios: &Query<
        (Entity, &ScenarioModifiers<Entity>, &Affiliation<Entity>),
        With<ScenarioMarker>,
    >,
    instance: Entity,
    get_modifier: &GetModifier<InstanceModifier>,
) -> i32 {
    scenarios.iter().fold(0, |x, (e, _, _)| {
        if get_modifier
            .get(e, instance)
            .is_some_and(|instance_modifier| {
                check_instance_modifier_inclusion(instance_modifier, instance, e, get_modifier)
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
    get_modifier: &GetModifier<InstanceModifier>,
    scenario: Entity,
    scenario_count: i32,
    icons: &Res<Icons>,
) {
    let instance_modifier = get_modifier
        .scenarios
        .get(scenario)
        .ok()
        .and_then(|(scenario_modifiers, _)| scenario_modifiers.get(&instance))
        .and_then(|e| get_modifier.modifiers.get(*e).ok());
    ui.horizontal(|ui| {
        // Selector widget
        selector.show_widget(instance, ui);
        // Include/hide model instance
        // Toggle between 3 visibility modes: Include -> Inherited -> Hidden
        // If this is a root scenario, we won't include the Inherited option
        if let Some(instance_modifier) = instance_modifier {
            match instance_modifier {
                InstanceModifier::Added(_) => {
                    if ui
                        .add(ImageButton::new(icons.show.egui()))
                        .on_hover_text("Model instance is included in this scenario")
                        .clicked()
                    {
                        // If this is a root scenario or Added modifier, toggle to Hidden
                        // Note: all modifiers are Added in root scenarios
                        update_instance.write(UpdateInstanceEvent {
                            scenario,
                            instance,
                            update: UpdateInstance::Hide,
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
                            update_instance.write(UpdateInstanceEvent {
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
                            update_instance.write(UpdateInstanceEvent {
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
                        update_instance.write(UpdateInstanceEvent {
                            scenario,
                            instance,
                            update: UpdateInstance::Include,
                        });
                    }
                }
            }
        } else {
            // Modifier is inherited
            if ui
                .add(ImageButton::new(icons.link.egui()))
                .on_hover_text("Model instance visibility is inherited in this scenario")
                .clicked()
            {
                update_instance.write(UpdateInstanceEvent {
                    scenario,
                    instance,
                    update: UpdateInstance::Hide,
                });
            }
        }
        // Delete instance from this site (all scenarios)
        if ui
            .add(ImageButton::new(icons.trash.egui()))
            .on_hover_text("Remove instance from all scenarios")
            .clicked()
        {
            delete.write(Delete::new(instance));
        }
        // Name of model instance and scenario count
        ui.label(format!("{}", name.0)).on_hover_text(format!(
            "Instance is included in {} scenarios",
            scenario_count
        ));
    });
}
