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
};
use bevy::{asset::LoadState, prelude::*};
use bevy_mod_outline::OutlineMeshExt;
use rmf_site_format::{AssetSource, ModelMarker, Pose, UrdfRoot, Scale};
use smallvec::SmallVec;


#[derive(Component, Debug, Clone)]
pub struct ModelScene {
    source: AssetSource,
    entity: Option<Entity>,
}

#[derive(Component, Deref, DerefMut)]
pub struct PendingSpawning(HandleUntyped);

/// A unit component to mark where a scene begins
#[derive(Component, Debug, Clone, Copy)]
pub struct ModelSceneRoot;

pub fn update_model_scenes(
    mut commands: Commands,
    mut changed_models: Query<
        (Entity, &AssetSource, &Pose),
        (Changed<AssetSource>, With<ModelMarker>),
    >,
    asset_server: Res<AssetServer>,
    loading_models: Query<(Entity, &PendingSpawning, &Scale)>,
    spawned_models: Query<Entity, (Without<PendingSpawning>, With<PreventDeletion>)>,
    mut current_scenes: Query<&mut ModelScene>,
    site_assets: Res<SiteAssets>,
    meshes: Res<Assets<Mesh>>,
    scenes: Res<Assets<Scene>>,
    urdfs: Res<Assets<UrdfRoot>>,
) {
    fn spawn_model(
        e: Entity,
        source: &AssetSource,
        pose: &Pose,
        asset_server: &AssetServer,
        commands: &mut Commands,
    ) {
        let mut commands = commands.entity(e);
        commands
            .insert(ModelScene{source: source.clone(), entity: None})
            .insert(SpatialBundle {
                transform: pose.transform(),
                ..default()
            })
            .insert(Category::Model);

        // TODO remove glb hardcoding? might create havoc with supported formats though
        // TODO is there a cleaner way to do this?
        let asset_source = match source {
            AssetSource::Remote(path) => {
                AssetSource::Remote(path.to_owned() + &".glb#Scene0".to_string())
            }
            AssetSource::Local(filename) => {
                // TODO(luca) remove this to make a generic solution for local files
                if filename.ends_with("glb") {
                    AssetSource::Local(filename.to_owned() + &"#Scene0".to_string())
                } else {
                    source.clone()
                }
            }
            AssetSource::Search(name) => {
                AssetSource::Search(name.to_owned() + &".glb#Scene0".to_string())
            }
            AssetSource::Bundled(name) => {
                AssetSource::Bundled(name.to_owned() + &".glb#Scene0".to_string())
            }
            AssetSource::Package(path) => {
                // TODO(luca) support glb here?
                source.clone()
            }
        };
        let handle = asset_server.load_untyped(&String::from(&asset_source));
        commands.insert(PreventDeletion::because(
            "Waiting for model to spawn".to_string()))
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
            let model_id = if scenes.contains(&h.typed_weak::<Scene>()) {
                let model_scene_id = commands.entity(e).add_children(|parent| {
                    let h_typed = h.0.clone().typed::<Scene>();
                    parent
                        .spawn(SceneBundle {
                            scene: h_typed,
                            transform: Transform::from_scale(**scale),
                            ..default()
                        })
                        .id()
                });
                Some(model_scene_id)
            } else if meshes.contains(&h.typed_weak::<Mesh>()) {
                let model_scene_id = commands.entity(e).add_children(|parent| {
                    let h_typed = h.0.clone().typed::<Mesh>();
                    parent
                        .spawn(PbrBundle {
                            mesh: h_typed,
                            material: site_assets.default_mesh_grey_material.clone(),
                            transform: Transform::from_scale(**scale),
                            ..default()
                        })
                        .id()
                });
                Some(model_scene_id)
            } else if urdfs.contains(&h.typed_weak::<UrdfRoot>()) {
                let h_typed = h.0.clone().typed::<UrdfRoot>();
                if let Some(urdf) = urdfs.get(&h_typed) {
                    let model_scene_id = commands.entity(e).add_children(|parent| {
                        parent
                            .spawn(SpatialBundle::VISIBLE_IDENTITY)
                            .insert(urdf.clone())
                            .insert(Category::Workcell)
                            .id()
                    });
                    Some(model_scene_id)
                } else {
                    None
                }
            } else {
                println!("Asset not found!");
                None
            };
            if let Some(id) = model_id {
                commands.entity(e)
                    .insert(ModelSceneRoot)
                    .insert(Selectable::new(e));
                current_scenes.get_mut(e).unwrap().entity = Some(id);
                commands.entity(e).remove::<PendingSpawning>();
            }
        }
    }

    // update changed models
    for (e, source, pose) in changed_models.iter_mut() {
        if let Ok(mut current_scene) = current_scenes.get_mut(e) {
            // Avoid respawning if spurious change detection was triggered
            if current_scene.source != *source {
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
                &mut commands,
            );
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
    model_scene_roots: Query<(Entity, &Selectable), Added<ModelSceneRoot>>,
    all_children: Query<&Children>,
    mesh_handles: Query<&Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    // We use adding of scene root as a marker of models being spawned, the component is added when
    // the scene fininshed loading and is spawned
    for (model_scene_root, selectable) in &model_scene_roots {
        // Use a small vec here to try to dodge heap allocation if possible.
        // TODO(MXG): Run some tests to see if an allocation of 16 is typically
        // sufficient.
        let mut queue: SmallVec<[Entity; 16]> = SmallVec::new();
        queue.push(model_scene_root);

        while let Some(e) = queue.pop() {
            commands
                .entity(e)
                .insert(selectable.clone())
                .insert(DragPlaneBundle::new(selectable.element, Vec3::Z));

            if let Ok(mesh_handle) = mesh_handles.get(e) {
                if let Some(mesh) = mesh_assets.get_mut(mesh_handle) {
                    if mesh.generate_outline_normals().is_err() {
                        println!(
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
