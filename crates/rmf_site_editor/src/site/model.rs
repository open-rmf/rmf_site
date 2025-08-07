/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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
    interaction::DragPlaneBundle,
    site::{CurrentScenario, Delete, SiteAssets, UpdateModifier},
    site_asset_io::MODEL_ENVIRONMENT_VARIABLE,
    Issue, ValidateWorkspace,
};
use bevy::{
    asset::{io::AssetReaderError, AssetLoadError},
    ecs::{
        hierarchy::ChildOf,
        relationship::DescendantIter,
        schedule::ScheduleConfigs,
        system::{EntityCommands, ScheduleSystem, SystemParam},
    },
    gltf::Gltf,
    prelude::*,
    render::view::RenderLayers,
    scene::SceneInstance,
};
use bevy_impulse::*;
use bevy_mod_outline::{GenerateOutlineNormalsSettings, OutlineMeshExt};
use rmf_site_camera::MODEL_PREVIEW_LAYER;
use rmf_site_format::{
    Affiliation, AssetSource, Group, Inclusion, IssueKey, ModelInstance, ModelMarker,
    ModelProperty, NameInSite, Pending, Scale,
};
use rmf_site_picking::Preview;
use smallvec::SmallVec;
use std::{any::TypeId, collections::HashSet, fmt, future::Future};
use thiserror::Error;
use uuid::Uuid;

/// Denotes the properties of the current spawned scene for the model, to despawn when updating AssetSource
/// and avoid spurious reloading if the new `AssetSource` is equal to the old one
#[derive(Component, Debug, Clone)]
pub struct ModelScene {
    source: AssetSource,
    scene_root: Entity,
}

/// Marks a pending model that has not completely loaded
#[derive(Component, Debug, Clone)]
pub struct PendingModel;

/// For a given `AssetSource`, return all the sources that we should try loading.
pub fn get_all_for_source(source: &AssetSource) -> Vec<AssetSource> {
    match source {
        AssetSource::Search(ref name) => {
            let split: SmallVec<[&str; 8]> = name.split('/').collect();
            let model_name = split.last().unwrap();
            let mut paths = common_model_directory_layouts(&name, model_name);

            if split.len() == 1 {
                // Check for the asset in the Open-RMF organization
                paths.extend(common_model_directory_layouts(
                    &format!("Open-RMF/{name}"),
                    &name,
                ));
            }

            paths
        }
        AssetSource::Local(_)
        | AssetSource::Remote(_)
        | AssetSource::Package(_)
        | AssetSource::Ros(_) => {
            let mut v = Vec::new();
            v.push(source.clone());
            v
        }
    }
}

fn common_model_directory_layouts(path: &str, model_name: &str) -> Vec<AssetSource> {
    vec![
        AssetSource::Search(path.to_owned()),
        AssetSource::Search(path.to_owned() + "/model.sdf"),
        AssetSource::Search(path.to_owned() + "/" + model_name + ".obj"),
        AssetSource::Search(path.to_owned() + ".glb"),
        AssetSource::Search(path.to_owned() + ".stl"),
        AssetSource::Search(path.to_owned() + "/" + model_name + ".glb"),
    ]
}

pub type ModelLoadingResult = Result<ModelLoadingSuccess, ModelLoadingError>;

pub type InstanceSpawningResult = Result<ModelLoadingSuccess, InstanceSpawningError>;

#[derive(Resource)]
/// Services that deal with model loading
// TODO(luca) revisit pub / private-ness of struct and fields
struct ModelLoadingServices {
    /// Continuous service that sends a response when the scene at the requested entity finished
    /// spawning.
    check_scene_is_spawned: Service<Entity, Entity>,
    /// System that tries to load a model and returns a result.
    pub load_model: Service<ModelLoadingRequest, ModelLoadingResult>,
    pub spawn_instance: Service<InstanceSpawningRequest, InstanceSpawningResult>,
}

#[derive(Default)]
pub struct ModelLoadingPlugin {}

impl Plugin for ModelLoadingPlugin {
    fn build(&self, app: &mut App) {
        let model_loading_services = ModelLoadingServices::from_app(app);
        app.insert_resource(model_loading_services);
    }
}

