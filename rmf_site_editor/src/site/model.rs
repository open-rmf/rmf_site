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
    interaction::{DragPlaneBundle, Preview, MODEL_PREVIEW_LAYER},
    site::SiteAssets,
    site_asset_io::MODEL_ENVIRONMENT_VARIABLE,
};
use bevy::{
    ecs::system::Command, gltf::Gltf, prelude::*, render::view::RenderLayers, scene::SceneInstance,
};
use bevy_impulse::*;
use bevy_mod_outline::OutlineMeshExt;
use rmf_site_format::{AssetSource, ModelMarker, Pending, Scale};
use smallvec::SmallVec;
use std::{any::TypeId, future::Future};
use thiserror::Error;

/// Denotes the properties of the current spawned scene for the model, to despawn when updating AssetSource
/// and avoid spurious reloading if the new `AssetSource` is equal to the old one
#[derive(Component, Debug, Clone)]
pub struct ModelScene {
    source: AssetSource,
    entity: Entity,
}

/// For a given `AssetSource`, return all the sources that we should try loading.
pub fn get_all_for_source(source: &AssetSource) -> SmallVec<[AssetSource; 6]> {
    match source {
        AssetSource::Search(ref name) => {
            let model_name = name.split('/').last().unwrap();
            SmallVec::from([
                AssetSource::Search(name.to_owned()),
                AssetSource::Search(name.to_owned() + "/model.sdf"),
                AssetSource::Search(name.to_owned() + "/" + model_name + ".obj"),
                AssetSource::Search(name.to_owned() + ".glb"),
                AssetSource::Search(name.to_owned() + ".stl"),
                AssetSource::Search(name.to_owned() + "/" + model_name + ".glb"),
            ])
        }
        AssetSource::Local(_) | AssetSource::Remote(_) | AssetSource::Package(_) => {
            let mut v = SmallVec::new();
            v.push(source.clone());
            v
        }
    }
}

pub type ModelLoadingResult = Result<ModelLoadingRequest, Option<ModelLoadingError>>;

#[derive(Resource)]
/// Services that deal with workspace loading
pub struct ModelLoadingServices {
    /// Service that loads the requested model
    pub load_model: Service<ModelLoadingRequest, ModelLoadingResult>,
    /// Continuous service that sends a response when the scene at the requested entity finished
    /// spawning.
    pub check_scene_is_spawned: Service<Entity, Entity>,
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
    instance_ids: Query<(), With<SceneInstance>>,
    // We use having children as a proxy for scene having been spawned, alternatives are fairly
    // complex (i.e. reading the instance_is_ready API needs the InstanceId that is private, the
    // SceneInstanceReady event needs access to the parent entity that we would need in a Local
    children: Query<(), With<Children>>,
) {
    let Some(mut orders) = orders.get_mut(&key) else {
        return;
    };

    orders.for_each(|order| {
        let req = order.request().clone();
        // Make sure the entity has a `SceneInstance` component that marks it as spawned
        if instance_ids.get(req).is_ok() && children.get(req).is_ok() {
            order.respond(req);
        }
    })
}

