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
    interaction::{DragPlaneBundle, Selectable, VisualCue, MODEL_PREVIEW_LAYER},
    site::{Category, Model, PreventDeletion, SiteAssets},
    site_asset_io::MODEL_ENVIRONMENT_VARIABLE,
};
use bevy::{
    asset::{AssetLoadError, LoadState, LoadedUntypedAsset, UntypedAssetId},
    ecs::system::{Command, SystemParam},
    gltf::Gltf,
    prelude::*,
    render::view::RenderLayers,
    scene::{scene_spawner_system, SceneInstance},
};
use bevy_impulse::*;
use bevy_mod_outline::OutlineMeshExt;
use rmf_site_format::{AssetSource, ModelMarker, Pending, Pose, Scale};
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

/// Stores a sequence of model formats to try loading, the site editor will try them in a sequence
/// until one is successful, or all fail
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub enum TentativeModelFormat {
    #[default]
    Plain,
    GlbFlat,
    Obj,
    Stl,
    GlbFolder,
    Sdf,
}

impl TentativeModelFormat {
    pub fn next(&self) -> Option<Self> {
        use TentativeModelFormat::*;
        match self {
            Plain => Some(GlbFlat),
            GlbFlat => Some(Obj),
            Obj => Some(Stl),
            Stl => Some(GlbFolder),
            GlbFolder => Some(Sdf),
            Sdf => None,
        }
    }

    // Returns what should be appended to the asset source to make it work with the bevy asset
    // loader matching the format
    pub fn to_string(&self, model_name: &str) -> String {
        use TentativeModelFormat::*;
        match self {
            Plain => "".to_owned(),
            Obj => ("/".to_owned() + model_name + ".obj").into(),
            GlbFlat => ".glb".into(),
            Stl => ".stl".into(),
            GlbFolder => ("/".to_owned() + model_name + ".glb").into(),
            Sdf => "/model.sdf".to_owned(),
        }
    }

    pub fn get_all_for_source(name: &str) -> SmallVec<[AssetSource; 6]> {
        let model_name = name.split('/').last().unwrap();
        let format = TentativeModelFormat::default();
        SmallVec::from([
            AssetSource::Search(name.to_owned()),
            AssetSource::Search(name.to_owned() + "/model.sdf"),
            AssetSource::Search(name.to_owned() + "/" + model_name + ".obj"),
            AssetSource::Search(name.to_owned() + ".glb"),
            AssetSource::Search(name.to_owned() + ".stl"),
            AssetSource::Search(name.to_owned() + "/" + model_name + ".glb"),
        ])
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct PendingSpawning(Handle<LoadedUntypedAsset>);

#[derive(Resource)]
/// Services that deal with workspace loading
pub struct ModelLoadingServices {
    /// Service that loads the requested model, returns the Entity where the model was spawned
    pub load_model: Service<ModelLoadingRequest, Entity>,
    pub load_asset_source: Service<AssetSource, Result<UntypedHandle, ModelLoadingError>>,
    pub try_all_tentative_formats: Service<SmallVec<[AssetSource; 6]>, Option<UntypedHandle>>,
    pub check_scene_is_spawned: Service<(Entity, Option<Handle<Scene>>), Entity>,
    /// This workflow updates an existing model to a new asset source
    // TODO(luca) consider using a Result here
    pub update_asset_source_for_model: Service<(Entity, AssetSource), ()>,
}

#[derive(SystemParam)]
pub struct ModelLoader<'w, 's> {
    commands: Commands<'w, 's>,
    load_model: Res<'w, ModelLoadingServices>,
}

#[derive(Default)]
pub struct ModelLoadingPlugin {}

impl Plugin for ModelLoadingPlugin {
    fn build(&self, app: &mut App) {
        let model_loading_services = ModelLoadingServices::from_app(app);
        app.insert_resource(model_loading_services);
    }
}

impl<'w, 's> ModelLoader<'w, 's> {
    pub fn load_model(&mut self, model: Model, parent: Entity) {
        let request = ModelLoadingRequest {
            model,
            parent,
            load_asset_source: self.load_model.load_asset_source,
            try_all_tentative_formats: self.load_model.try_all_tentative_formats,
            check_scene: self.load_model.check_scene_is_spawned,
        };
        self.commands
            .request(request, self.load_model.load_model)
            .detach();
    }
}

// For each InstanceId send a response when it is spawned
fn check_scenes_are_spawned(
    In(ContinuousService { key }): ContinuousServiceInput<(Entity, Option<Handle<Scene>>), Entity>,
    mut orders: ContinuousQuery<(Entity, Option<Handle<Scene>>), Entity>,
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
        match req.1 {
            Some(_) => {
                // There is a scene, make sure the entity has a `SceneInstance` component that marks
                // it as spawned
                if instance_ids.get(req.0).is_ok() && children.get(req.0).is_ok() {
                    order.respond(req.0);
                }
            }
            None => {
                // No scene is present, we can just proceed
                order.respond(req.0);
            }
        }
    })
}