// For each InstanceId send a response when it is spawned
fn check_scenes_are_spawned(
    In(ContinuousService { key }): ContinuousServiceInput<Entity, Entity>,
    mut orders: ContinuousQuery<Entity, Entity>,
    instance_ids: Query<&SceneInstance>,
    scene_spawner: Res<SceneSpawner>,
) {
    let Some(mut orders) = orders.get_mut(&key) else {
        return;
    };

    orders.for_each(|order| {
        let req = order.request().clone();
        // Make sure the instance is ready
        if instance_ids
            .get(req)
            .is_ok_and(|id| scene_spawner.instance_is_ready(**id))
        {
            order.respond(req);
        }
    })
}

fn load_asset_source(
    In(source): In<AssetSource>,
    asset_server: Res<AssetServer>,
) -> impl Future<Output = Result<UntypedHandle, ModelLoadingErrorKind>> {
    let asset_server = asset_server.clone();
    async move {
        let asset_path = match String::try_from(&source) {
            Ok(asset_path) => asset_path,
            Err(err) => {
                return Err(ModelLoadingErrorKind::InvalidAssetSource(err.to_string()));
            }
        };
        asset_server
            .load_untyped_async(&asset_path)
            .await
            .map_err(|err| {
                if !matches!(
                    err,
                    AssetLoadError::AssetReaderError(AssetReaderError::Io(_))
                ) {
                    // AssetReaderError::Io is a common error during searches, so
                    // we skip it, but other errors may indicate that a problem
                    // exists in the asset itself.
                    error!("Failed attempt to load asset with [{asset_path}]: {err}");
                }
                ModelLoadingErrorKind::AssetServerError(err.to_string())
            })
    }
}

pub fn spawn_scene_for_loaded_model(
    In((parent, h, source)): In<(Entity, UntypedHandle, AssetSource)>,
    world: &mut World,
) -> Option<(Entity, bool)> {
    // For each model that is loading, check if its scene has finished loading
    // yet. If the scene has finished loading, then insert it as a child of the
    // model entity and make it selectable.
    let type_id = h.type_id();
    let (model_id, is_scene) = if type_id == TypeId::of::<Gltf>() {
        // Note we can't do an `if let Some()` because get(Handle) panics if the type is
        // not the stored type
        let gltfs = world.resource::<Assets<Gltf>>();
        let gltf = gltfs.get(h.typed::<Gltf>().id())?;
        // Get default scene if present, otherwise index 0
        let scene = gltf
            .default_scene
            .as_ref()
            .or_else(|| gltf.scenes.get(0))
            .cloned()?;
        Some((
            world
                .spawn((
                    SceneRoot(scene),
                    Transform::default(),
                    GlobalTransform::default(),
                ))
                .id(),
            true,
        ))
    } else if type_id == TypeId::of::<Scene>() {
        let scene = h.typed::<Scene>();
        Some((
            world
                .spawn((
                    SceneRoot(scene),
                    Transform::default(),
                    GlobalTransform::default(),
                ))
                .id(),
            true,
        ))
    } else if type_id == TypeId::of::<Mesh>() {
        let site_assets = world.resource::<SiteAssets>();
        let mesh = h.typed::<Mesh>();
        Some((
            world
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(site_assets.default_mesh_grey_material.clone()),
                    Transform::default(),
                ))
                .id(),
            false,
        ))
    } else {
        None
    }?;
    // Add scene and visibility bundle if not present already
    world
        .entity_mut(parent)
        .insert(ModelScene {
            source: source,
            scene_root: model_id,
        })
        .add_child(model_id);
    if world.get::<Visibility>(parent).is_none() {
        world.entity_mut(parent).insert(Visibility::default());
    }
    Some((model_id, is_scene))
}

/// Return Ok(request) if the source changed and we might need to continue downstream operations.
/// Err(Ok(success)) if there was no change and we can skip downstream operations.
pub fn cleanup_if_asset_source_changed(
    In(request): In<ModelLoadingRequest>,
    mut commands: Commands,
    model_scenes: Query<&ModelScene>,
    scene_roots: Query<(&SceneRoot, Option<&SceneInstance>)>,
    mut scene_spawner: ResMut<SceneSpawner>,
    mut delete: EventWriter<Delete>,
) -> Result<ModelLoadingRequest, ModelLoadingResult> {
    commands
        .entity(request.parent)
        .insert(request.source.clone());
    let Ok(scene) = model_scenes.get(request.parent) else {
        return Ok(request);
    };

    if scene.source == request.source {
        return Err(Ok(ModelLoadingSuccess {
            request,
            unchanged: true,
        }));
    }
    if let Ok((_, scene_instance)) = scene_roots.get(scene.scene_root) {
        if let Some(old_instance) = scene_instance {
            scene_spawner.despawn_instance(**old_instance);
        }
    }
    delete.write(Delete::new(scene.scene_root));
    commands.entity(request.parent).remove::<ModelScene>();
    Ok(request)
}

