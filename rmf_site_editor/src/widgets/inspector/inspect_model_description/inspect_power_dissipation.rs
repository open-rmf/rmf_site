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

use super::{
    get_selected_description_entity, inspect_robot_properties::RobotPropertyWidgetRegistry,
};
use crate::{
    site::{
        AmbientSystem, Change, Group, MechanicalSystem, ModelMarker, ModelProperty,
        ModelPropertyQuery, PowerDissipation, RecallAmbientSystem, RecallMechanicalSystem,
        RecallPlugin, RecallPowerDissipation, RecallPropertyKind, Robot, RobotProperty,
        RobotPropertyKind, UpdateRobotPropertyKinds,
    },
    widgets::{
        inspector::inspect_model_description::RobotPropertyKindWidgetRegistration, prelude::*,
        Inspect,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, Grid, Ui};
use serde_json::{Map, Value};
use smallvec::SmallVec;

#[derive(SystemParam)]
pub struct InspectPowerDissipation<'w, 's> {
    robot_property_widgets: Res<'w, RobotPropertyWidgetRegistry>,
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    power_dissipation: Query<'w, 's, &'static PowerDissipation, (With<ModelMarker>, With<Group>)>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    children: Query<'w, 's, &'static Children>,
    recall_power_dissipation:
        Query<'w, 's, &'static RecallPowerDissipation, (With<ModelMarker>, With<Group>)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectPowerDissipation<'w, 's> {
    fn show(
        Inspect {
            selection,
            inspection: _,
            panel,
        }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };
        let Ok(ModelProperty(robot)) = params.model_descriptions.get(description_entity) else {
            return;
        };

        let recall_power_dissipation: Option<PowerDissipation> =
            match params.recall_power_dissipation.get(description_entity) {
                Ok(recall) => Some(PowerDissipation {
                    config: recall.config.clone().unwrap_or_default(),
                }),
                Err(_) => None,
            };

        let mut new_robot = robot.clone();
        let label = PowerDissipation::label();
        let power_dissipation = params.power_dissipation.get(description_entity).ok();

        if params.robot_property_widgets.get(&label).is_none() {
            ui.label(format!("No {} kind registered.", label));
            return;
        };

        let mut update_robot = true;
        let mut has_property = power_dissipation.is_some();
        ui.checkbox(&mut has_property, label.clone());
        if !has_property {
            if power_dissipation.is_some() {
                // RobotProperty toggled from enabled to disabled
                new_robot.properties.remove(&label);
            } else {
                return;
            }
        } else {
            let new_power_dissipation = match power_dissipation {
                Some(p) => p.clone(),
                None => match recall_power_dissipation {
                    Some(r) => r,
                    None => PowerDissipation::default(),
                },
            };

            update_robot = if power_dissipation.is_some_and(|p| *p == new_power_dissipation) {
                false
            } else if new_power_dissipation.is_default() {
                // Setting value as null to filter out invalid data on save
                new_robot.properties.insert(label, Value::Null);
                true
            } else {
                if let Ok(new_value) = serde_json::to_value(new_power_dissipation) {
                    new_robot.properties.insert(label, new_value);
                }
                true
            };
        }

        if update_robot {
            params
                .change_robot_property
                .write(Change::new(ModelProperty(new_robot), description_entity));
        }

        // Show children widgets
        if let Some(widget_registration) = params
            .robot_property_widgets
            .get(&PowerDissipation::label())
        {
            let children_widgets: Result<SmallVec<[_; 16]>, _> = params
                .children
                .get(widget_registration.property_widget)
                .map(|c| c.iter().collect());
            let Ok(children_widgets) = children_widgets else {
                return;
            };

            ui.indent("configure_power_dissipation", |ui| {
                for child in children_widgets {
                    let inspect = Inspect {
                        selection,
                        inspection: child,
                        panel,
                    };
                    ui.add_space(10.0);
                    let _ = world.try_show_in(child, inspect, ui);
                }
            });
        }
    }
}

/// This systems checks for changes in PowerDissipation and updates changes in the respective
/// dissipation kinds.
pub fn update_power_dissipation_kinds_component<T: RobotPropertyKind>(
    mut commands: Commands,
    mut update_robot_property_kinds: EventReader<UpdateRobotPropertyKinds>,
) {
    for update in update_robot_property_kinds.read() {
        if update.label != PowerDissipation::label() {
            continue;
        }

        let label = T::label();
        let Some(power_dissipation_map) = update
            .value
            .as_object()
            .and_then(|map| map.get("config"))
            .and_then(|config| config.as_object())
        else {
            continue;
        };
        if let Some(kind_component) = power_dissipation_map
            .get(&label)
            .and_then(|v| serde_json::from_value::<T>(v.clone()).ok())
        {
            commands.entity(update.entity).insert(kind_component);
        } else {
            commands.entity(update.entity).remove::<T>();
        }
    }
}

