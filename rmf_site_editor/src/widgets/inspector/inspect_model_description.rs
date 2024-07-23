/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
    site::{AssetSource, Change, DefaultFile, ModelProperty, Pending},
    widgets::{prelude::*, Inspect},
    CurrentWorkspace, InspectAssetSourceComponent, InspectScaleComponent, MainInspector,
};
use rmf_site_format::{
    Affiliation, DifferentialDrive, Group, IsStatic, ModelMarker, NameInSite, RecallAssetSource,
    Scale,
};

use bevy::{
    ecs::{component::ComponentInfo, query::WorldQuery, system::SystemParam},
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, ComboBox, RichText, Ui};
use smallvec::SmallVec;
use std::{any::TypeId, collections::HashMap};

#[derive(Default)]
pub struct InspectModelDescriptionPlugin {}

impl Plugin for InspectModelDescriptionPlugin {
    fn build(&self, app: &mut App) {
        let main_inspector = app.world.resource::<MainInspector>().id;
        let widget = Widget::new::<InspectModelDescription>(&mut app.world);
        let id = app.world.spawn(widget).set_parent(main_inspector).id();
        app.world.insert_resource(ModelDescriptionInspector { id });
        app.world.init_resource::<ModelPropertyNames>();
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

#[derive(Resource)]
pub struct ModelPropertyDefault<T: Default + Clone>(pub ModelProperty<T>);

/// This resource keeps track of all the properties that can be configured for a model description.
#[derive(Resource)]
pub struct ModelPropertyNames {
    pub required: HashMap<TypeId, String>,
    pub optional: HashMap<TypeId, String>,
}

impl FromWorld for ModelPropertyNames {
    fn from_world(_world: &mut World) -> Self {
        let mut required = HashMap::new();
        required.insert(
            TypeId::of::<ModelProperty<AssetSource>>(),
            "Asset Source".to_string(),
        );
        required.insert(TypeId::of::<ModelProperty<Scale>>(), "Scale".to_string());
        required.insert(
            TypeId::of::<ModelProperty<IsStatic>>(),
            "Is Static".to_string(),
        );

        let mut optional = HashMap::new();
        optional.insert(
            TypeId::of::<ModelProperty<DifferentialDrive>>(),
            "Differential Drive".to_string(),
        );
        Self { required, optional }
    }
}

pub struct InspectModelPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Default + Clone + FromReflect + TypePath,
{
    property_name: String,
    _property: ModelProperty<T>,
    _ignore: std::marker::PhantomData<W>,
}

impl<W, T> InspectModelPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Default + Clone + FromReflect + TypePath,
{
    pub fn new(property_name: String) -> Self {
        Self {
            property_name: property_name,
            _property: ModelProperty::<T>::default(),
            _ignore: Default::default(),
        }
    }
}

impl<W, T> Plugin for InspectModelPropertyPlugin<W, T>
where
    W: WidgetSystem<Inspect, ()> + 'static + Send + Sync,
    T: 'static + Send + Sync + Default + Clone + FromReflect + TypePath,
{
    fn build(&self, app: &mut App) {
        app.register_type::<ModelProperty<T>>();

        // If type has already been loaded as required type, do not allow it to be loaded as optional
        let type_id = TypeId::of::<ModelProperty<T>>();
        if !app
            .world
            .resource::<ModelPropertyNames>()
            .required
            .contains_key(&type_id)
        {
            app.world
                .resource_mut::<ModelPropertyNames>()
                .optional
                .insert(type_id, self.property_name.clone());
        }

        let inspector = app.world.resource::<ModelDescriptionInspector>().id;
        let widget = Widget::<Inspect>::new::<W>(&mut app.world);
        app.world.spawn(widget).set_parent(inspector);
    }
}

/// This is the base model description inspector widget, which allows the user to dynamically
/// configure the properties associated with a model description.
#[derive(SystemParam)]
struct InspectModelDescription<'w, 's> {
    model_instances:
        Query<'w, 's, &'static Affiliation<Entity>, (With<ModelMarker>, Without<Group>)>,
    model_descriptions: Query<'w, 's, &'static NameInSite, (With<ModelMarker>, With<Group>)>,
    model_property_names:
        Query<'w, 's, (Entity, &'static NameInSite), (With<Group>, With<ModelMarker>)>,
    model_properties: Res<'w, ModelPropertyNames>,
    inspect_model_description: Res<'w, ModelDescriptionInspector>,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelDescription<'w, 's> {
    fn show(
        Inspect {
            selection,
            inspection,
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

        let components_info: Vec<ComponentInfo> = world
            .inspect_entity(description_entity)
            .into_iter()
            .cloned()
            .collect();
        let params = state.get_mut(world);

        let mut description_property_types = Vec::<TypeId>::new();
        for component_info in components_info {
            if let Some(type_id) = component_info.type_id() {
                if params.model_properties.required.contains_key(&type_id) {
                    description_property_types.push(type_id);
                } else if params.model_properties.optional.contains_key(&type_id) {
                    description_property_types.push(type_id);
                }
            }
        }
        let mut available_property_types = Vec::<TypeId>::new();
        for (type_id, _) in params.model_properties.optional.iter() {
            if !description_property_types.contains(type_id) {
                available_property_types.push(*type_id);
            }
        }

        ui.separator();
        let description_name = params
            .model_descriptions
            .get(description_entity)
            .map(|n| n.0.clone())
            .unwrap_or("Unnamed".to_string());
        ui.label(RichText::new(format!("Model Properties of [{}]", description_name)).size(18.0));

        CollapsingHeader::new("Configure Properties")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for type_id in description_property_types {
                        let property_name = params.model_properties.required.get(&type_id).unwrap();
                        ui.add_enabled_ui(false, |ui| {
                            ui.toggle_value(&mut true, property_name);
                        });
                    }

                    for type_id in available_property_types {
                        let property_name = params.model_properties.optional.get(&type_id).unwrap();
                        if ui.toggle_value(&mut false, property_name).clicked() {
                            println!("Add property: {:?}", property_name);
                        }
                    }
                });
            });

        let children: Result<SmallVec<[_; 16]>, _> = params
            .children
            .get(params.inspect_model_description.id)
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
            let _ = world.try_show_in(child, inspect, ui);
        }
    }
}