fn handle_model_loading(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<ModelLoadingRequest>,
    model_services: Res<ModelLoadingServices>,
) -> impl Future<Output = Result<ModelLoadingRequest, ModelLoadingError>> {
    let check_scene_is_spawned = model_services.check_scene_is_spawned.clone();
    async move {
        let sources = get_all_for_source(&request.source);

        let load_asset_source = load_asset_source.into_async_callback();
        let mut handle = None;
        for source in sources {
            let res = channel
                .query(source, load_asset_source.clone())
                .await
                .available()
                .ok_or_else(|| {
                    ModelLoadingError::new(
                        request.clone(),
                        ModelLoadingErrorKind::WorkflowExecutionError,
                    )
                })?;
            if let Ok(h) = res {
                handle = Some(h);
                break;
            }
        }
        let Some(handle) = handle else {
            return Err(ModelLoadingError::new(
                request.clone(),
                ModelLoadingErrorKind::FailedLoadingAsset,
            ));
        };
        // Now we have a handle and a parent entity, call the spawn scene service
        let res = channel
            .query(
                (request.parent, handle, request.source.clone()),
                spawn_scene_for_loaded_model.into_blocking_callback(),
            )
            .await
            .available()
            .ok_or_else(|| {
                ModelLoadingError::new(
                    request.clone(),
                    ModelLoadingErrorKind::WorkflowExecutionError,
                )
            })?;
        let Some((scene_entity, is_scene)) = res else {
            return Err(ModelLoadingError::new(
                request.clone(),
                ModelLoadingErrorKind::NonModelAsset,
            ));
        };
        if is_scene {
            // Wait for the scene to be spawned, if there is one
            channel
                .query(scene_entity, check_scene_is_spawned)
                .await
                .available()
                .ok_or_else(|| {
                    ModelLoadingError::new(
                        request.clone(),
                        ModelLoadingErrorKind::WorkflowExecutionError,
                    )
                })?;
        }
        Ok(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, DeliveryLabel)]
struct SpawnModelLabel(Entity);

/// Component added to models that are being loaded
#[derive(Component, Deref, DerefMut)]
pub struct ModelLoadingState(Promise<ModelLoadingResult>);

/// Component added to models that failed loading and containing the reason loading failed.
#[derive(Component, Deref, DerefMut)]
pub struct ModelFailedLoading(ModelLoadingError);

/// Polling system that checks the state of promises and prints errors / adds marker components if
/// models failed loading
fn handle_model_loading_errors(
    In(result): In<ModelLoadingResult>,
    model_scenes: Query<&ModelScene>,
    mut commands: Commands,
    scene_roots: Query<(&SceneRoot, Option<&SceneInstance>)>,
    mut scene_spawner: ResMut<SceneSpawner>,
    mut delete: EventWriter<Delete>,
) -> ModelLoadingResult {
    let parent = match result {
        Ok(ref success) => success.request.parent,
        Err(ref err) => {
            let parent = err.request.parent;
            // There was an actual error, cleanup the scene
            if let Ok(scene) = model_scenes.get(parent) {
                if let Ok((_, scene_instance)) = scene_roots.get(scene.scene_root) {
                    if let Some(old_instance) = scene_instance {
                        scene_spawner.despawn_instance(**old_instance);
                    }
                }
                delete.write(Delete::new(scene.scene_root));
                commands.entity(parent).remove::<ModelScene>();
            }
            error!("{err}");
            if let Ok(mut entity_mut) = commands.get_entity(parent) {
                // The parent entity might not exist any longer after the loading
                // failed, so we check for its existence before inserting to it.
                entity_mut.insert(ModelFailedLoading(err.clone()));
            }

            parent
        }
    };

    if let Ok(mut entity_mut) = commands.get_entity(parent) {
        // The parent entity might not exist any longer after the loading failed,
        // so we check for its existence before removing from it.
        entity_mut.remove::<ModelLoadingState>();
    }
    result
}

fn instance_spawn_request_into_model_load_request(
    In(request): In<InstanceSpawningRequest>,
    descriptions: Query<&ModelProperty<AssetSource>>,
) -> Result<ModelLoadingRequest, InstanceSpawningError> {
    let Some(affiliation) = request.affiliation.0 else {
        return Err(InstanceSpawningError::NoAffiliation);
    };

    let Ok(source) = descriptions.get(affiliation) else {
        return Err(InstanceSpawningError::AffiliationMissing);
    };

    Ok(ModelLoadingRequest {
        parent: request.parent,
        source: source.0.clone(),
        interaction: Some(DragPlaneBundle::new(request.parent, Vec3::Z)),
    })
}

/// `SystemParam` used to request for model loading operations
#[derive(SystemParam)]
pub struct ModelLoader<'w, 's> {
    services: Res<'w, ModelLoadingServices>,
    commands: Commands<'w, 's>,
    model_instances: Query<
        'w,
        's,
        (Entity, &'static Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>, With<AssetSource>),
    >,
}

impl<'w, 's> ModelLoader<'w, 's> {
    /// Spawn a new model instance and begin a workflow to load its asset source
    /// from the affiliated model description.
    /// This is only for brand new models does not support reacting to the load finishing.
    pub fn spawn_model_instance(
        &mut self,
        parent: Entity,
        instance: ModelInstance<Entity>,
    ) -> EntityCommands<'_> {
        self.spawn_model_instance_impulse(parent, instance, move |impulse| {
            impulse.detach();
        })
    }

    /// Spawn a new model instance and begin a workflow to load its asset source.
    /// Additionally build on the impulse chain of the asset source loading workflow.
    pub fn spawn_model_instance_impulse(
        &mut self,
        parent: Entity,
        instance: ModelInstance<Entity>,
        impulse: impl FnOnce(Impulse<InstanceSpawningResult, ()>),
    ) -> EntityCommands<'_> {
        let affiliation = instance.description.clone();
        let id = self
            .commands
            .spawn(instance)
            .insert(ChildOf(parent))
            .insert(PendingModel) // Set instance as pending until it completes loading
            .insert(Visibility::Hidden) // Set instance to hidden until it completes loading
            .insert(Inclusion::Hidden)
            .id();
        let spawning_impulse = self.commands.request(
            InstanceSpawningRequest::new(id, affiliation),
            self.services
                .spawn_instance
                .clone()
                .instruct(SpawnModelLabel(id).preempt()),
        );
        (impulse)(spawning_impulse);
        self.commands.entity(id)
    }

    /// Run a basic workflow to update the asset source of an existing entity
    pub fn update_asset_source(&mut self, entity: Entity, source: AssetSource) {
        let interaction = DragPlaneBundle::new(entity, Vec3::Z);
        self.update_asset_source_impulse(entity, source, Some(interaction))
            .detach();
    }

    /// Update an asset source and then keep attaching impulses to its outcome.
    /// Remember to call `.detach()` when finished or else the whole chain will be
    /// dropped right away.
    pub fn update_asset_source_impulse(
        &mut self,
        entity: Entity,
        source: AssetSource,
        interaction: Option<DragPlaneBundle>,
    ) -> Impulse<'w, 's, '_, ModelLoadingResult, ()> {
        self.commands.request(
            ModelLoadingRequest::new(entity, source, interaction),
            self.services
                .load_model
                .clone()
                .instruct(SpawnModelLabel(entity).preempt()),
        )
    }

    /// Update the asset source of all model instances affiliated with the provided
    /// model description
    pub fn update_description_asset_source(&mut self, entity: Entity, source: AssetSource) {
        let mut instance_entities = HashSet::new();
        for (e, affiliation) in self.model_instances.iter() {
            if let Some(description_entity) = affiliation.0 {
                if entity == description_entity {
                    instance_entities.insert(e);
                }
            }
        }
        let interaction = DragPlaneBundle::new(entity, Vec3::Z);
        for e in instance_entities.iter() {
            self.update_asset_source_impulse(*e, source.clone(), Some(interaction.clone()))
                .detach();
        }
    }
}

