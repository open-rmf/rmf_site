/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use bevy::{
    ecs::{
        component::{ComponentId, ComponentInfo},
        hierarchy::ChildOf,
        query::QueryData,
        system::{EntityCommands, SystemParam},
    },
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, ComboBox, RichText, Ui};
use rmf_site_egui::*;
use smallvec::SmallVec;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    site::{
        update_model_instances, Affiliation, AssetSource, Change, Group, IsStatic, ModelLoader,
        ModelMarker, ModelProperty, ModelPropertyQuery, NameInSite, Scale,
    },
    widgets::{prelude::*, Inspect},
    MainInspector,
};

pub mod inspect_collision;
pub use inspect_collision::*;

pub mod inspect_mobility;
pub use inspect_mobility::*;

pub mod inspect_power_source;
pub use inspect_power_source::*;

pub mod inspect_power_dissipation;
pub use inspect_power_dissipation::*;

pub mod inspect_required_properties;
pub use inspect_required_properties::*;

pub mod inspect_robot_properties;
pub use inspect_robot_properties::*;

#[derive(Default)]
pub struct InspectModelDescriptionPlugin {}

impl Plugin for InspectModelDescriptionPlugin {
    fn build(&self, app: &mut App) {
        let main_inspector = app.world().resource::<MainInspector>().id;
        let widget = Widget::new::<InspectModelDescription>(app.world_mut());
        let id = app
            .world_mut()
            .spawn(widget)
            .insert(ChildOf(main_inspector))
            .id();
        app.world_mut()
            .insert_resource(ModelDescriptionInspector { id });
        app.world_mut().init_resource::<ModelPropertyData>();
    }
}

/// Contains a reference to the model description inspector widget.
#[derive(Resource)]
pub struct ModelDescriptionInspector {
    id: Entity,
}

impl ModelDescriptionInspector {
    pub fn get(&self) -> Entity {
        self.id
    }
}

/// Function that inserts a default property into an entity
type InsertModelPropertyFn = fn(EntityCommands);

fn get_insert_model_property_fn<T: Component + Default>() -> InsertModelPropertyFn {
    |mut e_commands| {
        e_commands.insert(T::default());
    }
}

/// Function that removes a property, if it exists, from an entity
type RemoveModelPropertyFn = fn(EntityCommands);

fn get_remove_model_property_fn<T: Component + Default>() -> RemoveModelPropertyFn {
    |mut e_commands| {
        e_commands.remove::<T>();
    }
}

/// This resource keeps track of all the properties that can be configured for a model description.
#[derive(Resource)]
pub struct ModelPropertyData {
    pub required: HashMap<ComponentId, (String, InsertModelPropertyFn, RemoveModelPropertyFn)>,
    pub optional: HashMap<ComponentId, (String, InsertModelPropertyFn, RemoveModelPropertyFn)>,
}

impl FromWorld for ModelPropertyData {
    fn from_world(world: &mut World) -> Self {
        let mut required = HashMap::new();
        world.register_component::<ModelProperty<AssetSource>>();
        required.insert(
            world
                .components()
                .component_id::<ModelProperty<AssetSource>>()
                .unwrap(),
            (
                "Asset Source".to_string(),
                get_insert_model_property_fn::<ModelProperty<AssetSource>>(),
                get_remove_model_property_fn::<ModelProperty<AssetSource>>(),
            ),
        );
        world.register_component::<ModelProperty<Scale>>();
        required.insert(
            world
                .components()
                .component_id::<ModelProperty<Scale>>()
                .unwrap(),
            (
                "Scale".to_string(),
                get_insert_model_property_fn::<ModelProperty<Scale>>(),
                get_remove_model_property_fn::<ModelProperty<Scale>>(),
            ),
        );
        world.register_component::<ModelProperty<IsStatic>>();
        required.insert(
            world
                .components()
                .component_id::<ModelProperty<IsStatic>>()
                .unwrap(),
            (
                "Is Static".to_string(),
                get_insert_model_property_fn::<IsStatic>(),
                get_remove_model_property_fn::<IsStatic>(),
            ),
        );
        let optional = HashMap::new();
        Self { required, optional }
    }
}

