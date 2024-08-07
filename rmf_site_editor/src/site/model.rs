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
    interaction::{DragPlaneBundle, Selectable, MODEL_PREVIEW_LAYER},
    site::{Category, Model, PreventDeletion, SiteAssets},
    site_asset_io::MODEL_ENVIRONMENT_VARIABLE,
};
use bevy::{
    asset::{AssetLoadError, LoadState, LoadedUntypedAsset, UntypedAssetId},
    ecs::system::SystemParam,
    gltf::Gltf,
    prelude::*,
    render::view::RenderLayers,
};
use bevy_impulse::*;
use bevy_mod_outline::OutlineMeshExt;
use rmf_site_format::{AssetSource, ModelMarker, Pending, Pose, Scale};
use smallvec::SmallVec;
use std::{any::TypeId, future::Future};

/// Denotes the properties of the current spawned scene for the model, to despawn when updating AssetSource
/// and avoid spurious reloading if the new `AssetSource` is equal to the old one
#[derive(Component, Debug, Clone)]
pub struct ModelScene {
    source: AssetSource,
    entity: Option<Entity>,
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
            AssetSource::Search(name.to_owned() + "/" + model_name + ".obj"),
            AssetSource::Search(name.to_owned() + ".glb"),
            AssetSource::Search(name.to_owned() + ".stl"),
            AssetSource::Search(name.to_owned() + "/" + model_name + ".glb"),
            AssetSource::Search(name.to_owned() + "/model.sdf"),
        ])
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct PendingSpawning(Handle<LoadedUntypedAsset>);

#[derive(Resource)]
/// Services that deal with workspace loading
pub struct ModelLoadingServices {
    /// Service that loads the requested model, returns the Entity where the model was spawned
    pub load_model: Service<ModelLoadingRequest, Result<(), ModelLoadingError>>,
    // TODO(luca) reparent service? or have the previous have entity as an input?
    pub load_asset_source: Service<AssetSource, Result<UntypedHandle, ModelLoadingError>>,
    pub try_all_tentative_formats: Service<SmallVec<[AssetSource; 6]>, Option<UntypedHandle>>,
}

#[derive(SystemParam)]
pub struct ModelLoader<'w, 's> {
    commands: Commands<'w, 's>,
    load_model: Res<'w, ModelLoadingServices>,
}

impl<'w, 's> ModelLoader<'w, 's> {
    pub fn load_model(&mut self, model: Model, parent: Entity) {
        // TODO(luca) cleanup all the structs that we don't need in Model, maybe just use a builder
        // pattern to also include components that we want to propagate
        let request = ModelLoadingRequest {
            model,
            parent,
            load_asset_source: self.load_model.load_asset_source,
            try_all_tentative_formats: self.load_model.try_all_tentative_formats,
        };
        self.commands
            .request(request, self.load_model.load_model)
            .detach();
    }
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
            let handle = match asset_server.load_untyped_async(&asset_path).await {
                Ok(handle) => {
                    if asset_server.is_loaded_with_dependencies(handle.id()) {
                        futures::future::Either::Left(std::future::ready(true))
                    } else {
                        futures::future::Either::Right(std::future::pending())
                    }
                    .await;
                    // TODO(luca) should we make sure it is loaded with dependencies? Return None
                    // if any of the depedencies failed
                    Ok(handle)
                }
                Err(e) => {
                    warn!("Failed loading asset {:?}, reason: {:?}", asset_path, e);
                    Err(ModelLoadingError::AssetServerError(e.to_string()))
                }
            };
            handle
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
        let handle = match channel.query(asset_path, load_callback).await.take() {
            PromiseState::Available(h) => h,
            _ => {
                error!("Failed getting promise from asset server loading");
                return Err(ModelLoadingError::WorkflowExecutionError);
            }
        }?;
        println!("Loaded asset {:?}", request);
        Ok(handle)
    }
}