fn load_model_dependencies(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<ModelLoadingRequest>,
    children_q: Query<&Children>,
    models: Query<&AssetSource, With<ModelMarker>>,
    model_loading: Res<ModelLoadingServices>,
) -> impl Future<Output = Result<ModelLoadingRequest, ModelLoadingError>> {
    let models = DescendantIter::new(&children_q, request.parent)
        .filter_map(|c| models.get(c).ok().map(|source| (c, source.clone())))
        .collect::<Vec<_>>();
    let load_model = model_loading.load_model.clone();
    async move {
        for (model_entity, source) in models {
            channel
                .query(
                    ModelLoadingRequest::new(model_entity, source, request.interaction.clone()),
                    load_model,
                )
                .await
                .available()
                .ok_or_else(|| {
                    ModelLoadingError::new(
                        request.clone(),
                        ModelLoadingErrorKind::WorkflowExecutionError,
                    )
                })?
                .map_err(|err| {
                    ModelLoadingError::new(
                        request.clone(),
                        ModelLoadingErrorKind::FailedLoadingDependency(err.to_string()),
                    )
                })?;
        }
        Ok(request)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ModelLoadingSet {
    /// Label to run the system that checks if scenes have been spawned
    CheckSceneSystem,
    /// Flush commands and impulses
    CheckSceneFlush,
}

impl ModelLoadingServices {
    pub fn from_app(app: &mut App) -> Self {
        app.configure_sets(
            PostUpdate,
            (
                ModelLoadingSet::CheckSceneSystem,
                ModelLoadingSet::CheckSceneFlush,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            (ApplyDeferred, flush_impulses()).in_set(ModelLoadingSet::CheckSceneFlush),
        );
        let check_scene_is_spawned = app.spawn_continuous_service(
            PostUpdate,
            check_scenes_are_spawned.configure(|config: ScheduleConfigs<ScheduleSystem>| {
                config.in_set(ModelLoadingSet::CheckSceneSystem)
            }),
        );
        let skip_if_unchanged = cleanup_if_asset_source_changed.into_blocking_callback();
        let load_model_dependencies = app.world_mut().spawn_service(load_model_dependencies);
        let model_loading_service = app.world_mut().spawn_service(handle_model_loading);
        // This workflow tries to load the model without doing any error handling
        let try_load_model: Service<ModelLoadingRequest, ModelLoadingResult, ()> =
            app.world_mut().spawn_workflow(|scope, builder| {
                scope
                    .input
                    .chain(builder)
                    .then(skip_if_unchanged)
                    .branch_for_err(|res| res.connect(scope.terminate))
                    .then(model_loading_service)
                    .connect_on_err(scope.terminate)
                    .then(load_model_dependencies)
                    .connect_on_err(scope.terminate)
                    // The model and its dependencies are spawned, make them selectable / propagate
                    // render layers
                    .then(propagate_model_properties.into_blocking_callback())
                    .then(make_models_selectable.into_blocking_callback())
                    .then(make_models_visible.into_blocking_callback())
                    .map_block(|req| {
                        Ok(ModelLoadingSuccess {
                            request: req,
                            unchanged: false,
                        })
                    })
                    .connect(scope.terminate)
            });

        // Complete model loading with error handling, by having it as a separate
        // workflow we can easily capture all the early returns on error
        let load_model = app.world_mut().spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(try_load_model)
                .then(handle_model_loading_errors.into_blocking_callback())
                .connect(scope.terminate)
        });

        // Model instance spawning workflow
        let spawn_instance = app.world_mut().spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(instance_spawn_request_into_model_load_request.into_blocking_callback())
                .connect_on_err(scope.terminate)
                .then(load_model)
                .map_block(|res| res.map_err(InstanceSpawningError::ModelError))
                .connect(scope.terminate)
        });

        Self {
            load_model,
            check_scene_is_spawned,
            spawn_instance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelLoadingRequest {
    /// The entity to spawn the model for
    pub parent: Entity,
    /// AssetSource pointing to which asset we want to load
    pub source: AssetSource,
    /// Indicates if and which entity should be made selectable
    pub interaction: Option<DragPlaneBundle>,
}

impl ModelLoadingRequest {
    pub fn new(parent: Entity, source: AssetSource, interaction: Option<DragPlaneBundle>) -> Self {
        Self {
            parent,
            source,
            interaction,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelLoadingSuccess {
    pub request: ModelLoadingRequest,
    pub unchanged: bool,
}

#[derive(Clone, Debug)]
pub struct ModelLoadingError {
    pub request: ModelLoadingRequest,
    pub kind: ModelLoadingErrorKind,
}

impl ModelLoadingError {
    pub fn new(request: ModelLoadingRequest, kind: ModelLoadingErrorKind) -> Self {
        Self { request, kind }
    }
}

impl fmt::Display for ModelLoadingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Failed to execute model loading request for entity {0:?} and source {1:?} ",
            self.request.parent, self.request.source
        )?;
        write!(f, "Reason: {0}", self.kind)
    }
}

#[derive(Clone, Debug, Error)]
pub enum ModelLoadingErrorKind {
    #[error("Error executing the model loading workflow")]
    WorkflowExecutionError,
    #[error("Asset server error: {0}")]
    AssetServerError(String),
    #[error(
        "Invalid syntax while creating asset path for model. \
        Check that your asset information was input correctly. \
        Current value:\n{0:?}"
    )]
    InvalidAssetSource(String),
    #[error(
        "Failed loading asset, make sure it is in a supported format (.dae is not supported),\
        try using an API key if it belongs to a private organization \
        or add its path to the {MODEL_ENVIRONMENT_VARIABLE} environment variable."
    )]
    FailedLoadingAsset,
    #[error("Asset did not contain a model")]
    NonModelAsset,
    #[error("Failed loading dependency for model, error: {0}")]
    FailedLoadingDependency(String),
}