fn load_asset_source(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<AssetSource>,
    asset_server: Res<AssetServer>,
) -> impl Future<Output = Result<UntypedHandle, ModelLoadingError>> {
    let callback = move |In(id): In<UntypedAssetId>, asset_server: Res<AssetServer>| {
        asset_server.get_load_state(id)
    };
    let get_load_state_callback = callback.into_blocking_callback();
    let load_asset_callback = move |In(asset_path): In<String>, asset_server: Res<AssetServer>| {
        let asset_server = asset_server.clone();
        async move {
            asset_server
                .load_untyped_async(&asset_path)
                .await
                .map_err(|e| ModelLoadingError::AssetServerError(e.to_string()))
        }
    };
    let load_callback = load_asset_callback.into_async_callback();
    async move {
        let asset_path = match String::try_from(&request) {
            Ok(asset_path) => asset_path,
            Err(err) => {
                error!(
                    "Invalid syntax while creating asset path for a model: {err}. \
                    Check that your asset information was input correctly. \
                    Current value:\n{:?}",
                    request,
                );
                // TODO(luca) don't both log and return, just a single map err is probably ok?
                return Err(ModelLoadingError::InvalidAssetSource(request.clone()));
            }
        };
        channel
            .query(asset_path, load_callback)
            .await
            .available()
            .ok_or(ModelLoadingError::WorkflowExecutionError)?
    }
}

pub fn spawn_scene_for_loaded_model(
    In((parent, h)): In<(Entity, UntypedHandle)>,
    world: &mut World,
) -> Option<(Entity, Option<Handle<Scene>>)> {
    // For each model that is loading, check if its scene has finished loading
    // yet. If the scene has finished loading, then insert it as a child of the
    // model entity and make it selectable.
    let type_id = h.type_id();
    let (model_id, scene_handle) = if type_id == TypeId::of::<Gltf>() {
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
        Some((
            world
                .spawn(SceneBundle {
                    scene: scene.clone(),
                    ..default()
                })
                .id(),
            Some(scene),
        ))
    } else if type_id == TypeId::of::<Scene>() {
        let scene = h.clone().typed::<Scene>();
        Some((
            world
                .spawn(SceneBundle {
                    scene: scene.clone(),
                    ..default()
                })
                .id(),
            Some(scene),
        ))
    } else if type_id == TypeId::of::<Mesh>() {
        let site_assets = world.resource::<SiteAssets>();
        let mesh = h.clone().typed::<Mesh>();
        Some((
            world
                .spawn(PbrBundle {
                    mesh,
                    material: site_assets.default_mesh_grey_material.clone(),
                    ..default()
                })
                .id(),
            None,
        ))
    } else {
        None
    }?;
    world.entity_mut(model_id).set_parent(parent);
    Some((model_id, scene_handle))
}

/// Return Some(input) if the source changed and we might need to continue downstream operations.
/// None if there was no change and we can dispose downstream operations.
pub fn despawn_if_asset_source_changed(
    In((e, source)): In<(Entity, AssetSource)>,
    mut commands: Commands,
    model_scenes: Query<&ModelScene>,
) -> Option<(Entity, AssetSource)> {
    let Ok(scene) = model_scenes.get(e) else {
        return Some((e, source));
    };

    if scene.source == source {
        return None;
    }
    commands.entity(scene.entity).despawn_recursive();
    commands.entity(e).remove::<ModelScene>();
    Some((e, source))
}