#[derive(Default)]
pub struct InspectMechanicalSystemPlugin {}

impl Plugin for InspectMechanicalSystemPlugin {
    fn build(&self, app: &mut App) {
        let property_label = PowerDissipation::label();
        let Some(inspector) = app
            .world()
            .resource::<RobotPropertyWidgetRegistry>()
            .0
            .get(&property_label)
            .map(|registration| registration.property_widget.clone())
        else {
            return;
        };
        let widget = Widget::<Inspect>::new::<InspectMechanicalSystem>(app.world_mut());
        app.world_mut().spawn(widget).insert(ChildOf(inspector));
        app.world_mut()
            .resource_mut::<RobotPropertyWidgetRegistry>()
            .0
            .get_mut(&property_label)
            .map(|registration| {
                registration.kinds.insert(
                    MechanicalSystem::label(),
                    RobotPropertyKindWidgetRegistration {
                        default: || serde_json::to_value(MechanicalSystem::default()),
                    },
                );
            });

        app.add_plugins(RecallPlugin::<RecallMechanicalSystem>::default())
            .add_systems(
                PostUpdate,
                update_power_dissipation_kinds_component::<MechanicalSystem>,
            );
    }
}

#[derive(SystemParam)]
pub struct InspectMechanicalSystem<'w, 's> {
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
    model_descriptions: Query<
        'w,
        's,
        (
            &'static ModelProperty<Robot>,
            Option<&'static MechanicalSystem>,
        ),
        (With<ModelMarker>, With<Group>),
    >,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    recall_mechanical_system:
        Query<'w, 's, &'static RecallMechanicalSystem, (With<ModelMarker>, With<Group>)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectMechanicalSystem<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);

        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };

        let label = MechanicalSystem::label();
        let Ok((ModelProperty(robot), opt_mechanical_system)) =
            params.model_descriptions.get_mut(description_entity)
        else {
            return;
        };

        let mut new_robot = robot.clone();
        let mut has_mechanical_system = opt_mechanical_system.is_some();
        let power_dissipation_label = PowerDissipation::label();

        ui.checkbox(&mut has_mechanical_system, label.clone());
        if !has_mechanical_system {
            if opt_mechanical_system.is_some() {
                // Mechanical toggled from enabled to disabled, update Robot
                if let Some(power_dissipation_map) = new_robot
                    .properties
                    .get_mut(&power_dissipation_label)
                    .and_then(|value| value.as_object_mut())
                    .and_then(|map| map.get_mut("config"))
                    .and_then(|config| config.as_object_mut())
                {
                    power_dissipation_map.remove(&label);
                }
            } else {
                return;
            }
        } else {
            let mechanical_system = match opt_mechanical_system {
                Some(m) => m.clone(),
                None => match params.recall_mechanical_system.get(description_entity) {
                    Ok(recall) => recall.assume(),
                    Err(_) => MechanicalSystem::default(),
                },
            };

            let mut new_mechanical_system = mechanical_system.clone();

            if opt_mechanical_system.is_some() {
                ui.indent("inspect_mechanical_system_properties", |ui| {
                    Grid::new("inspect_mechanical_system")
                        .num_columns(3)
                        .show(ui, |ui| {
                            ui.label("Mass");
                            ui.add(
                                DragValue::new(&mut new_mechanical_system.mass)
                                    .range(0_f32..=std::f32::INFINITY)
                                    .speed(0.01),
                            );
                            ui.label("kg");
                            ui.end_row();

                            ui.label("Moment of Inertia");
                            ui.add(
                                DragValue::new(&mut new_mechanical_system.moment_of_inertia)
                                    .range(0_f32..=std::f32::INFINITY)
                                    .speed(0.01),
                            );
                            ui.label("kgm^2");
                            ui.end_row();

                            ui.label("Friction Coefficient");
                            ui.add(
                                DragValue::new(&mut new_mechanical_system.friction_coefficient)
                                    .range(0_f32..=std::f32::INFINITY)
                                    .speed(0.01),
                            );
                            ui.end_row();
                        });
                });
            }

            if opt_mechanical_system.is_none() || new_mechanical_system != mechanical_system {
                // Update Robot properties
                if let Ok(mech_sys_value) = serde_json::to_value(new_mechanical_system) {
                    if let Some(power_dissipation_map) = new_robot
                        .properties
                        .get_mut(&power_dissipation_label)
                        .and_then(|value| value.as_object_mut())
                        .map(|map| map.entry("config").or_insert(Value::Object(Map::new())))
                        .and_then(|config| config.as_object_mut())
                    {
                        power_dissipation_map.insert(label.clone(), mech_sys_value);
                    }
                }
            } else {
                return;
            }
        }

        params
            .change_robot_property
            .write(Change::new(ModelProperty(new_robot), description_entity));
    }
}

