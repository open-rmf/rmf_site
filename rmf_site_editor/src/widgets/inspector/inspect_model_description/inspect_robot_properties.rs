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
        update_model_instances, Affiliation, ChangePlugin, Group, ModelMarker, ModelProperty,
        Robot, Tasks,
    },
    widgets::{prelude::*, Inspect},
    ModelPropertyData,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, RichText, Ui};
use rmf_site_format::RobotProperty;
use smallvec::SmallVec;
use std::collections::HashMap;

pub type ShowRobotPropertyWidgetFn = fn(&mut serde_json::Value, &mut Ui);

/// This resource keeps track of all the properties that can be configured for a robot.
#[derive(Resource)]
pub struct RobotPropertyData(pub HashMap<String, HashMap<String, ShowRobotPropertyWidgetFn>>);

impl FromWorld for RobotPropertyData {
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
            .add_systems(
                PreUpdate,
                (add_remove_robot_tasks, update_model_instances::<Robot>),
            )
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
        app.world.init_resource::<RobotPropertyData>();
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
        ui.label(RichText::new(format!("Robot Properties")).size(18.0));

        let children: Result<SmallVec<[_; 16]>, _> = params
            .children
            .get(params.inspect_robot_properties.id)
            .map(|children| children.iter().copied().collect());
        let Ok(children) = children else {
            return;
        };

        for child in children {
            let inspect = Inspect {
                selection,
                inspection: child,
                panel,
            };
            ui.add_space(10.0);
            let _ = world.try_show_in(child, inspect, ui);
        }
    }
}

// TODO(@xiyuoh) get rid of this and use checkbox to enable tasks instead?
/// When the Robot is added or removed, add or remove the Tasks component
fn add_remove_robot_tasks(
    mut commands: Commands,
    instances: Query<(Entity, Ref<Robot>), Without<Group>>,
    tasks: Query<&Tasks, (With<Robot>, Without<Group>)>,
    mut removals: RemovedComponents<ModelProperty<Robot>>,
) {
    // all instances with this description - add/remove Tasks component

    for removal in removals.read() {
        if instances.get(removal).is_ok() {
            commands.entity(removal).remove::<Tasks>();
        }
    }

    for (e, marker) in instances.iter() {
        if marker.is_added() {
            if let Ok(_) = tasks.get(e) {
                continue;
            }
            commands.entity(e).insert(Tasks::default());
        }
    }
}

/// Implement this plugin to add a new configurable robot property of type T to the
/// robot properties inspector.
pub struct InspectRobotPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Default + Clone + RobotProperty + Component,
{
    _ignore: std::marker::PhantomData<(W, T)>,
}

impl<W, T> InspectRobotPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Default + Clone + RobotProperty + Component,
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
    T: 'static + Send + Sync + Default + Clone + RobotProperty + Component,
{
    fn build(&self, app: &mut App) {
        app.world
            .resource_mut::<RobotPropertyData>()
            .0
            .insert(T::label(), HashMap::new());

        let inspector = app.world.resource::<RobotPropertiesInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        app.world.spawn(widget).set_parent(inspector);
    }
}

pub fn show_robot_property<'de, T: Component + Clone + Default + PartialEq + RobotProperty>(
    ui: &mut Ui,
    property: Option<T>,
    robot_property_data: ResMut<RobotPropertyData>,
) -> Result<Option<T>, ()> {
    let mut has_property = property.is_some();
    let property_label = T::label();

    ui.checkbox(&mut has_property, property_label.clone());
    if !has_property {
        if property.is_some() {
            return Ok(None);
        }
        return Err(());
    }

    let mut new_property = match property {
        Some(ref p) => p.clone(),
        None => T::default(),
    };
    let selected_property_kind = if !new_property.is_empty() {
        new_property.kind().clone()
    } else {
        "Select Kind".to_string()
    };

    if let Some(property_kinds) = robot_property_data.0.get(&property_label) {
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
            if !new_property.is_default() {
                if let Some(show_widget) = property_kinds.get(&new_property.kind()) {
                    show_widget(&mut new_property.config_mut(), ui);
                }
            }
        });
    } else {
        ui.label(format!("No {} kind registered.", property_label));
    }

    ui.add_space(10.0);

    if property.is_none() || property.is_some_and(|m| m != new_property && !new_property.is_empty())
    {
        // TODO(@xiyuoh) fix saving empty robot properties
        return Ok(Some(new_property));
    }
    Err(())
}