/// Err(()) if no model is actually spawned so we can't update its source
/// Ok(()) if there was a model and a request to update it was dispatched
pub fn update_asset_source_for_model(
    In((e, source)): In<(Entity, AssetSource)>,
    mut commands: Commands,
    mut models: Query<(
        Entity,
        &mut AssetSource,
        &NameInSite,
        &Pose,
        &IsStatic,
        &Scale,
    )>,
) -> Result<(), ()> {
    let Ok((scene_entity, mut current_source, name, pose, is_static, scale)) = models.get_mut(e)
    else {
        return Err(());
    };

    *current_source = source.clone();
    // TODO(luca) a different workflow here? We don't need to readd all the components
    commands.spawn_model(
        e,
        Model {
            name: name.clone(),
            source: source.clone(),
            pose: pose.clone(),
            is_static: is_static.clone(),
            scale: scale.clone(),
            marker: Default::default(),
        },
        None,
    );
    Ok(())
}

async fn handle_model_loading(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<ModelLoadingRequest>,
) -> Result<(Entity, Option<Handle<Scene>>), ModelLoadingError> {
    let spawn_scene = spawn_scene_for_loaded_model.into_blocking_callback();
    let sources = match request.model.source {
        AssetSource::Search(ref name) => TentativeModelFormat::get_all_for_source(name),
        AssetSource::Local(_) | AssetSource::Remote(_) | AssetSource::Package(_) => {
            let mut v = SmallVec::new();
            v.push(request.model.source.clone());
            v
        }
    };

    // TODO(luca) spread this with a fork_clone to parallelize asset loading over all search
    // variants
    // Note that the spreading + collecting workflow is not necessarily faster because we could be
    // returning early in cases in which the first variant is found (which is usually the case for
    // submeshes in remote SDF files)
    /*
    let handle = channel.query(
        sources, request.try_all_tentative_formats
    ).await.available().ok_or(ModelLoadingError::WorkflowExecutionError)?;
    */
    let mut handle = None;
    for source in sources {
        let res = channel
            .query(source, request.load_asset_source)
            .await
            .available()
            .ok_or(ModelLoadingError::WorkflowExecutionError)?;
        if let Ok(h) = res {
            handle = Some(h);
            break;
        }
    }
    let Some(handle) = handle else {
        return Err(ModelLoadingError::FailedLoadingAsset(request.model.source));
    };
    // Now we have a handle and a parent entity, call the spawn scene service
    let res = channel
        .query((request.parent, handle), spawn_scene)
        .await
        .available()
        .ok_or(ModelLoadingError::WorkflowExecutionError)?;
    // TODO(luca) remove unwrap here, handle fail scenarios of spawn_scene_for_loaded_model
    let Some((scene_entity, scene_handle)) = res else {
        return Err(ModelLoadingError::NonModelAsset(request.model.source));
    };
    // Spawn a ModelScene to keep track of what was spawned, as well as setting scale in the
    // request
    let add_components_to_spawned_model = add_components_to_spawned_model.into_blocking_callback();
    channel
        .query(
            (request.parent, scene_entity, request.model.clone()),
            add_components_to_spawned_model,
        )
        .await
        .available()
        .ok_or(ModelLoadingError::WorkflowExecutionError)?;
    let _ = channel
        .query((scene_entity, scene_handle.clone()), request.check_scene)
        .await
        .available()
        .ok_or(ModelLoadingError::WorkflowExecutionError)?;
    Ok((scene_entity, scene_handle))
}

pub fn add_components_to_spawned_model(
    In((parent, scene_entity, model)): In<(Entity, Entity, Model)>,
    mut commands: Commands,
    vis: Query<&Visibility>,
) {
    // TODO(luca) just use commands.insert_if_new when updating to bevy 0.15, check
    // https://github.com/bevyengine/bevy/pull/14646
    commands
        .entity(parent)
        .insert(ModelScene {
            source: model.source.clone(),
            entity: scene_entity,
        })
        .insert(TransformBundle::from_transform(Transform::from_scale(
            model.scale.0,
        )))
        .insert(model)
        .insert(Category::Model)
        .insert(Selectable::new(parent))
        .add_child(scene_entity);
    if vis.get(parent).is_err() {
        commands.entity(parent).insert(VisibilityBundle::default());
    }
}

/// Commands used to spawn a model
pub struct ModelSpawnCommand {
    parent: Entity,
    model: Model,
    then: Option<Callback<Entity, Entity>>,
}

