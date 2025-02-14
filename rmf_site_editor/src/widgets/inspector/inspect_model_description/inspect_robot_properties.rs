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

use super::{get_selected_description_entity, ModelDescriptionInspector, ModelPropertyQuery};
use crate::{
    site::{
        update_model_instances, Change, ChangePlugin, Group, IssueKey, ModelMarker, ModelProperty,
        NameInSite, Recall, RecallPlugin, Robot, SiteUpdateSet,
    },
    widgets::{prelude::*, Inspect},
    AppState, Issue, ModelPropertyData, ValidateWorkspace,
};
use bevy::{
    ecs::system::SystemParam,
    prelude::{Component, *},
    utils::Uuid,
};
use bevy_egui::egui::{ComboBox, Ui};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Error, Map, Value};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt::Debug;

type InsertDefaultValueFn = fn() -> Result<Value, Error>;

pub struct RobotPropertyWidgetRegistration {
    pub property_widget: Entity,
    pub kinds: HashMap<String, RobotPropertyKindWidgetRegistration>,
}

pub struct RobotPropertyKindWidgetRegistration {
    pub default: InsertDefaultValueFn,
}

/// This resource keeps track of all the properties that can be configured for a robot.
#[derive(Resource, Deref, DerefMut)]
pub struct RobotPropertyWidgetRegistry(pub HashMap<String, RobotPropertyWidgetRegistration>);