#[derive(Clone, Debug)]
pub struct InstanceSpawningRequest {
    pub parent: Entity,
    pub affiliation: Affiliation<Entity>,
}

impl InstanceSpawningRequest {
    pub fn new(parent: Entity, affiliation: Affiliation<Entity>) -> Self {
        Self {
            parent,
            affiliation,
        }
    }
}

#[derive(Clone, Debug)]
pub enum InstanceSpawningError {
    NoAffiliation,
    AffiliationMissing,
    ModelError(ModelLoadingError),
}

pub fn update_model_scales(
    changed_scales: Query<(&Scale, &ModelScene), Or<(Changed<Scale>, Changed<ModelScene>)>>,
    mut transforms: Query<&mut Transform>,
) {
    for (scale, scene) in changed_scales.iter() {
        if let Ok(mut tf) = transforms.get_mut(scene.scene_root) {
            tf.scale = **scale;
        }
    }
}

pub fn make_models_selectable(
    In(req): In<ModelLoadingRequest>,
    mut commands: Commands,
    pending_or_previews: Query<(), Or<(With<Pending>, With<Preview>)>>,
    scene_roots: Query<&RenderLayers, With<ModelMarker>>,
    all_children: Query<&Children>,
    mesh_handles: Query<&Mesh3d>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) -> ModelLoadingRequest {
    // Pending items (i.e. mouse previews) should not be selectable
    if pending_or_previews.get(req.parent).is_ok() {
        return req;
    }
    // Use a small vec here to try to dodge heap allocation if possible.
    // TODO(MXG): Run some tests to see if an allocation of 16 is typically
    // sufficient.
    let mut queue: SmallVec<[Entity; 16]> = SmallVec::new();
    // If layer should not be visible, don't make it selectable
    if scene_roots
        .get(req.parent)
        .is_ok_and(|r| r.iter().all(|l| l == MODEL_PREVIEW_LAYER))
    {
        return req;
    }
    let Some(interaction) = req.interaction.clone() else {
        return req;
    };
    queue.push(req.parent);

    while let Some(e) = queue.pop() {
        commands.entity(e).insert(interaction.clone());

        if let Ok(mesh_handle) = mesh_handles.get(e) {
            if let Some(mesh) = mesh_assets.get_mut(mesh_handle) {
                if mesh
                    .generate_outline_normals(&GenerateOutlineNormalsSettings::default())
                    .is_err()
                {
                    warn!(
                        "WARNING: Unable to generate outline normals for \
                        a model mesh"
                    );
                }
            }
        }

        if let Ok(children) = all_children.get(e) {
            for child in children {
                queue.push(*child);
            }
        }
    }
    req
}