impl Command for ModelSpawnCommand {
    fn apply(self, world: &mut World) {
        let services = world.get_resource::<ModelLoadingServices>()
            .expect("Model loading services not found, make sure the `ModelLoadingServices` Resource has been added to your world");
        let request = ModelLoadingRequest {
            model: self.model,
            parent: self.parent,
            load_asset_source: services.load_asset_source,
            try_all_tentative_formats: services.try_all_tentative_formats,
            check_scene: services.check_scene_is_spawned,
        };
        let load_model = services.load_model.clone();
        world.command(|commands| {
            commands.request(request, load_model).detach();
        });
    }
}

/// Command used to change the asset source for a preexisting model
pub struct SetModelAssetSourceCommand {
    model: Entity,
    source: AssetSource,
}

impl Command for SetModelAssetSourceCommand {
    fn apply(self, world: &mut World) {
        if !world.contains_resource::<ModelLoadingServices>() {
            panic!("Model loading services not found, make sure the `ModelLoadingServices` Resource has been added to your world");
        }
        world.resource_scope(|world, services: Mut<ModelLoadingServices>| {
            world.command(|commands| {
                commands
                    .request(
                        (self.model, self.source),
                        services.update_asset_source_for_model,
                    )
                    .detach();
            });
        });
    }
}

pub trait ModelSpawningExt<'w, 's> {
    fn spawn_model(&mut self, parent: Entity, model: Model, then: Option<Callback<Entity, Entity>>);

    fn set_model_asset_source(&mut self, model: Entity, source: AssetSource);
}

impl<'w, 's> ModelSpawningExt<'w, 's> for Commands<'w, 's> {
    fn spawn_model(
        &mut self,
        parent: Entity,
        model: Model,
        then: Option<Callback<Entity, Entity>>,
    ) {
        self.add(ModelSpawnCommand {
            parent,
            model,
            then,
        });
    }

    fn set_model_asset_source(&mut self, model: Entity, source: AssetSource) {
        self.add(SetModelAssetSourceCommand { model, source });
    }
}

fn load_model_dependencies(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<Entity>,
    children_q: Query<&Children>,
    parents: Query<&Parent>,
    models: Query<(&NameInSite, &AssetSource, &Pose, &IsStatic, &Scale), With<ModelMarker>>,
) -> impl Future<Output = Result<Entity, ModelLoadingError>> {
    let models = DescendantIter::new(&children_q, request)
        .filter_map(|c| {
            models
                .get(c)
                .ok()
                .map(|(name, source, pose, is_static, scale)| {
                    (
                        c,
                        Model {
                            name: name.clone(),
                            source: source.clone(),
                            pose: pose.clone(),
                            is_static: is_static.clone(),
                            scale: scale.clone(),
                            marker: Default::default(),
                        },
                    )
                })
        })
        .collect::<Vec<_>>();
    async move {
        for (model_entity, model) in models {
            channel
                .command(move |commands| {
                    commands.spawn_model(model_entity, model, None);
                })
                .await
                .available()
                .ok_or(ModelLoadingError::WorkflowExecutionError)?;
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

/// Cleans up a model that failed spawning
pub fn cleanup_failed_model(In(e): In<Entity>) {}

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
        let load_asset_source = app.world.spawn_service(load_asset_source);
        let update_asset_source_for_model = app.world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(despawn_if_asset_source_changed.into_blocking_callback())
                .dispose_on_none()
                .then(update_asset_source_for_model.into_blocking_callback())
                .map_block(|_| ())
                .connect(scope.terminate)
        });
        let load_model = app.world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                /*
                .fork_unzip((
                    |entity| entity.output(),
                    |model| {
                        let source = model.source.clone();
                        source.then(update_asset_source_for_model).map_block(|res|
                    }
                ))
                */
                .then(model_loading_service)
                .map_block(|res| match res {
                    Ok(entity) => Ok(entity),
                    Err(e) => {
                        error!("{:?}", e);
                        Err(e)
                    }
                })
                .cancel_on_quiet_err()
                .map_block(|res| res.0)
                // Implement, iterate through descendants and for each emit a model_loading_service
                // .then(check_scene_is_spawned)
                /*
                .fork_unzip((
                        |entity| entity.output(),
                        |handle| handle.dispose_on_none().then(check_scene_is_spawned).output(),
                ))
                .join(builder)
                .map_block(|(e, _)| e)
                */
                .then(load_model_dependencies)
                .cancel_on_quiet_err()
                // The model and its dependencies are spawned, make them selectable / propagate
                // render layers
                // TODO(luca) Consider cleanup strategies for failure, should we despawn models?
                .then(propagate_model_render_layers.into_blocking_callback())
                .then(make_models_selectable.into_blocking_callback())
                .connect(scope.terminate)
        });
        // Input is a SmallVec[AssetSource; 6], spread to parallelize and collect results, get the
        // first that succeeded
        let try_all_tentative_formats = app.world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .spread()
                .then(load_asset_source)
                .dispose_on_quiet_err()
                .collect::<1>(0, Some(1))
                .map_block(|res| res.into_iter().next())
                .connect(scope.terminate)
        });

        Self {
            load_model,
            load_asset_source,
            try_all_tentative_formats,
            check_scene_is_spawned,
            update_asset_source_for_model,
        }
    }
}