/// Implement this plugin to add a new configurable property of type T to the model description inspector.
pub struct InspectModelPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Default + Clone + FromReflect + TypePath + Component,
{
    property_name: String,
    _ignore: std::marker::PhantomData<(W, ModelProperty<T>)>,
}

impl<W, T> InspectModelPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Default + Clone + FromReflect + TypePath + Component,
{
    pub fn new(property_name: String) -> Self {
        Self {
            property_name: property_name,
            _ignore: Default::default(),
        }
    }
}

impl<W, T> Plugin for InspectModelPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Debug + Default + Clone + FromReflect + TypePath + Component,
{
    fn build(&self, app: &mut App) {
        let component_id = app
            .world()
            .components()
            .component_id::<ModelProperty<T>>()
            .unwrap();
        if !app
            .world()
            .resource::<ModelPropertyData>()
            .required
            .contains_key(&component_id)
        {
            app.add_systems(PreUpdate, update_model_instances::<T>);

            app.world_mut()
                .resource_mut::<ModelPropertyData>()
                .optional
                .insert(
                    component_id,
                    (
                        self.property_name.clone(),
                        get_insert_model_property_fn::<ModelProperty<T>>(),
                        get_remove_model_property_fn::<ModelProperty<T>>(),
                    ),
                );
        }

        let inspector = app.world().resource::<ModelDescriptionInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(app.world_mut());
        app.world_mut().spawn(widget).insert(ChildOf(inspector));
    }
}

/// This is the base model description inspector widget, which allows the user to dynamically
/// configure the properties associated with a model description.
#[derive(SystemParam)]
struct InspectModelDescription<'w, 's> {
    commands: Commands<'w, 's>,
    model_instances: Query<
        'w,
        's,
        &'static Affiliation,
        (With<ModelMarker>, Without<Group>, With<NameInSite>),
    >,
    model_descriptions: Query<'w, 's, &'static NameInSite, (With<ModelMarker>, With<Group>)>,
    model_properties: Res<'w, ModelPropertyData>,
    inspect_model_description: Res<'w, ModelDescriptionInspector>,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelDescription<'w, 's> {
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
        // Get description entity from within closure, since inspect_entity requires immutable reference to world
        let description_entity = {
            let params = state.get_mut(world);
            if let Some(description_entity) = get_selected_description_entity(
                selection,
                &params.model_instances,
                &params.model_descriptions,
            ) {
                description_entity
            } else {
                return;
            }
        };

        let Ok(components_info) = world
            .inspect_entity(description_entity)
            .map(|c| c.cloned().collect::<Vec<ComponentInfo>>())
        else {
            return;
        };

        let mut inserts_to_execute = Vec::<InsertModelPropertyFn>::new();
        let mut removals_to_execute = Vec::<RemoveModelPropertyFn>::new();

