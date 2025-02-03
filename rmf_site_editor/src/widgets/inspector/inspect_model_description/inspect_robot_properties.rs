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

use super::{get_selected_description_entity, ModelDescriptionInspector};
use crate::{
    site::{
        update_model_instances, Affiliation, Change, ChangePlugin, Group, ModelMarker,
        ModelProperty, Robot,
    },
    widgets::{prelude::*, Inspect},
    ModelPropertyData,
};
use bevy::{
    ecs::system::{EntityCommands, SystemParam},
    prelude::*,
};
use bevy_egui::egui::{ComboBox, Ui};
use rmf_site_format::{RobotProperty, RobotPropertyKind};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt::Debug;

pub type InsertPropertyKindFn = fn(EntityCommands);
pub type RemovePropertyKindFn = fn(EntityCommands);

/// This resource keeps track of all the properties that can be configured for a robot.
#[derive(Resource)]
pub struct RobotPropertyWidgets(
    pub  HashMap<
        String,
        (
            Entity, // entity id of the widget
            HashMap<String, (InsertPropertyKindFn, RemovePropertyKindFn)>,
        ),
    >,
);

impl FromWorld for RobotPropertyWidgets {
    fn from_world(_world: &mut World) -> Self {
        Self(HashMap::new())
    }
}

#[derive(Default)]
pub struct InspectRobotPropertiesPlugin {}

impl Plugin for InspectRobotPropertiesPlugin {
    fn build(&self, app: &mut App) {
        // Allows us to toggle Robot as a configurable property
        // from the model description inspector
        app.world.init_component::<ModelProperty<Robot>>();
        let component_id = app
            .world
            .components()
            .component_id::<ModelProperty<Robot>>()
            .unwrap();
        app.add_plugins(ChangePlugin::<ModelProperty<Robot>>::default())
            .add_systems(PreUpdate, update_model_instances::<Robot>)
            .init_resource::<ModelPropertyData>()
            .world
            .resource_mut::<ModelPropertyData>()
            .optional
            .insert(
                component_id,
                (
                    "Robot".to_string(),
                    |mut e_cmd| {
                        e_cmd.insert(ModelProperty::<Robot>::default());
                    },
                    |mut e_cmd| {
                        e_cmd.remove::<ModelProperty<Robot>>();
                    },
                ),
            );
        let inspector = app.world.resource::<ModelDescriptionInspector>().id;
        let widget = Widget::<Inspect>::new::<InspectRobotProperties>(&mut app.world);
        let id = app.world.spawn(widget).set_parent(inspector).id();
        app.world.insert_resource(RobotPropertiesInspector { id });
        app.world.init_resource::<RobotPropertyWidgets>();
        app.add_event::<UpdateRobotPropertyKinds>();
    }
}

/// Contains a reference to the robot properties inspector widget.
#[derive(Resource)]
pub struct RobotPropertiesInspector {
    id: Entity,
}

impl RobotPropertiesInspector {
    pub fn get(&self) -> Entity {
        self.id
    }
}

#[derive(SystemParam)]
struct InspectRobotProperties<'w, 's> {
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<Robot>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    inspect_robot_properties: Res<'w, RobotPropertiesInspector>,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectRobotProperties<'w, 's> {
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
        let params = state.get_mut(world);
        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };
        // Ensure that this widget is displayed only when there is a valid Robot property
        let Ok(ModelProperty(_robot)) = params.model_descriptions.get(description_entity) else {
            return;
        };
        ui.label("Robot Properties");

        let children: Result<SmallVec<[_; 16]>, _> = params
            .children
            .get(params.inspect_robot_properties.id)
            .map(|children| children.iter().copied().collect());
        let Ok(children) = children else {
            return;
        };

        ui.indent("inspect_robot_properties", |ui| {
            for child in children {
                let inspect = Inspect {
                    selection,
                    inspection: child,
                    panel,
                };
                ui.add_space(10.0);
                let _ = world.try_show_in(child, inspect, ui);
            }
        });
        ui.add_space(10.0);
    }
}

/// Implement this plugin to add a new configurable robot property of type T to the
/// robot properties inspector.
pub struct InspectRobotPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static
        + Send
        + Sync
        + Default
        + Clone
        + RobotProperty
        + Component
        + for<'de> Deserialize<'de>,
{
    _ignore: std::marker::PhantomData<(W, T)>,
}