impl FromWorld for RobotPropertyWidgetRegistry {
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
        app.world.init_resource::<RobotPropertyWidgetRegistry>();
        app.add_event::<UpdateRobotPropertyKinds>().add_systems(
            PreUpdate,
            check_for_missing_robot_property_kinds
                .after(SiteUpdateSet::ProcessChangesFlush)
                .run_if(AppState::in_displaying_mode()),
        );
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
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
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

pub trait RobotProperty:
    'static + Send + Sync + Default + Clone + Component + PartialEq + Serialize + DeserializeOwned
{
    fn new(kind: String, config: serde_json::Value) -> Self;

    fn is_default(&self) -> bool;

    fn kind(&self) -> Option<String>;

    fn label() -> String;
}

pub trait RobotPropertyKind:
    'static + Send + Sync + Default + Clone + Component + PartialEq + Serialize + DeserializeOwned
{
    fn label() -> String;
}

/// Implement this plugin to add a new configurable robot property of type T to the
/// robot properties inspector.
pub struct InspectRobotPropertyPlugin<W, Property, RecallProperty>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Property: RobotProperty,
    RecallProperty: Recall + Component + Default,
    RecallProperty::Source: RobotProperty,
{
    _ignore: std::marker::PhantomData<(W, Property, RecallProperty)>,
}

impl<W, Property, RecallProperty> InspectRobotPropertyPlugin<W, Property, RecallProperty>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Property: RobotProperty,
    RecallProperty: Recall + Component + Default,
    RecallProperty::Source: RobotProperty,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W, Property, RecallProperty> Plugin for InspectRobotPropertyPlugin<W, Property, RecallProperty>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Property: RobotProperty,
    RecallProperty: Recall + Component + Default,
    RecallProperty::Source: RobotProperty,
{
    fn build(&self, app: &mut App) {
        let inspector = app.world.resource::<RobotPropertiesInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        let id = app.world.spawn(widget).set_parent(inspector).id();
        app.world
            .resource_mut::<RobotPropertyWidgetRegistry>()
            .0
            .insert(
                Property::label(),
                RobotPropertyWidgetRegistration {
                    property_widget: id,
                    kinds: HashMap::new(),
                },
            );
        app.add_systems(PreUpdate, update_robot_property_components::<Property>)
            .add_plugins(RecallPlugin::<RecallProperty>::default());
    }
}

/// Implement this plugin to add a new configurable robot property kind of type T to the
/// robot properties inspector.
pub struct InspectRobotPropertyKindPlugin<W, Kind, Property>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Kind: RobotPropertyKind,
    Property: RobotProperty,
{
    _ignore: std::marker::PhantomData<(W, Kind, Property)>,
}

impl<W, Kind, Property> InspectRobotPropertyKindPlugin<W, Kind, Property>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Kind: RobotPropertyKind,
    Property: RobotProperty,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W, Kind, Property> Plugin for InspectRobotPropertyKindPlugin<W, Kind, Property>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Kind: RobotPropertyKind,
    Property: RobotProperty,
{
    fn build(&self, app: &mut App) {
        let property_label = Property::label();
        let Some(inspector) = app
            .world
            .resource::<RobotPropertyWidgetRegistry>()
            .0
            .get(&property_label)
            .map(|registration| registration.property_widget.clone())
        else {
            return;
        };
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        app.world.spawn(widget).set_parent(inspector);
        app.world
            .resource_mut::<RobotPropertyWidgetRegistry>()
            .0
            .get_mut(&property_label)
            .map(|registration| {
                registration.kinds.insert(
                    Kind::label(),
                    RobotPropertyKindWidgetRegistration {
                        default: || serde_json::to_value(Kind::default()),
                    },
                );
            });
        app.add_systems(
            PreUpdate,
            update_robot_property_kind_components::<Kind, Property>,
        );
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
pub fn update_robot_property_components<T: RobotProperty>(
    mut commands: Commands,
    model_properties: Query<(Entity, Ref<ModelProperty<Robot>>), (With<ModelMarker>, With<Group>)>,
    mut removals: RemovedComponents<ModelProperty<Robot>>,
    mut update_robot_property_kinds: EventWriter<UpdateRobotPropertyKinds>,
) {
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
            // Update robot property
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

/// This system inserts or removes robot property kind components when a robot property is updated
pub fn update_robot_property_kind_components<Kind: RobotPropertyKind, Property: RobotProperty>(
    mut commands: Commands,
    mut update_robot_property_kinds: EventReader<UpdateRobotPropertyKinds>,
) {
    for update in update_robot_property_kinds.read() {
        let property_label = Property::label();
        let property_kind_label = Kind::label();
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
                    .and_then(|config| serde_json::from_value::<Kind>(config.clone()).ok())
                {
                    Some(property_kind) => {
                        commands.entity(update.entity).insert(property_kind);
                    }
                    None => {
                        commands.entity(update.entity).insert(Kind::default());
                    }
                }
                continue;
            }
        }
        commands.entity(update.entity).remove::<Kind>();
    }
}

/// This system displays each RobotProperty widget and enables users to toggle
/// the properties on and off, and select relevant RobotPropertyKinds.
pub fn show_robot_property_widget<T: RobotProperty>(
    ui: &mut Ui,
    property_query: Query<&T, (With<ModelMarker>, With<Group>)>,
    property_recall: Option<T>,
    mut change_robot_property: EventWriter<Change<ModelProperty<Robot>>>,
    robot: &Robot,
    robot_property_widgets: &Res<RobotPropertyWidgetRegistry>,
    description_entity: Entity,
) {
    let mut new_robot = robot.clone();
    let property_label = T::label();
    let property = property_query.get(description_entity).ok();

    let Some(widget_registration) = robot_property_widgets.get(&property_label) else {
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
            None => match property_recall {
                Some(r) => r,
                None => T::default(),
            },
        };

        // Display Select Kind widget only if property kinds are provided
        if !widget_registration.kinds.is_empty() {
            let selected_property_kind = match new_property
                .kind()
                .filter(|k| widget_registration.kinds.contains_key(k))
            {
                Some(kind) => kind,
                None => "Select Kind".to_string(),
            };

            ui.indent("configure_".to_owned() + &property_label, |ui| {
                ui.horizontal(|ui| {
                    ui.label(property_label.to_owned() + " Kind");
                    ComboBox::from_id_source("select_".to_owned() + &property_label + "_kind")
                        .selected_text(selected_property_kind)
                        .show_ui(ui, |ui| {
                            for (kind, kind_registration) in widget_registration.kinds.iter() {
                                let get_default_config = kind_registration.default;
                                let config = match get_default_config() {
                                    Ok(c) => c,
                                    Err(_) => Value::Null,
                                };
                                ui.selectable_value(
                                    &mut new_property,
                                    T::new(kind.clone(), config),
                                    kind.clone(),
                                );
                            }
                        });
                });
            });
        }

        if property.is_some_and(|p| *p == new_property) {
            return;
        }
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
pub fn serialize_and_change_robot_property<Property: RobotProperty, Kind: RobotPropertyKind>(
    mut change_robot_property: EventWriter<Change<ModelProperty<Robot>>>,
    property_kind: Kind,
    robot: &Robot,
    description_entity: Entity,
) {
    if let Ok(new_property) =
        serde_json::to_value(property_kind).map(|val| Property::new(Kind::label(), val))
    {
        if let Ok(new_property_value) = serde_json::to_value(new_property) {
            let mut new_robot = robot.clone();
            new_robot
                .properties
                .insert(Property::label(), new_property_value);
            change_robot_property.send(Change::new(ModelProperty(new_robot), description_entity));
        }
    }
}

/// Unique UUID to identify issue of missing robot property kind
pub const MISSING_ROBOT_PROPERTY_KIND_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x655d6b52d8dd4f4f8324a77c497d9396u128);

pub fn check_for_missing_robot_property_kinds(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    robot_property_widgets: Res<RobotPropertyWidgetRegistry>,
    robots: Query<(Entity, &NameInSite, &ModelProperty<Robot>), (With<ModelMarker>, With<Group>)>,
) {
    for root in validate_events.read() {
        for (entity, description_name, robot) in robots.iter() {
            for (property, value) in robot.0.properties.iter() {
                let Some(widget_registration) = robot_property_widgets.get(property) else {
                    continue;
                };
                if widget_registration.kinds.is_empty() {
                    continue;
                }
                if value
                    .as_object()
                    .and_then(|m| m.get("kind"))
                    .and_then(|k| k.as_str())
                    .is_some_and(|k| widget_registration.kinds.contains_key(&k.to_string()))
                {
                    continue;
                }

                let brief = match value {
                    Value::Null => format!(
                        "RobotPropertyKind not found for RobotProperty {:?} \
                        with affiliated model description {:?}",
                        property, description_name.0
                    ),
                    _ => format!(
                        "Invalid RobotPropertyKind found for RobotProperty {:?} \
                        with affiliated model description {:?}",
                        property, description_name.0,
                    ),
                };
                let issue = Issue {
                    key: IssueKey {
                        entities: [entity].into(),
                        kind: MISSING_ROBOT_PROPERTY_KIND_ISSUE_UUID,
                    },
                    brief,
                    hint: format!(
                        "RobotProperty {} requires a RobotPropertyKind. \
                        Select a valid RobotPropertyKind.",
                        property
                    ),
                };
                let issue_id = commands.spawn(issue).id();
                commands.entity(**root).add_child(issue_id);
            }
        }
    }
}