pub fn make_models_visible(
    In(req): In<ModelLoadingRequest>,
    mut commands: Commands,
    current_scenario: Res<CurrentScenario>,
) -> ModelLoadingRequest {
    if let Some(current_scenario_entity) = current_scenario.0 {
        commands.entity(req.parent).remove::<PendingModel>();
        commands.trigger(UpdateModifier::modify(
            current_scenario_entity,
            req.parent,
            Inclusion::Included,
        ));
    }
    req
}

/// Assigns the render layer of the root, if present, to all the children
pub fn propagate_model_properties(
    In(req): In<ModelLoadingRequest>,
    mut commands: Commands,
    render_layers: Query<&RenderLayers>,
    previews: Query<&Preview>,
    pendings: Query<&Pending>,
    mesh_entities: Query<(), With<Mesh3d>>,
    children: Query<&Children>,
) -> ModelLoadingRequest {
    propagate_model_property(
        req.parent,
        &render_layers,
        &children,
        &mesh_entities,
        &mut commands,
    );
    propagate_model_property(
        req.parent,
        &previews,
        &children,
        &mesh_entities,
        &mut commands,
    );
    propagate_model_property(
        req.parent,
        &pendings,
        &children,
        &mesh_entities,
        &mut commands,
    );
    req
}

pub fn propagate_model_property<Property: Component + Clone + std::fmt::Debug>(
    root: Entity,
    property_query: &Query<&Property>,
    children: &Query<&Children>,
    mesh_entities: &Query<(), With<Mesh3d>>,
    commands: &mut Commands,
) {
    let Ok(property) = property_query.get(root) else {
        return;
    };

    for c in DescendantIter::new(children, root) {
        if mesh_entities.contains(c) {
            commands.entity(c).insert(property.clone());
        }
    }
}