impl<W, T> InspectRobotPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static
        + Send
        + Sync
        + Default
        + Clone
        + RobotProperty
        + Component
        + for<'de> Deserialize<'de>,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W, T> Plugin for InspectRobotPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static
        + Send
        + Sync
        + Default
        + Clone
        + RobotProperty
        + Component
        + for<'de> Deserialize<'de>,
{
    fn build(&self, app: &mut App) {
        let inspector = app.world.resource::<RobotPropertiesInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        let id = app.world.spawn(widget).set_parent(inspector).id();
        app.world
            .resource_mut::<RobotPropertyWidgets>()
            .0
            .insert(T::label(), (id, HashMap::new()));
        app.add_systems(PreUpdate, update_robot_properties::<T>);
    }
}

/// Implement this plugin to add a new configurable robot property kind of type T to the
/// robot properties inspector.
pub struct InspectRobotPropertyKindPlugin<W, T, U>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static
        + Send
        + Sync
        + Default
        + Clone
        + RobotPropertyKind
        + Component
        + for<'de> Deserialize<'de>,
    U: 'static + Send + Sync + Default + Clone + RobotProperty + Component,
{
    _ignore: std::marker::PhantomData<(W, T, U)>,
}

impl<W, T, U> InspectRobotPropertyKindPlugin<W, T, U>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static
        + Send
        + Sync
        + Default
        + Clone
        + RobotPropertyKind
        + Component
        + for<'de> Deserialize<'de>,
    U: 'static + Send + Sync + Default + Clone + RobotProperty + Component,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W, T, U> Plugin for InspectRobotPropertyKindPlugin<W, T, U>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static
        + Send
        + Sync
        + Default
        + Clone
        + RobotPropertyKind
        + Component
        + for<'de> Deserialize<'de>,
    U: 'static + Send + Sync + Default + Clone + RobotProperty + Component,
{
    fn build(&self, app: &mut App) {
        let property_label = U::label();
        let Some(inspector) = app
            .world
            .resource::<RobotPropertyWidgets>()
            .0
            .get(&property_label)
            .map(|(id, _)| id.clone())
        else {
            return;
        };
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        app.world.spawn(widget).set_parent(inspector);
        app.world
            .resource_mut::<RobotPropertyWidgets>()
            .0
            .get_mut(&property_label)
            .map(|(_, ref mut m)| {
                m.insert(
                    T::label(),
                    (
                        |mut e_commands| {
                            e_commands.insert(T::default());
                        },
                        |mut e_commands| {
                            e_commands.remove::<T>();
                        },
                    ),
                );
            });
        app.add_systems(PreUpdate, update_robot_property_kinds::<T, U>);
    }
}

#[derive(Debug, Event)]
pub struct UpdateRobotPropertyKinds {
    pub entity: Entity,
    pub label: String,
    pub value: serde_json::Value,
}

/// This system monitors changes to ModelProperty<Robot> and inserts robot property components
/// to the model descriptions
pub fn update_robot_properties<
    'de,
    T: Component + Clone + Default + RobotProperty + DeserializeOwned,
>(
    mut commands: Commands,
    model_properties: Query<(Entity, Ref<ModelProperty<Robot>>), (With<ModelMarker>, With<Group>)>,
    mut removals: RemovedComponents<ModelProperty<Robot>>,
    mut update_robot_property_kinds: EventWriter<UpdateRobotPropertyKinds>,
) {
    // TODO(@xiyuoh) change back to RobotPropertyData now that its no longer widgets
    let property_label = T::label();

    // Remove Robot property entirely
    for description_entity in removals.read() {
        commands.entity(description_entity).remove::<T>();
        update_robot_property_kinds.send(UpdateRobotPropertyKinds {
            entity: description_entity,
            label: property_label.clone(),
            value: serde_json::Value::Object(Map::new()),
        });
    }

    for (entity, robot) in model_properties.iter() {
        if robot.is_changed() {
            let mut value = serde_json::Value::Object(Map::new());
            // Update robot property first
            match robot.0.properties.get(&property_label) {
                Some(property_value) => {
                    if property_value.as_object().is_none_or(|m| m.is_empty()) {
                        commands.entity(entity).insert(T::default());
                    } else if let Ok(property) = serde_json::from_value::<T>(property_value.clone())
                    {
                        commands.entity(entity).insert(property.clone());
                        value = property_value.clone();
                    } else {
                        continue;
                    }
                }
                None => {
                    commands.entity(entity).remove::<T>();
                }
            }
            // Update robot property kinds
            update_robot_property_kinds.send(UpdateRobotPropertyKinds {
                entity,
                label: property_label.clone(),
                value,
            });
        }
    }
}