pub struct ModelLoadingRequest {
    parent: Entity,
    model: Model,
    load_asset_source: Service<AssetSource, Result<UntypedHandle, ModelLoadingError>>,
    try_all_tentative_formats: Service<SmallVec<[AssetSource; 6]>, Option<UntypedHandle>>,
    check_scene: Service<(Entity, Option<Handle<Scene>>), Entity>,
}

#[derive(Debug)]
pub enum ModelLoadingError {
    WorkflowExecutionError,
    AssetServerError(String),
    InvalidAssetSource(AssetSource),
    FailedLoadingAsset(AssetSource),
    /// The file that was loaded does not contain a model
    NonModelAsset(AssetSource),
}

use crate::site::{CollisionMeshMarker, VisualMeshMarker};
use crate::site::{IsStatic, NameInSite};
pub fn load_new_models(
    mut commands: Commands,
    mut new_models: Query<
        (
            Entity,
            &NameInSite,
            &AssetSource,
            &Pose,
            &IsStatic,
            &Scale,
            Option<&mut ModelScene>,
            Has<Visibility>,
        ),
        (With<ModelMarker>, Changed<AssetSource>),
    >,
    submeshes: Query<(), Or<(With<CollisionMeshMarker>, With<VisualMeshMarker>)>>,
) {
    return;
    /*
    for (e, name, source, pose, is_static, scale, mut model_scene, has_vis) in new_models.iter_mut()
    {
        if !has_vis && submeshes.get(e).is_err() {
            commands.entity(e).insert(VisibilityBundle::default());
        }
        // Only trigger a load if there is no model scene or it is different from the current one
        if let Some(mut scene) = model_scene {
            if scene.source != *source {
                if let Some(scene_entity) = scene.entity {
                    commands.entity(scene_entity).despawn_recursive();
                }
                commands.entity(e).remove::<ModelScene>();
                // Spawn
                commands.spawn_model(
                    e,
                    Model {
                        name: name.clone(),
                        source: source.clone(),
                        pose: pose.clone(),
                        is_static: is_static.clone(),
                        scale: scale.clone(),
                        marker: Default::default(),
                    },
                    None,
                );
            }
        } else {
            // Spawn
            commands.spawn_model(
                e,
                Model {
                    name: name.clone(),
                    source: source.clone(),
                    pose: pose.clone(),
                    is_static: is_static.clone(),
                    scale: scale.clone(),
                    marker: Default::default(),
                },
                None,
            );
        }
    }
    */
}