#[derive(Default)]
pub struct InspectAmbientSystemPlugin {}

impl Plugin for InspectAmbientSystemPlugin {
    fn build(&self, app: &mut App) {
        let property_label = PowerDissipation::label();
        let Some(inspector) = app
            .world()
            .resource::<RobotPropertyWidgetRegistry>()
            .0
            .get(&property_label)
            .map(|registration| registration.property_widget.clone())
        else {
            return;
        };
        let widget = Widget::<Inspect>::new::<InspectAmbientSystem>(app.world_mut());
        app.world_mut().spawn(widget).insert(ChildOf(inspector));
        app.world_mut()
            .resource_mut::<RobotPropertyWidgetRegistry>()
            .0
            .get_mut(&property_label)
            .map(|registration| {
                registration.kinds.insert(
                    AmbientSystem::label(),
                    RobotPropertyKindWidgetRegistration {
                        default: || serde_json::to_value(AmbientSystem::default()),
                    },
                );
            });

        app.add_plugins(RecallPlugin::<RecallAmbientSystem>::default())
            .add_systems(
                PostUpdate,
                update_power_dissipation_kinds_component::<AmbientSystem>,
            );
    }
}

#[derive(SystemParam)]
pub struct InspectAmbientSystem<'w, 's> {
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
    model_descriptions: Query<
        'w,
        's,
        (
            &'static ModelProperty<Robot>,
            Option<&'static AmbientSystem>,
        ),
        (With<ModelMarker>, With<Group>),
    >,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    recall_ambient_system:
        Query<'w, 's, &'static RecallAmbientSystem, (With<ModelMarker>, With<Group>)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectAmbientSystem<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);

        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };

        let label = AmbientSystem::label();
        let Ok((ModelProperty(robot), opt_ambient_system)) =
            params.model_descriptions.get_mut(description_entity)
        else {
            return;
        };

        let mut new_robot = robot.clone();
        let mut has_ambient_system = opt_ambient_system.is_some();
        let power_dissipation_label = PowerDissipation::label();

        ui.checkbox(&mut has_ambient_system, label.clone());
        if !has_ambient_system {
            if opt_ambient_system.is_some() {
                // Idle Power toggled from enabled to disabled, update Robot
                if let Some(power_dissipation_map) = new_robot
                    .properties
                    .get_mut(&power_dissipation_label)
                    .and_then(|value| value.as_object_mut())
                    .and_then(|map| map.get_mut("config"))
                    .and_then(|config| config.as_object_mut())
                {
                    power_dissipation_map.remove(&label);
                }
            } else {
                return;
            }
        } else {
            let ambient_system = match opt_ambient_system {
                Some(m) => m.clone(),
                None => match params.recall_ambient_system.get(description_entity) {
                    Ok(recall) => recall.assume(),
                    Err(_) => AmbientSystem::default(),
                },
            };

            let mut new_ambient_system = ambient_system.clone();

            if opt_ambient_system.is_some() {
                ui.indent("inspect_ambient_system_properties", |ui| {
                    Grid::new("inspect_ambient_system")
                        .num_columns(3)
                        .show(ui, |ui| {
                            ui.label("Power");
                            ui.add(
                                DragValue::new(&mut new_ambient_system.idle_power)
                                    .range(0_f32..=std::f32::INFINITY)
                                    .speed(0.01),
                            );
                            ui.label("W");
                            ui.end_row();
                        });
                });
            }

            if opt_ambient_system.is_none() || new_ambient_system != ambient_system {
                // Update Robot properties
                if let Ok(ambient_system_value) = serde_json::to_value(new_ambient_system) {
                    if let Some(power_dissipation_map) = new_robot
                        .properties
                        .get_mut(&power_dissipation_label)
                        .and_then(|value| value.as_object_mut())
                        .map(|map| map.entry("config").or_insert(Value::Object(Map::new())))
                        .and_then(|config| config.as_object_mut())
                    {
                        power_dissipation_map.insert(label.clone(), ambient_system_value);
                    }
                }
            } else {
                return;
            }
        }

        params
            .change_robot_property
            .write(Change::new(ModelProperty(new_robot), description_entity));
    }
}