/// When inspecting a selected instance of a model description, this widget allows the user to view
/// and change its description
#[derive(SystemParam)]
pub struct InspectSelectedModelDescription<'w, 's> {
    model_instances:
        Query<'w, 's, &'static Affiliation<Entity>, (With<ModelMarker>, Without<Group>)>,
    model_descriptions:
        Query<'w, 's, (Entity, &'static NameInSite), (With<ModelMarker>, With<Group>)>,
    change_affiliation: EventWriter<'w, Change<Affiliation<Entity>>>,
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
        let (current_description_entity, current_description_name) = self
            .model_descriptions
            .get(current_description_entity)
            .unwrap();

        let mut new_description_entity = current_description_entity.clone();
        ui.horizontal(|ui| {
            ui.label("Description");
            ComboBox::from_id_source("model_description_affiliation")
                .selected_text(current_description_name.0.as_str())
                .show_ui(ui, |ui| {
                    for (entity, name, ..) in self.model_descriptions.iter() {
                        ui.selectable_value(&mut new_description_entity, entity, name.0.as_str());
                    }
                });
        });
        if new_description_entity != current_description_entity {
            self.change_affiliation
                .send(Change::new(Affiliation(Some(new_description_entity)), id));
        }
    }
}

/// Helper function to get the corresponding description entity for a given model instance entity
/// if it exists.
fn get_selected_description_entity<'w, 's, T: WorldQuery>(
    selection: Entity,
    model_instances: &Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>),
    >,
    model_descriptions: &Query<'w, 's, T, (With<ModelMarker>, With<Group>)>,
) -> Option<Entity> {
    match model_descriptions.get(selection) {
        Ok(_) => Some(selection),
        Err(_) => match model_instances
            .get(selection)
            .ok()
            .and_then(|affiliation| affiliation.0)
        {
            Some(affiliation) => {
                if model_descriptions.get(affiliation).is_ok() {
                    Some(affiliation)
                } else {
                    warn!("Model instance is affiliated with a non-existent description");
                    None
                }
            }
            None => None,
        },
    }
}

///
/// Basic model properties inspector here for the time being
///

#[derive(SystemParam)]
pub struct InspectModelScale<'w, 's> {
    model_instances:
        Query<'w, 's, &'static Affiliation<Entity>, (With<ModelMarker>, Without<Group>)>,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Scale>, (With<ModelMarker>, With<Group>)>,
    change_scale: EventWriter<'w, Change<ModelProperty<Scale>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelScale<'w, 's> {
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

        let Ok(ModelProperty(scale)) = params.model_descriptions.get(description_entity) else {
            return;
        };
        if let Some(new_scale) = InspectScaleComponent::new(scale).show(ui) {
            params
                .change_scale
                .send(Change::new(ModelProperty(new_scale), description_entity));
        }
        ui.add_space(10.0);
    }
}

#[derive(SystemParam)]
pub struct InspectModelAssetSource<'w, 's> {
    model_instances:
        Query<'w, 's, &'static Affiliation<Entity>, (With<ModelMarker>, Without<Group>)>,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<AssetSource>, (With<ModelMarker>, With<Group>)>,
    change_asset_source: EventWriter<'w, Change<ModelProperty<AssetSource>>>,
    current_workspace: Res<'w, CurrentWorkspace>,
    default_file: Query<'w, 's, &'static DefaultFile>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelAssetSource<'w, 's> {
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

        let Ok(ModelProperty(source)) = params.model_descriptions.get(description_entity) else {
            return;
        };

        let default_file = params
            .current_workspace
            .root
            .map(|e| params.default_file.get(e).ok())
            .flatten();

        if let Some(new_source) =
            InspectAssetSourceComponent::new(source, &RecallAssetSource::default(), default_file)
                .show(ui)
        {
            params
                .change_asset_source
                .send(Change::new(ModelProperty(new_source), description_entity));
        }
    }
}

#[derive(SystemParam)]
pub struct InspectModelDifferentialDrive<'w, 's> {
    model_instances:
        Query<'w, 's, &'static Affiliation<Entity>, (With<ModelMarker>, Without<Group>)>,
    model_descriptions: Query<
        'w,
        's,
        (
            &'static ModelProperty<DifferentialDrive>,
            &'static RecallAssetSource,
        ),
        (With<ModelMarker>, With<Group>),
    >,
    change_asset_source: EventWriter<'w, Change<ModelProperty<AssetSource>>>,
    current_workspace: Res<'w, CurrentWorkspace>,
    default_file: Query<'w, 's, &'static DefaultFile>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelDifferentialDrive<'w, 's> {
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
            ui.label("Differential Drive");
        };
    }
}