/// This system keeps model instances up to date with the properties of their affiliated descriptions
pub fn update_model_instances<T: Component + Default + Clone>(
    mut commands: Commands,
    model_properties: Query<Ref<ModelProperty<T>>, (With<ModelMarker>, With<Group>)>,
    model_instances: Query<(Entity, Ref<Affiliation<Entity>>), (With<ModelMarker>, Without<Group>)>,
    mut removals: RemovedComponents<ModelProperty<T>>,
) {
    // Removals
    if !removals.is_empty() {
        for description_entity in removals.read() {
            for (instance_entity, affiliation) in model_instances.iter() {
                if affiliation.0 == Some(description_entity) {
                    commands.entity(instance_entity).remove::<T>();
                }
            }
        }
    }

    // Changes
    for (instance_entity, affiliation) in model_instances.iter() {
        if let Some(description_entity) = affiliation.0 {
            if let Ok(property) = model_properties.get(description_entity) {
                if property.is_changed() || affiliation.is_changed() {
                    let mut cmd = commands.entity(instance_entity);
                    cmd.insert(property.0.clone());
                }
            }
        }
    }
}

pub type ModelPropertyQuery<'w, 's, P> =
    Query<'w, 's, &'static Affiliation<Entity>, (With<ModelMarker>, Without<Group>, With<P>)>;

/// Unique UUID to identify issue of orphan model instance
pub const ORPHAN_MODEL_INSTANCE_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x4e98ce0bc28e4fe528cb0a028f4d5c08u128);

pub fn check_for_orphan_model_instances(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    mut orphan_instances: Query<
        (Entity, &NameInSite, &Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>, Without<ChildOf>),
    >,
    model_descriptions: Query<&NameInSite, (With<ModelMarker>, With<Group>)>,
) {
    for root in validate_events.read() {
        for (instance_entity, instance_name, affiliation) in orphan_instances.iter_mut() {
            let brief = match affiliation
                .0
                .map(|e| model_descriptions.get(e).ok())
                .flatten()
            {
                Some(description_name) => format!(
                    "Parent level entity not found for model instance {:?} with \
                    affiliated model description {:?}",
                    instance_name, description_name
                ),
                None => format!(
                    "Parent level entity not found for model instance {:?} when saving",
                    instance_name,
                ),
            };
            let issue = Issue {
                key: IssueKey {
                    entities: [instance_entity].into(),
                    kind: ORPHAN_MODEL_INSTANCE_ISSUE_UUID,
                },
                brief,
                hint: "Model instances need to be assigned to a parent level entity. \
                      Respawn the orphan model instance"
                    .to_string(),
            };
            let issue_id = commands.spawn(issue).id();
            commands.entity(**root).add_child(issue_id);
        }
    }
}