/// This system inserts or removes robot property kinds when a robot property is updated
pub fn update_robot_property_kinds<
    'de,
    T: Component + Clone + Default + RobotPropertyKind + DeserializeOwned,
    U: Clone + RobotProperty,
>(
    mut commands: Commands,
    mut update_robot_property_kinds: EventReader<UpdateRobotPropertyKinds>,
) {
    for update in update_robot_property_kinds.read() {
        let property_label = U::label();
        let property_kind_label = T::label();
        if update.label != property_label {
            continue;
        }

        if let Some(property) = update.value.as_object() {
            if property
                .get("kind")
                .and_then(|k| k.as_str())
                .is_some_and(|label| label == property_kind_label.as_str())
            {
                match property
                    .get("config")
                    .and_then(|config| serde_json::from_value::<T>(config.clone()).ok())
                {
                    Some(property_kind) => {
                        commands.entity(update.entity).insert(property_kind);
                    }
                    None => {
                        commands.entity(update.entity).insert(T::default());
                    }
                }
                continue;
            }
        }
        commands.entity(update.entity).remove::<T>();
    }
}

/// This system displays each RobotProperty widget and enables users to toggle
/// the properties on and off, and select relevant RobotPropertyKinds.
pub fn show_robot_property_widget<
    'de,
    T: Component + Clone + Debug + Default + PartialEq + RobotProperty + Serialize,
>(
    ui: &mut Ui,
    mut property_query: Query<&T, (With<ModelMarker>, With<Group>)>,
    mut change_robot_property: EventWriter<Change<ModelProperty<Robot>>>,
    robot: &Robot,
    robot_property_widgets: &Res<RobotPropertyWidgets>,
    description_entity: Entity,
) {
    let mut new_robot = robot.clone();
    let property_label = T::label();
    let property = property_query.get_mut(description_entity).ok();

    let Some((_, property_kinds)) = robot_property_widgets.0.get(&property_label) else {
        ui.label(format!("No {} kind registered.", property_label));
        return;
    };

    let mut has_property = property.is_some();
    ui.checkbox(&mut has_property, property_label.clone());
    if !has_property {
        if property.is_some() {
            // RobotProperty toggled from enabled to disabled
            new_robot.properties.remove(&property_label);
        } else {
            return;
        }
    } else {
        let mut new_property = match property {
            Some(p) => p.clone(),
            None => T::default(),
        };
        let selected_property_kind = if !new_property.is_default() {
            new_property.kind().clone()
        } else {
            "Select Kind".to_string()
        };

        ui.indent("configure_".to_owned() + &property_label, |ui| {
            ui.horizontal(|ui| {
                ui.label(property_label.to_owned() + " Kind");
                ComboBox::from_id_source("select_".to_owned() + &property_label + "_kind")
                    .selected_text(selected_property_kind)
                    .show_ui(ui, |ui| {
                        for (kind, _) in property_kinds.iter() {
                            ui.selectable_value(
                                new_property.kind_mut(),
                                kind.clone(),
                                kind.clone(),
                            );
                        }
                    });
            });
        });

        ui.add_space(10.0);

        if property.is_some_and(|p| p.kind() == new_property.kind()) {
            return;
        }
        // Update changes in RobotPropertyKind only, values will be updated in the respective widgets
        if new_property.is_default() {
            // Setting value as null to filter out invalid data on save
            new_robot.properties.insert(property_label, Value::Null);
        } else {
            if let Ok(new_value) = serde_json::to_value(new_property) {
                new_robot.properties.insert(property_label, new_value);
            }
        }
    }
    change_robot_property.send(Change::new(ModelProperty(new_robot), description_entity));
}

/// This system updates ModelProperty<Robot> based on updates to the property components
pub fn serialize_and_change_robot_property<
    'de,
    T: Clone + Default + PartialEq + RobotProperty + Serialize,
    U: Clone + Default + PartialEq + RobotPropertyKind + Serialize,
>(
    mut change_robot_property: EventWriter<Change<ModelProperty<Robot>>>,
    property_kind: U,
    robot: &Robot,
    description_entity: Entity,
) {
    if let Ok(new_property) = serde_json::to_value(property_kind).map(|val| T::new(U::label(), val))
    {
        if let Ok(new_property_value) = serde_json::to_value(new_property) {
            let mut new_robot = robot.clone();
            new_robot.properties.insert(T::label(), new_property_value);
            change_robot_property.send(Change::new(ModelProperty(new_robot), description_entity));
        }
    }
}
