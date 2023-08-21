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
    interaction::{DragPlaneBundle, Selectable},
    site::{Category, PreventDeletion, SiteAssets},
    SdfRoot,
};
use bevy::{asset::LoadState, gltf::Gltf, prelude::*};
use bevy_mod_outline::OutlineMeshExt;
use rmf_site_format::{AssetSource, ModelMarker, Pending, Pose, Scale, UrdfRoot};
use smallvec::SmallVec;

#[derive(Component, Debug, Clone)]
pub struct ModelScene {
    source: AssetSource,
    format: TentativeModelFormat,
    entity: Option<Entity>,
}

/// Stores a sequence of model formats to try loading, the site editor will try them in a sequence
/// until one is successful, or all fail
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub enum TentativeModelFormat {
    #[default]
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
            Obj => ("/".to_owned() + model_name + ".obj").into(),
            GlbFlat => ".glb".into(),
            Stl => ".stl".into(),
            GlbFolder => ("/".to_owned() + model_name + ".glb").into(),
            Sdf => "/model.sdf".to_owned(),
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct PendingSpawning(HandleUntyped);

/// A unit component to mark where a scene begins
#[derive(Component, Debug, Clone, Copy)]
pub struct ModelSceneRoot;

pub fn update_model_scenes(
    mut commands: Commands,
    changed_models: Query<
        (
            Entity,
            &AssetSource,
            &Pose,
            &TentativeModelFormat,
            Option<&Visibility>,
        ),
        (Changed<TentativeModelFormat>, With<ModelMarker>),
    >,
    asset_server: Res<AssetServer>,
    loading_models: Query<(Entity, &PendingSpawning, &Scale), With<ModelMarker>>,
    spawned_models: Query<
        Entity,
        (
            Without<PendingSpawning>,
            With<ModelMarker>,
            With<PreventDeletion>,
        ),
    >,
    mut current_scenes: Query<&mut ModelScene>,
    site_assets: Res<SiteAssets>,
    meshes: Res<Assets<Mesh>>,
    scenes: Res<Assets<Scene>>,
    gltfs: Res<Assets<Gltf>>,
    urdfs: Res<Assets<UrdfRoot>>,
    sdfs: Res<Assets<SdfRoot>>,
) {
    fn spawn_model(
        e: Entity,
        source: &AssetSource,
        pose: &Pose,
        asset_server: &AssetServer,
        tentative_format: &TentativeModelFormat,
        has_visibility: bool,
        commands: &mut Commands,
    ) {
        let mut commands = commands.entity(e);
        commands
            .insert(ModelScene {
                source: source.clone(),
                format: tentative_format.clone(),
                entity: None,
            })
            .insert(TransformBundle::from_transform(pose.transform()))
            .insert(Category::Model);

        if !has_visibility {
            commands.insert(VisibilityBundle {
                visibility: Visibility::VISIBLE,
                ..default()
            });
        }

        // For search assets, look at subfolders and iterate through file formats
        // TODO(luca) This will also iterate for non search assets, fix
        let asset_source = match source {
            AssetSource::Search(name) => {
                let model_name = name.split('/').last().unwrap();
                AssetSource::Search(name.to_owned() + &tentative_format.to_string(model_name))
            }
            _ => source.clone(),
        };
        let handle = asset_server.load_untyped(&String::from(&asset_source));
        commands
            .insert(PreventDeletion::because(
                "Waiting for model to spawn".to_string(),
            ))
            .insert(PendingSpawning(handle));
    }

    // There is a bug(?) in bevy scenes, which causes panic when a scene is despawned
    // immediately after it is spawned.
    // Work around it by checking the `spawned` container BEFORE updating it so that
    // entities are only despawned at the next frame. This also ensures that entities are
    // "fully spawned" before despawning.
    for e in spawned_models.iter() {
        commands.entity(e).remove::<PreventDeletion>();
    }

    // For each model that is loading, check if its scene has finished loading
    // yet. If the scene has finished loading, then insert it as a child of the
    // model entity and make it selectable.
    for (e, h, scale) in loading_models.iter() {
        if asset_server.get_load_state(&h.0) == LoadState::Loaded {
            let model_id = if let Some(gltf) = gltfs.get(&h.typed_weak::<Gltf>()) {
                Some(commands.entity(e).add_children(|parent| {
                    // Get default scene if present, otherwise index 0
                    let scene = gltf
                        .default_scene
                        .as_ref()
                        .map(|s| s.clone())
                        .unwrap_or(gltf.scenes.get(0).unwrap().clone());
                    parent
                        .spawn(SceneBundle {
                            scene,
                            transform: Transform::from_scale(**scale),
                            ..default()
                        })
                        .id()
                }))
            } else if scenes.contains(&h.typed_weak::<Scene>()) {
                Some(commands.entity(e).add_children(|parent| {
                    let h_typed = h.0.clone().typed::<Scene>();
                    parent
                        .spawn(SceneBundle {
                            scene: h_typed,
                            transform: Transform::from_scale(**scale),
                            ..default()
                        })
                        .id()
                }))
            } else if meshes.contains(&h.typed_weak::<Mesh>()) {
                Some(commands.entity(e).add_children(|parent| {
                    let h_typed = h.0.clone().typed::<Mesh>();
                    parent
                        .spawn(PbrBundle {
                            mesh: h_typed,
                            material: site_assets.default_mesh_grey_material.clone(),
                            transform: Transform::from_scale(**scale),
                            ..default()
                        })
                        .id()
                }))
            } else if let Some(urdf) = urdfs.get(&h.typed_weak::<UrdfRoot>()) {
                Some(commands.entity(e).add_children(|parent| {
                    parent
                        .spawn(SpatialBundle::VISIBLE_IDENTITY)
                        .insert(urdf.clone())
                        .insert(Category::Workcell)
                        .id()
                }))
            } else if let Some(sdf) = sdfs.get(&h.typed_weak::<SdfRoot>()) {
                Some(commands.entity(e).add_children(|parent| {
                    parent
                        .spawn(SpatialBundle::VISIBLE_IDENTITY)
                        .insert(sdf.clone())
                        .id()
                }))
            } else {
                None
            };
            if let Some(id) = model_id {
                commands
                    .entity(e)
                    .insert(ModelSceneRoot)
                    .insert(Selectable::new(e));
                commands.entity(e).remove::<PendingSpawning>();
                current_scenes.get_mut(e).unwrap().entity = Some(id);
            }
        }
    }

    // update changed models
    for (e, source, pose, tentative_format, vis_option) in changed_models.iter() {
        if let Ok(current_scene) = current_scenes.get_mut(e) {
            // Avoid respawning if spurious change detection was triggered
            if current_scene.source != *source || current_scene.format != *tentative_format {
                if let Some(scene_entity) = current_scene.entity {
                    commands.entity(scene_entity).despawn_recursive();
                    commands.entity(e).remove_children(&[scene_entity]);
                    commands.entity(e).remove::<ModelSceneRoot>();
                }
                // Updated model
                spawn_model(
                    e,
                    source,
                    pose,
                    &asset_server,
                    tentative_format,
                    vis_option.is_some(),
                    &mut commands,
                );
            }
        } else {
            // New model
            spawn_model(
                e,
                source,
                pose,
                &asset_server,
                tentative_format,
                vis_option.is_some(),
                &mut commands,
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
) {
    for e in changed_models.iter() {
        // Reset to the first format
        commands.entity(e).insert(TentativeModelFormat::default());
    }
    // Check from the asset server if any format failed, if it did try the next
    for (e, mut tentative_format, h, source) in loading_models.iter_mut() {
        match asset_server.get_load_state(&h.0) {
            LoadState::Failed => {
                if let Some(fmt) = tentative_format.next() {
                    *tentative_format = fmt;
                    commands.entity(e).remove::<PendingSpawning>();
                } else {
                    warn!("Model with source {} not found", String::from(source));
                    commands.entity(e).remove::<TentativeModelFormat>();
                    commands.entity(e).remove::<PreventDeletion>();
                }
            }
            _ => {}
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

pub fn make_models_selectable(
    mut commands: Commands,
    new_scene_roots: Query<Entity, (Added<ModelSceneRoot>, Without<Pending>)>,
    parents: Query<&Parent>,
    scene_roots: Query<&Selectable, With<ModelMarker>>,
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
        let selectable = AncestorIter::new(&parents, model_scene_root)
            .filter_map(|p| scene_roots.get(p).ok())
            .last()
            .unwrap_or(scene_roots.get(model_scene_root).unwrap());
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
