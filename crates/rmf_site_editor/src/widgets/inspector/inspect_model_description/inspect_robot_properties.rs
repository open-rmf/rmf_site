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

use super::*;
use crate::{
    site::{
        Change, Group, IssueKey, ModelMarker, ModelProperty, ModelPropertyQuery, NameInSite, Robot,
        RobotPropertyRegistry, SiteUpdateSet,
    },
    widgets::Inspect,
    AppState, Issue, ValidateWorkspace,
};
use bevy::ecs::{hierarchy::ChildOf, system::SystemParam};
use bevy_egui::egui::{ComboBox, Ui};
use rmf_site_format::robot_properties::*;
use serde_json::Value;
use smallvec::SmallVec;
use uuid::Uuid;

#[derive(Default)]
pub struct InspectRobotPropertiesPlugin {}

impl Plugin for InspectRobotPropertiesPlugin {
    fn build(&self, app: &mut App) {
        // Allows us to toggle Robot as a configurable property
        // from the model description inspector
        let inspector = app.world().resource::<ModelDescriptionInspector>().id;
        let widget = Widget::<Inspect>::new::<InspectRobotProperties>(app.world_mut());
        let id = app
            .world_mut()
            .spawn(widget)
            .insert(ChildOf(inspector))
            .id();
        app.world_mut()
            .insert_resource(RobotPropertiesInspector { id });
        app.add_systems(
            PreUpdate,
            check_for_invalid_robot_property_kinds
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
            .map(|children| children.iter().collect());
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

/// Implement this plugin to add a widget for the target robot property to the
/// robot properties inspector.
pub struct InspectRobotPropertyPlugin<W, Property>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Property: RobotProperty,
{
    _ignore: std::marker::PhantomData<(W, Property)>,
}

impl<W, Property> InspectRobotPropertyPlugin<W, Property>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Property: RobotProperty,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W, Property> Plugin for InspectRobotPropertyPlugin<W, Property>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    Property: RobotProperty,
{
    fn build(&self, app: &mut App) {
        let inspector = app.world().resource::<RobotPropertiesInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(app.world_mut());
        let id = app
            .world_mut()
            .spawn(widget)
            .insert(ChildOf(inspector))
            .id();
        let mut registry = app.world_mut().resource_mut::<RobotPropertyRegistry>();
        if let Some(registration) = registry.0.get_mut(&Property::label()) {
            registration.widget = Some(id);
        }
    }
}

/// Implement this plugin to add a widget for the target robot property kind to the
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
        let Some(inspector) = app
            .world()
            .resource::<RobotPropertyRegistry>()
            .0
            .get(&Property::label())
            .and_then(|registration| registration.widget)
        else {
            return;
        };
        let widget = Widget::<Inspect>::new::<W>(app.world_mut());
        app.world_mut().spawn(widget).insert(ChildOf(inspector));
    }
}

/// This system displays each RobotProperty widget and enables users to toggle
/// the properties on and off, and select relevant RobotPropertyKinds.
pub fn show_robot_property_widget<T: RobotProperty>(
    ui: &mut Ui,
    commands: &mut Commands,
    property_query: Query<&T, (With<ModelMarker>, With<Group>)>,
    property_recall: Option<T>,
    robot: &Robot,
    robot_property_registry: &Res<RobotPropertyRegistry>,
    description_entity: Entity,
) {
    let mut new_robot = robot.clone();
    let property_label = T::label();
    let property = property_query.get(description_entity).ok();

    let Some(widget_registration) = robot_property_registry.get(&property_label) else {
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
                    ComboBox::from_id_salt("select_".to_owned() + &property_label + "_kind")
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
    commands.trigger(Change::new(ModelProperty(new_robot), description_entity));
}

/// Unique UUID to identify issue of invalid robot property kind
pub const INVALID_ROBOT_PROPERTY_KIND_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x655d6b52d8dd4f4f8324a77c497d9396u128);

pub fn check_for_invalid_robot_property_kinds(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    robot_property_registry: Res<RobotPropertyRegistry>,
    robots: Query<(Entity, &NameInSite, &ModelProperty<Robot>), (With<ModelMarker>, With<Group>)>,
) {
    for root in validate_events.read() {
        for (entity, description_name, robot) in robots.iter() {
            for (property, value) in robot.0.properties.iter() {
                let Some(widget_registration) = robot_property_registry.get(property) else {
                    continue;
                };
                if widget_registration.kinds.is_empty() {
                    continue;
                }
                let property_kind = value
                    .as_object()
                    .and_then(|m| m.get("kind"))
                    .and_then(|k| k.as_str());
                // Ignore if
                // - This RobotProperty simple toggles on-off
                // - This RobotProperty does not have a RobotPropertyKind
                // - This RobotPropertyKind is valid and registered
                if value.is_null()
                    || property_kind.is_none()
                    || property_kind
                        .is_some_and(|k| widget_registration.kinds.contains_key(&k.to_string()))
                {
                    continue;
                }

                let brief = format!(
                    "Invalid RobotPropertyKind {:?} found for RobotProperty {:?} \
                    with affiliated model description {:?}",
                    property_kind.unwrap(),
                    property,
                    description_name.0,
                );
                let issue = Issue {
                    key: IssueKey {
                        entities: [entity].into(),
                        kind: INVALID_ROBOT_PROPERTY_KIND_ISSUE_UUID,
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