fn load_asset_source(
    In(source): In<AssetSource>,
    asset_server: Res<AssetServer>,
) -> impl Future<Output = Result<UntypedHandle, ModelLoadingError>> {
    let asset_server = asset_server.clone();
    async move {
        let asset_path = match String::try_from(&source) {
            Ok(asset_path) => asset_path,
            Err(err) => {
                return Err(ModelLoadingError::InvalidAssetSource(
                    source,
                    err.to_string(),
                ));
            }
        };
        asset_server
            .load_untyped_async(&asset_path)
            .await
            .map_err(|e| ModelLoadingError::AssetServerError(source, e.to_string()))
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
        let gltf = gltfs.get(&h)?;
        // Get default scene if present, otherwise index 0
        let scene = gltf
            .default_scene
            .as_ref()
            .map(|s| s.clone())
            .unwrap_or(gltf.scenes.get(0).unwrap().clone());
        Some((world.spawn(SceneBundle { scene, ..default() }).id(), true))
    } else if type_id == TypeId::of::<Scene>() {
        let scene = h.typed::<Scene>();
        Some((world.spawn(SceneBundle { scene, ..default() }).id(), true))
    } else if type_id == TypeId::of::<Mesh>() {
        let site_assets = world.resource::<SiteAssets>();
        let mesh = h.typed::<Mesh>();
        Some((
            world
                .spawn(PbrBundle {
                    mesh,
                    material: site_assets.default_mesh_grey_material.clone(),
                    ..default()
                })
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
            entity: model_id,
        })
        .add_child(model_id);
    if world.get::<Visibility>(parent).is_none() {
        world.entity_mut(parent).insert(VisibilityBundle::default());
    }
    Some((model_id, is_scene))
}

/// Return true if the source changed and we might need to continue downstream operations.
/// false if there was no change and we can dispose downstream operations.
pub fn despawn_if_asset_source_changed(
    In((e, source)): In<(Entity, AssetSource)>,
    mut commands: Commands,
    model_scenes: Query<&ModelScene>,
) -> bool {
    commands.entity(e).insert(source.clone());
    let Ok(scene) = model_scenes.get(e) else {
        return true;
    };

    if scene.source == source {
        return false;
    }
    commands.entity(scene.entity).despawn_recursive();
    commands.entity(e).remove::<ModelScene>();
    true
}

fn handle_model_loading(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<ModelLoadingRequest>,
    model_services: Res<ModelLoadingServices>,
) -> impl Future<Output = ModelLoadingResult> {
    let check_scene_is_spawned = model_services.check_scene_is_spawned.clone();
    let spawn_scene = spawn_scene_for_loaded_model.into_blocking_callback();
    async move {
        let asset_changed = channel
            .query(
                (request.parent, request.source.clone()),
                despawn_if_asset_source_changed.into_blocking_callback(),
            )
            .await
            .available()
            .ok_or(ModelLoadingError::WorkflowExecutionError)?;
        if !asset_changed {
            return Err(None);
        }
        let sources = get_all_for_source(&request.source);

        let load_asset_source = load_asset_source.into_async_callback();
        let mut handle = None;
        for source in sources {
            let res = channel
                .query(source, load_asset_source.clone())
                .await
                .available()
                .ok_or(ModelLoadingError::WorkflowExecutionError)?;
            if let Ok(h) = res {
                handle = Some(h);
                break;
            }
        }
        let Some(handle) = handle else {
            return Err(Some(ModelLoadingError::FailedLoadingAsset(request.source)));
        };
        // Now we have a handle and a parent entity, call the spawn scene service
        let res = channel
            .query(
                (request.parent, handle, request.source.clone()),
                spawn_scene,
            )
            .await
            .available()
            .ok_or(Some(ModelLoadingError::WorkflowExecutionError))?;
        let Some((scene_entity, is_scene)) = res else {
            return Err(Some(ModelLoadingError::NonModelAsset(request.source)));
        };
        if is_scene {
            // Wait for the scene to be spawned, if there is one
            channel
                .query(scene_entity, check_scene_is_spawned)
                .await
                .available()
                .ok_or(Some(ModelLoadingError::WorkflowExecutionError))?;
        }
        Ok(request)
    }
}

impl Command for ModelLoadingRequest {
    fn apply(self, world: &mut World) {
        let services = world.get_resource::<ModelLoadingServices>()
            .expect("Model loading services not found, make sure the `ModelLoadingServices` Resource has been added to your world");
        let load_model = services.load_model.clone();
        world.command(|commands| {
            let e = self.parent;
            let promise = commands.request(self, load_model).take_response();
            commands.entity(e).insert(ModelLoadingState(promise));
        });
    }
}

/// Component added to models that are being loaded
#[derive(Component, Deref, DerefMut)]
pub struct ModelLoadingState(Promise<ModelLoadingResult>);

/// Component added to models that failed loading and containing the reason loading failed.
#[derive(Component, Deref, DerefMut)]
pub struct ModelFailedLoading(ModelLoadingError);

/// Polling system that checks the state of promises and prints errors / adds marker components if
/// models failed loading
pub fn maintain_model_loading_states(
    mut loading_states: Query<(Entity, &mut ModelLoadingState, Option<&ModelScene>)>,
    mut commands: Commands,
) {
    for (e, mut state, scene_opt) in &mut loading_states {
        if (**state).peek().is_available() {
            let result = state.take().available().unwrap();
            if let Some(err) = result.err().flatten() {
                // Cleanup the scene
                if let Some(scene) = scene_opt {
                    commands.entity(scene.entity).despawn_recursive();
                    commands.entity(e).remove::<ModelScene>();
                }
                error!("{err}");
                commands.entity(e).insert(ModelFailedLoading(err));
            }
            commands.entity(e).remove::<ModelLoadingState>();
        }
    }
}

pub trait ModelSpawningExt<'w, 's> {
    fn spawn_model(&mut self, request: ModelLoadingRequest);
}

impl<'w, 's> ModelSpawningExt<'w, 's> for Commands<'w, 's> {
    fn spawn_model(&mut self, request: ModelLoadingRequest) {
        self.add(request);
    }
}

fn load_model_dependencies(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<ModelLoadingRequest>,
    children_q: Query<&Children>,
    models: Query<&AssetSource, With<ModelMarker>>,
    model_loading: Res<ModelLoadingServices>,
) -> impl Future<Output = ModelLoadingResult> {
    let models = DescendantIter::new(&children_q, request.parent)
        .filter_map(|c| models.get(c).ok().map(|source| (c, source.clone())))
        .collect::<Vec<_>>();
    let load_model = model_loading.load_model.clone();
    let root_source = request.source.clone();
    async move {
        for (model_entity, source) in models {
            channel
                .query((model_entity, source).into(), load_model)
                .await
                .available()
                .ok_or(Some(ModelLoadingError::WorkflowExecutionError))?
                .map_err(|err| {
                    Some(ModelLoadingError::FailedLoadingDependency(
                        root_source.clone(),
                        err.map(|e| e.to_string()).unwrap_or("No error".to_string()),
                    ))
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

fn finalize_model(In(request): In<ModelLoadingRequest>, world: &mut World) -> ModelLoadingResult {
    let parent = request.parent.clone();
    if let Some(then) = request.then.clone() {
        world.command(|cmd: &mut Commands| {
            cmd.request(parent, then).detach();
        });
    }
    Ok(request)
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
        .add_systems(Update, maintain_model_loading_states)
        .add_systems(
            PostUpdate,
            (apply_deferred, flush_impulses()).in_set(ModelLoadingSet::CheckSceneFlush),
        );
        let check_scene_is_spawned = app.spawn_continuous_service(
            PostUpdate,
            check_scenes_are_spawned.configure(|config: SystemConfigs| {
                config.in_set(ModelLoadingSet::CheckSceneSystem)
            }),
        );
        let load_model_dependencies = app.world.spawn_service(load_model_dependencies);
        let model_loading_service = app.world.spawn_service(handle_model_loading);
        let load_model = app.world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(model_loading_service)
                .connect_on_err(scope.terminate)
                .then(load_model_dependencies)
                .connect_on_err(scope.terminate)
                // The model and its dependencies are spawned, make them selectable / propagate
                // render layers
                .then(propagate_model_properties.into_blocking_callback())
                .then(make_models_selectable.into_blocking_callback())
                .then(finalize_model.into_blocking_callback())
                .connect(scope.terminate)
        });

        Self {
            load_model,
            check_scene_is_spawned,
        }
    }
}

pub struct ModelLoadingRequest {
    /// The entity to spawn the model for
    // TODO(luca) consider making this an option to avoid users having to do spawn_empty if they
    // don't need to pass an entity
    pub parent: Entity,
    /// AssetSource pointing to which asset we want to load
    pub source: AssetSource,
    /// A callback to be executed on the spawned model. This can be used for complex operations
    /// that require querying / interactions with the ECS
    pub then: Option<Callback<Entity, ()>>,
}

impl From<(Entity, AssetSource)> for ModelLoadingRequest {
    fn from(t: (Entity, AssetSource)) -> Self {
        ModelLoadingRequest::new(t.0, t.1)
    }
}

impl ModelLoadingRequest {
    pub fn new(parent: Entity, source: AssetSource) -> Self {
        Self {
            parent,
            source,
            then: None,
        }
    }

    pub fn then(mut self, then: Callback<Entity, ()>) -> Self {
        self.then = Some(then);
        self
    }
}

#[derive(Debug, Error)]
pub enum ModelLoadingError {
    #[error("Error executing the model loading workflow")]
    WorkflowExecutionError,
    #[error("Asset server error while loading [{0:?}]: {1}")]
    AssetServerError(AssetSource, String),
    #[error(
        "Invalid syntax while creating asset path for a model: {1}. \
        Check that your asset information was input correctly. \
        Current value:\n{0:?}"
    )]
    InvalidAssetSource(AssetSource, String),
    #[error(
        "Failed loading asset [{0:?}], make sure it is in a supported format (.dae is not supported),\
        try using an API key if it belongs to a private organization \
        or add its path to the {MODEL_ENVIRONMENT_VARIABLE} environment variable."
    )]
    FailedLoadingAsset(AssetSource),
    #[error("Asset at source [{0:?}] did not contain a model")]
    NonModelAsset(AssetSource),
    #[error("Failed loading dependency for model with source [{0:?}]: {1}")]
    FailedLoadingDependency(AssetSource, String),
}

