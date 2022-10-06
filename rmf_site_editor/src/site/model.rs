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
    interaction::Selectable,
    site::{Category, PreventDeletion},
};
use bevy::{asset::LoadState, prelude::*};
use rmf_site_format::{Kind, ModelMarker, Pose};
use smallvec::SmallVec;
use std::collections::HashMap;

#[derive(Default, Debug, Clone, Deref, DerefMut)]
pub struct LoadingModels(pub HashMap<Entity, Handle<Scene>>);

#[derive(Default, Debug, Clone)]
pub struct SpawnedModels(Vec<Entity>);

#[derive(Component, Debug, Clone)]
pub struct ModelScene {
    kind: Kind,
    scene_entity: Option<Entity>,
}

/// A unit component to mark where a scene begins
#[derive(Component, Debug, Clone, Copy)]
pub struct ModelSceneRoot;

pub fn update_model_scenes(
    mut commands: Commands,
    mut changed_models: Query<(Entity, &Kind, &Pose), (Changed<Kind>, With<ModelMarker>)>,
    asset_server: Res<AssetServer>,
    mut loading_models: ResMut<LoadingModels>,
    mut spawned_models: ResMut<SpawnedModels>,
    mut current_scenes: Query<&mut ModelScene>,
) {
    fn spawn_model(
        e: Entity,
        kind: &Kind,
        pose: &Pose,
        asset_server: &AssetServer,
        commands: &mut Commands,
        loading_models: &mut LoadingModels,
    ) {
        let mut commands = commands.entity(e);
        commands
            .insert(ModelScene {
                kind: kind.clone(),
                scene_entity: None,
            })
            .insert_bundle(SpatialBundle {
                transform: pose.transform(),
                ..default()
            })
            .insert(Category("Model".to_string()));

        if let Some(kind) = &kind.0 {
            let bundle_path = String::from("rmf-site://") + kind + &".glb#Scene0".to_string();
            let scene: Handle<Scene> = asset_server.load(&bundle_path);
            loading_models.insert(e, scene.clone());
            commands.insert(PreventDeletion::because(
                "Waiting for model to spawn".to_string(),
            ));
        }
    }

    // There is a bug(?) in bevy scenes, which causes panic when a scene is despawned
    // immediately after it is spawned.
    // Work around it by checking the `spawned` container BEFORE updating it so that
    // entities are only despawned at the next frame. This also ensures that entities are
    // "fully spawned" before despawning.
    for e in spawned_models.0.iter() {
        commands.entity(*e).remove::<PreventDeletion>();
    }
    spawned_models.0.clear();

    // For each model that is loading, check if its scene has finished loading
    // yet. If the scene has finished loading, then insert it as a child of the
    // model entity and make it selectable.
    for (e, h) in loading_models.0.iter() {
        if asset_server.get_load_state(h) == LoadState::Loaded {
            let model_scene_id = commands.entity(*e).add_children(|parent| {
                parent
                    .spawn_bundle(SceneBundle {
                        scene: h.clone(),
                        ..default()
                    })
                    .insert(ModelSceneRoot)
                    .insert(Selectable::new(*e))
                    .id()
            });

            current_scenes.get_mut(*e).unwrap().scene_entity = Some(model_scene_id);
            spawned_models.0.push(*e);
        }
    }

    // for any models whose scenes have finished spawning, remove them from the
    // list of models that are loading
    for e in spawned_models.0.iter() {
        loading_models.0.remove(e);
    }

    // update changed models
    for (e, kind, pose) in changed_models.iter_mut() {
        if let Ok(mut current_scene) = current_scenes.get_mut(e) {
            if current_scene.kind != *kind {
                if let Some(scene_entity) = current_scene.scene_entity {
                    commands.entity(scene_entity).despawn_recursive();
                }
                current_scene.scene_entity = None;
                spawn_model(
                    e,
                    kind,
                    pose,
                    &asset_server,
                    &mut commands,
                    &mut loading_models,
                );
            }
        } else {
            // If there isn't a current scene, then we will assume this model
            // is being added for the first time.
            spawn_model(
                e,
                kind,
                pose,
                &asset_server,
                &mut commands,
                &mut loading_models,
            );
        }
    }
}

pub fn make_models_selectable(
    mut commands: Commands,
    model_scene_roots: Query<(Entity, &Selectable), (With<ModelSceneRoot>, Changed<Children>)>,
    all_children: Query<&Children>,
) {
    // If the children of a model scene root changed, then we assume that the
    // scene was just populated with its meshes and that all its children should
    // recursively be made selectable. This might be a fragile assumption if
    // another plugin happens to modify the children of the ModelSceneRoot, so
    // we may want to reconsider this in the future.
    for (model_scene_root, selectable) in &model_scene_roots {
        // Use a small vec here to try to dodge heap allocation if possible.
        // TODO(MXG): Run some tests to see if an allocation of 32 is typically
        // sufficient.
        let mut queue: SmallVec<[Entity; 32]> = SmallVec::new();
        queue.push(model_scene_root);

        while let Some(e) = queue.pop() {
            commands.entity(e).insert(selectable.clone());
            if let Ok(children) = all_children.get(e) {
                for child in children {
                    queue.push(*child);
                }
            }
        }
    }
}