// TODO*luca) APPLY SCALE TO TRANSFORM
// PROPAGATE RENDER LAYER
// PROPAGATE SELECTABLE
// LOAD WITH DEPENDENCIES
// CURRENT SCENE HANDLING FOR SPURIOUS CHANGE DETECTION
// Input is parent entity and handle for asset
pub fn spawn_scene_for_loaded_model(
    In((e, h)): In<(Entity, UntypedHandle)>,
    mut commands: Commands,
    loading_models: Query<(Entity, &Scale), With<ModelMarker>>,
    mut current_scenes: Query<&mut ModelScene>,
    asset_server: Res<AssetServer>,
    site_assets: Res<SiteAssets>,
    gltfs: Res<Assets<Gltf>>,
) -> Option<Entity> {
    // For each model that is loading, check if its scene has finished loading
    // yet. If the scene has finished loading, then insert it as a child of the
    // model entity and make it selectable.
    let type_id = h.type_id();
    let model_id = if type_id == TypeId::of::<Gltf>() {
        // Guaranteed to be safe in this scope
        // Note we can't do an `if let Some()` because get(Handle) panics if the type is
        // not the stored type
        let gltf = gltfs.get(&h).unwrap();
        // Get default scene if present, otherwise index 0
        let scene = gltf
            .default_scene
            .as_ref()
            .map(|s| s.clone())
            .unwrap_or(gltf.scenes.get(0).unwrap().clone());
        Some(commands.spawn(SceneBundle { scene, ..default() }).id())
    } else if type_id == TypeId::of::<Scene>() {
        let scene = h.clone().typed::<Scene>();
        Some(commands.spawn(SceneBundle { scene, ..default() }).id())
    } else if type_id == TypeId::of::<Mesh>() {
        let mesh = h.clone().typed::<Mesh>();
        Some(
            commands
                .spawn(PbrBundle {
                    mesh,
                    material: site_assets.default_mesh_grey_material.clone(),
                    ..default()
                })
                .id(),
        )
    } else {
        None
    };
    model_id
}

async fn handle_model_loading(
    In(AsyncService {
        request, channel, ..
    }): AsyncServiceInput<ModelLoadingRequest>,
) -> Result<Entity, ModelLoadingError> {
    let entity = channel
        .command(|commands| commands.spawn_empty().id())
        .await
        .available()
        .ok_or(ModelLoadingError::WorkflowExecutionError)?;
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
    let mut handle = None;
    // Note that the spreading + collecting workflow is not necessarily faster because we could be
    // returning early in cases in which the first variant is found (which is usually the case for
    // submeshes in remote SDF files)
    /*
    let handle = channel.query(
        sources, request.try_all_tentative_formats
    ).await.available().ok_or(ModelLoadingError::WorkflowExecutionError)?;
    */
    for source in sources {
        // TODO(luca) remove clone here
        let res = channel
            .query(source.clone(), request.load_asset_source)
            .await
            .available()
            .ok_or(ModelLoadingError::WorkflowExecutionError)?;
        match res {
            Ok(h) => {
                // TODO(luca) now we have a valid asset, spawn a scene here
                info!("Model is OK! {:?}", source);
                handle = Some(h);
                break;
            }
            Err(e) => {}
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
    let scene_entity = res.unwrap();
    // Spawn a ModelScene to keep track of what was spawned, as well as setting scale in the
    // request
    let entity = channel
        .command(move |commands| {
            commands
                .entity(request.parent)
                .insert(ModelScene {
                    source: request.model.source,
                    entity: Some(scene_entity),
                })
                .remove::<PendingSpawning>()
                .add_child(scene_entity);
            commands
                .entity(scene_entity)
                .insert(Transform::from_scale(*request.model.scale));
        })
        .await
        .available()
        .ok_or(ModelLoadingError::WorkflowExecutionError)?;
    Ok(scene_entity)
}

impl FromWorld for ModelLoadingServices {
    fn from_world(world: &mut World) -> Self {
        let model_loading_service = world.spawn_service(handle_model_loading);
        let load_asset_source = world.spawn_service(load_asset_source);
        let load_model = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(model_loading_service)
                .map_block(|res| {
                    // Discard handle for now
                    match res {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            error!("{:?}", e);
                            Err(e)
                        }
                    }
                })
                // .cancel_on_none()
                .connect(scope.terminate)
        });
        // Input is a SmallVec[AssetSource; 6], spread to parallelize and collect results, get the
        // first that succeeded
        let try_all_tentative_formats = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .spread()
                .then(load_asset_source)
                .collect_all::<6>()
                .map_block(|res| res.into_iter().find_map(|el| el.ok()))
                .connect(scope.terminate)
        });

        Self {
            load_model,
            load_asset_source,
            try_all_tentative_formats,
        }
    }
}