pub fn update_model_scales(
    changed_scales: Query<(&Scale, &ModelScene), Or<(Changed<Scale>, Changed<ModelScene>)>>,
    mut transforms: Query<&mut Transform>,
) {
    for (scale, scene) in changed_scales.iter() {
        if let Ok(mut tf) = transforms.get_mut(scene.entity) {
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
    mesh_handles: Query<&Handle<Mesh>>,
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
        .ok()
        .is_some_and(|r| r.iter().all(|l| l == MODEL_PREVIEW_LAYER))
    {
        return req;
    }
    queue.push(req.parent);

    while let Some(e) = queue.pop() {
        commands
            .entity(e)
            .insert(DragPlaneBundle::new(req.parent, Vec3::Z));

        if let Ok(mesh_handle) = mesh_handles.get(e) {
            if let Some(mesh) = mesh_assets.get_mut(mesh_handle) {
                if mesh.generate_outline_normals().is_err() {
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

/// Assigns the render layer of the root, if present, to all the children
pub fn propagate_model_properties(
    In(req): In<ModelLoadingRequest>,
    mut commands: Commands,
    render_layers: Query<&RenderLayers>,
    previews: Query<&Preview>,
    pendings: Query<&Pending>,
    mesh_entities: Query<(), With<Handle<Mesh>>>,
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
    mesh_entities: &Query<(), With<Handle<Mesh>>>,
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
