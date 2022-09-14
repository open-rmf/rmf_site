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

use bevy::{
    prelude::*,
    asset::LoadState,
};
use rmf_site_format::Model;
use crate::{
    interaction::Selectable,
    deletion::DespawnBlocker,
};
use std::collections::HashMap;
use smallvec::SmallVec;

#[derive(Default, Debug, Clone)]
pub struct LoadingModels(pub HashMap<Entity, (Model, Handle<Scene>)>);

#[derive(Default, Debug, Clone)]
pub struct SpawnedModels(Vec<Entity>);

#[derive(Component, Debug, Clone)]
pub struct ModelScene {
    name: String,
    scene_entity: Option<Entity>,
}

/// A unit component to mark where a scene begins
#[derive(Component, Debug, Clone, Copy)]
pub struct ModelSceneRoot;

pub fn update_models(
    mut commands: Commands,
    added_models: Query<(Entity, &Model), Added<Model>>,
    mut changed_models: Query<(Entity, &Model, &mut Transform), (Changed<Model>, With<Model>)>,
    asset_server: Res<AssetServer>,
    mut loading_models: ResMut<LoadingModels>,
    mut spawned_models: ResMut<SpawnedModels>,
    mut q_current_scene: Query<&mut ModelScene>,
) {
    fn spawn_model(
        e: Entity,
        model: &Model,
        asset_server: &AssetServer,
        commands: &mut Commands,
        loading_models: &mut LoadingModels,
    ) {
        let bundle_path =
            String::from("sandbox://") + &model.kind + &String::from(".glb#Scene0");
        let scene: Handle<Scene> = asset_server.load(&bundle_path);
        commands
            .entity(e)
            .insert(DespawnBlocker)
            .insert(ModelScene{name: model.name.clone(), scene_entity: None});
        loading_models.0.insert(e, (model.clone(), scene.clone()));
    }

    // There is a bug(?) in bevy scenes, which causes panic when a scene is despawned
    // immediately after it is spawned.
    // Work around it by checking the `spawned` container BEFORE updating it so that
    // entities are only despawned at the next frame. This also ensures that entities are
    // "fully spawned" before despawning.
    for e in spawned_models.0.iter() {
        commands.entity(*e).remove::<DespawnBlocker>();
    }
    spawned_models.0.clear();

    // For each model that is loading, check if its scene has finished loading
    // yet. If the scene has finished loading, then insert it as a child of the
    // model entity and make it selectable.
    for (e, (model, h)) in loading_models.0.iter() {
        if asset_server.get_load_state(h) == LoadState::Loaded {
            let model_scene_id = commands
                .entity(*e)
                .insert_bundle(SpatialBundle {
                    transform: model.pose.transform(),
                    ..default()
                })
                .add_children(|parent| {
                    parent.spawn_bundle(SceneBundle {
                        scene: h.clone(),
                        ..default()
                    })
                    .insert(ModelSceneRoot)
                    .insert(Selectable::new(*e))
                    .id()
                });

            q_current_scene.get_mut(*e).unwrap().scene_entity = Some(model_scene_id);
            spawned_models.0.push(*e);
        }
    }

    // for any models whose scenes have finished spawning, remove them from the
    // list of models that are loading
    for e in spawned_models.0.iter() {
        loading_models.0.remove(e);
    }

    // spawn the scenes for any newly added models
    for (e, model) in added_models.iter() {
        spawn_model(e, model, &asset_server, &mut commands, &mut loading_models);
    }

    // update changed models
    for (e, model, mut t) in changed_models.iter_mut() {
        *t = model.pose.transform();
        if let Ok(mut current_scene) = q_current_scene.get_mut(e) {
            if current_scene.name != model.name {
                if let Some(scene_entity) = current_scene.scene_entity {
                    commands.entity(scene_entity).despawn_recursive();
                }
                current_scene.scene_entity = None;
                spawn_model(e, model, &asset_server, &mut commands, &mut loading_models);
            }
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