        {
            let params = state.get_mut(world);

            let mut enabled_property_types = Vec::<ComponentId>::new();
            for component_info in components_info {
                let component_id = component_info.id();
                if params.model_properties.optional.contains_key(&component_id) {
                    enabled_property_types.push(component_id);
                }
            }
            let mut disabled_property_types = Vec::<ComponentId>::new();
            for (component_id, _) in params.model_properties.optional.iter() {
                if !enabled_property_types.contains(component_id) {
                    disabled_property_types.push(component_id.clone());
                }
            }

            ui.separator();
            let description_name = params
                .model_descriptions
                .get(description_entity)
                .map(|n| n.0.clone())
                .unwrap_or("Unnamed".to_string());
            ui.label(
                RichText::new(format!("Description Properties of [{}]", description_name))
                    .size(18.0),
            );

            CollapsingHeader::new("Configure Properties")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        // Required
                        for (property_name, _, _) in params.model_properties.required.values() {
                            ui.add_enabled_ui(false, |ui| {
                                ui.selectable_label(true, property_name)
                                    .on_disabled_hover_text(
                                        "This property is required and cannot be toggled",
                                    );
                            });
                        }
                        // Enabled
                        for component_id in enabled_property_types {
                            let (property_name, _, remove_fn) =
                                params.model_properties.optional.get(&component_id).unwrap();
                            if ui
                                .selectable_label(true, property_name)
                                .on_hover_text("Click to toggle")
                                .clicked()
                            {
                                removals_to_execute.push(*remove_fn);
                            }
                        }
                        // Disabled
                        for component_id in disabled_property_types {
                            let (property_name, insert_fn, _) =
                                params.model_properties.optional.get(&component_id).unwrap();
                            if ui
                                .selectable_label(false, property_name)
                                .on_hover_text("Click to toggle")
                                .clicked()
                            {
                                inserts_to_execute.push(insert_fn.clone());
                            }
                        }
                    });
                });

            let children: Result<SmallVec<[_; 16]>, _> = params
                .children
                .get(params.inspect_model_description.id)
                .map(|children| children.iter().collect());
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

        for insert_fn in inserts_to_execute {
            insert_fn(state.get_mut(world).commands.entity(description_entity));
        }
        for remove_fn in removals_to_execute {
            remove_fn(state.get_mut(world).commands.entity(description_entity));
        }
    }
}

/// When inspecting a selected instance of a model description, this widget allows the user to view
/// and change its description
#[derive(SystemParam)]
pub struct InspectSelectedModelDescription<'w, 's> {
    model_instances: ModelPropertyQuery<'w, 's, NameInSite>,
    model_descriptions: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            &'static ModelProperty<AssetSource>,
        ),
        (With<ModelMarker>, With<Group>),
    >,
    model_loader: ModelLoader<'w, 's>,
    change_affiliation: EventWriter<'w, Change<Affiliation>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectSelectedModelDescription<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(selection, ui);
    }
}

impl<'w, 's> InspectSelectedModelDescription<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Some(current_description_entity) =
            get_selected_description_entity(id, &self.model_instances, &self.model_descriptions)
        else {
            return;
        };
        let (current_description_entity, current_description_name, _) = self
            .model_descriptions
            .get(current_description_entity)
            .unwrap();

        if !self.model_instances.get(id).is_ok() {
            return;
        }

        let mut new_description_entity = current_description_entity.clone();
        ui.horizontal(|ui| {
            ui.label("Description");
            ComboBox::from_id_salt("model_description_affiliation")
                .selected_text(current_description_name.0.as_str())
                .show_ui(ui, |ui| {
                    for (entity, name, ..) in self.model_descriptions.iter() {
                        ui.selectable_value(&mut new_description_entity, entity, name.0.as_str());
                    }
                });
        });
        if new_description_entity != current_description_entity {
            self.change_affiliation
                .write(Change::new(Affiliation(Some(new_description_entity)), id));
            let (_, _, new_source) = self.model_descriptions.get(new_description_entity).unwrap();
            self.model_loader
                .update_asset_source(id, new_source.0.clone());
        }
    }
}

/// Helper function to get the corresponding description entity for a given model instance entity
/// if it exists.
fn get_selected_description_entity<'w, 's, P: Component, T: QueryData>(
    selection: Entity,
    model_instances: &ModelPropertyQuery<'w, 's, P>,
    model_descriptions: &Query<'w, 's, T, (With<ModelMarker>, With<Group>)>,
) -> Option<Entity> {
    if model_descriptions.get(selection).ok().is_some() {
        return Some(selection);
    }

    if let Some(affiliation) = model_instances.get(selection).ok() {
        let Some(affiliation) = affiliation.0 else {
            warn!("Model instance is not affiliated with a description");
            return None;
        };

        if model_descriptions.get(*affiliation).is_ok() {
            return Some(*affiliation);
        } else {
            return None;
        }
    }

    return None;
}