pub fn update_model_tentative_formats(
    mut commands: Commands,
    changed_models: Query<Entity, (Changed<AssetSource>, With<ModelMarker>)>,
    mut loading_models: Query<
        (
            Entity,
            &mut TentativeModelFormat,
            &PendingSpawning,
            &AssetSource,
        ),
        With<ModelMarker>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: ModelLoader,
) {
    return;
    static SUPPORTED_EXTENSIONS: &[&str] = &["obj", "stl", "sdf", "glb", "gltf"];
    for e in changed_models.iter() {
        // Reset to the first format
        commands.entity(e).insert(TentativeModelFormat::default());
    }
    // Check from the asset server if any format failed, if it did try the next
    for (e, mut tentative_format, h, source) in loading_models.iter_mut() {
        if matches!(asset_server.get_load_state(h.id()), Some(LoadState::Failed)) {
            let mut cmd = commands.entity(e);
            cmd.remove::<PreventDeletion>();
            // We want to iterate only for search asset types, for others just print an error
            if matches!(source, AssetSource::Search(_)) {
                if let Some(fmt) = tentative_format.next() {
                    *tentative_format = fmt;
                    cmd.remove::<PendingSpawning>();
                    continue;
                }
            }
            let asset_path = match String::try_from(source) {
                Ok(asset_path) => asset_path,
                Err(err) => {
                    error!(
                        "Invalid syntax while creating asset path to load a model: {err}. \
                        Check that your asset information was input correctly. \
                        Current value:\n{:?}",
                        source,
                    );
                    continue;
                }
            };
            let model_ext = asset_path
                .rsplit_once('.')
                .map(|s| s.1.to_owned())
                .unwrap_or_else(|| tentative_format.to_string(""));
            let reason = if !SUPPORTED_EXTENSIONS.iter().any(|e| model_ext.ends_with(e)) {
                "Format not supported".to_owned()
            } else {
                match source {
                    AssetSource::Search(_) | AssetSource::Remote(_) => format!(
                        "Model not found, try using an API key if it belongs to \
                                a private organization, or add its path to the {} \
                                environment variable",
                        MODEL_ENVIRONMENT_VARIABLE
                    ),
                    _ => "Failed parsing file".to_owned(),
                }
            };
            warn!(
                "Failed loading Model with source {}: {}",
                asset_path, reason
            );
            cmd.remove::<TentativeModelFormat>();
        }
    }
}

pub fn update_model_scales(
    changed_scales: Query<(&Scale, &ModelScene), Changed<Scale>>,
    mut transforms: Query<&mut Transform>,
) {
    for (scale, scene) in changed_scales.iter() {
        if let Ok(mut tf) = transforms.get_mut(scene.entity) {
            tf.scale = **scale;
        }
    }
}

pub fn make_models_selectable(
    In(model_scene_root): In<Entity>,
    mut commands: Commands,
    pending: Query<(), With<Pending>>,
    parents: Query<&Parent>,
    scene_roots: Query<(&Selectable, Option<&RenderLayers>), With<ModelMarker>>,
    all_children: Query<&Children>,
    mesh_handles: Query<&Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) -> Entity {
    // Pending items (i.e. mouse previews) should not be selectable
    if pending.get(model_scene_root).is_ok() {
        return model_scene_root;
    }
    // Use a small vec here to try to dodge heap allocation if possible.
    // TODO(MXG): Run some tests to see if an allocation of 16 is typically
    // sufficient.
    let mut queue: SmallVec<[Entity; 16]> = SmallVec::new();
    // A root might be a child of another root, for example for SDF models that have multiple
    // submeshes. We need to traverse up to find the highest level scene to use for selecting
    // behavior
    let Some((selectable, render_layers)) = AncestorIter::new(&parents, model_scene_root)
        .filter_map(|p| scene_roots.get(p).ok())
        .last()
        .or_else(|| scene_roots.get(model_scene_root).ok())
    else {
        return model_scene_root;
    };
    // If layer should not be visible, don't make it selectable
    if render_layers.is_some_and(|r| r.iter().all(|l| l == MODEL_PREVIEW_LAYER)) {
        return model_scene_root;
    }
    queue.push(model_scene_root);

    while let Some(e) = queue.pop() {
        commands
            .entity(e)
            .insert(selectable.clone())
            .insert(DragPlaneBundle::new(selectable.element, Vec3::Z));

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
    model_scene_root
}

/// Assigns the render layer of the root, if present, to all its children
pub fn propagate_model_render_layers(
    In(e): In<Entity>,
    mut commands: Commands,
    render_layers: Query<&RenderLayers>,
    parents: Query<&Parent>,
    mesh_entities: Query<Entity, With<Handle<Mesh>>>,
    children: Query<&Children>,
) -> Entity {
    let Some(render_layers) = AncestorIter::new(&parents, e)
        .filter_map(|p| render_layers.get(p).ok())
        .last()
    else {
        return e;
    };
    for c in DescendantIter::new(&children, e) {
        if mesh_entities.get(c).is_ok() {
            commands.entity(c).insert(render_layers.clone());
        }
    }
    e
}