pub struct ModelLoadingRequest {
    parent: Entity,
    model: Model,
    load_asset_source: Service<AssetSource, Result<UntypedHandle, ModelLoadingError>>,
    try_all_tentative_formats: Service<SmallVec<[AssetSource; 6]>, Option<UntypedHandle>>,
}

#[derive(Debug)]
pub enum ModelLoadingError {
    WorkflowExecutionError,
    AssetServerError(String),
    InvalidAssetSource(AssetSource),
    FailedLoadingAsset(AssetSource),
}

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
        ),
        (With<ModelMarker>, Changed<AssetSource>),
    >,
    mut model_loader: ModelLoader,
    trashcan: Res<ModelTrashcan>,
) {
    for (e, name, source, pose, is_static, scale, mut model_scene) in new_models.iter_mut() {
        // Only trigger a load if there is no model scene or it is different from the current one
        if let Some(mut scene) = model_scene {
            if scene.source != *source {
                info!("Despawning");
                if let Some(scene_entity) = scene.entity {
                    info!("Found scene");
                    commands.entity(scene_entity).set_parent(trashcan.0);
                }
                commands.entity(e).remove::<ModelScene>();
                // Spawn
                model_loader.load_model(
                    Model {
                        name: name.clone(),
                        source: source.clone(),
                        pose: pose.clone(),
                        is_static: is_static.clone(),
                        scale: scale.clone(),
                        marker: Default::default(),
                    },
                    e,
                );
            }
        } else {
            // Spawn
            model_loader.load_model(
                Model {
                    name: name.clone(),
                    source: source.clone(),
                    pose: pose.clone(),
                    is_static: is_static.clone(),
                    scale: scale.clone(),
                    marker: Default::default(),
                },
                e,
            );
        }
    }
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
        if let Some(scene) = scene.entity {
            if let Ok(mut tf) = transforms.get_mut(scene) {
                tf.scale = **scale;
            }
        }
    }
}

#[derive(Component)]
pub struct Trashcan;

/// The current data structures of models may have nested structures where we
/// spawn "models" within the descendant tree of another model. This can lead to
/// situations where we might try to delete the descendant tree of a model while
/// also modifying one of those descendants. Bevy's current implementation of
/// such commands leads to panic when attempting to modify a despawned entity.
/// To deal with this we defer deleting model descendants by placing them in the
/// trash can and waiting to despawn them during a later stage after any
/// modifier commands have been flushed.
#[derive(Resource)]
pub struct ModelTrashcan(pub Entity);

impl FromWorld for ModelTrashcan {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(Trashcan).id())
    }
}

pub fn clear_model_trashcan(
    mut commands: Commands,
    trashcans: Query<&Children, (With<Trashcan>, Changed<Children>)>,
) {
    for trashcan in &trashcans {
        for trash in trashcan {
            commands.entity(*trash).despawn_recursive();
        }
    }
}

pub fn make_models_selectable(
    mut commands: Commands,
    new_scene_roots: Query<Entity, (Added<ModelScene>, Without<Pending>)>,
    parents: Query<&Parent>,
    scene_roots: Query<(&Selectable, Option<&RenderLayers>), With<ModelMarker>>,
    all_children: Query<&Children>,
    mesh_handles: Query<&Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    // We use adding of scene root as a marker of models being spawned, the component is added when
    // the scene fininshed loading and is spawned
    for model_scene_root in &new_scene_roots {
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
            continue;
        };
        // If layer should not be visible, don't make it selectable
        if render_layers.is_some_and(|r| r.iter().all(|l| l == MODEL_PREVIEW_LAYER)) {
            continue;
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
    }
}

/// Assigns the render layer of the root, if present, to all the children
pub fn propagate_model_render_layers(
    mut commands: Commands,
    new_scene_roots: Query<Entity, Added<ModelScene>>,
    render_layers: Query<&RenderLayers>,
    parents: Query<&Parent>,
    mesh_entities: Query<Entity, With<Handle<Mesh>>>,
    children: Query<&Children>,
) {
    for e in &new_scene_roots {
        let Some(render_layers) = AncestorIter::new(&parents, e)
            .filter_map(|p| render_layers.get(p).ok())
            .last()
        else {
            continue;
        };
        for c in DescendantIter::new(&children, e) {
            if mesh_entities.get(c).is_ok() {
                commands.entity(c).insert(render_layers.clone());
            }
        }
    }
}
