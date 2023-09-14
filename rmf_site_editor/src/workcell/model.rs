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

use crate::site::{Dependents, ModelTrashcan, Pending};
use bevy::prelude::*;
use rmf_site_format::{
    ModelMarker, NameInSite, NameInWorkcell, NameOfWorkcell, Pose, PrimitiveShape,
};

/// SDFs loaded through site editor wrap all the collisions and visuals into a single Model entity.
/// This doesn't quite work for URDF / workcells since we need to export and edit single visuals
/// and collisions, hence we process the loaded models to flatten them here
pub fn flatten_loaded_models_hierarchy(
    mut commands: Commands,
    new_models: Query<
        (Entity, &Parent),
        (
            Without<Pending>,
            Or<(Added<ModelMarker>, Added<PrimitiveShape>)>,
        ),
    >,
    all_model_parents: Query<(Entity, &Parent), (With<ModelMarker>, Without<Pending>)>,
    mut poses: Query<&mut Pose>,
    mut dependents: Query<&mut Dependents>,
    parents: Query<&Parent>,
    trashcan: Res<ModelTrashcan>,
) {
    for (e, parent) in &new_models {
        // Traverse up the hierarchy to find the first model parent and reassign it
        if let Some((parent_entity, model_parent)) =
            AncestorIter::new(&parents, **parent).find_map(|e| all_model_parents.get(e).ok())
        {
            let Ok(parent_pose) = poses.get(parent_entity).cloned() else {
                continue;
            };
            let Ok(mut child_pose) = poses.get_mut(e) else {
                continue;
            };
            if let Ok(mut parent_dependents) = dependents.get_mut(**model_parent) {
                parent_dependents.remove(&parent_entity);
            }
            commands.entity(**model_parent).add_child(e);
            if let Ok(mut deps) = dependents.get_mut(**model_parent) {
                deps.insert(e);
            }
            for (mut t1, t2) in child_pose.trans.iter_mut().zip(parent_pose.trans.iter()) {
                *t1 += t2;
            }
            // Now despawn the unnecessary model
            commands.entity(parent_entity).set_parent(trashcan.0);
        }
    }
}

pub fn replace_name_in_site_components(
    mut commands: Commands,
    new_names: Query<(Entity, &NameInSite), Added<NameInSite>>,
    workcells: Query<(), With<NameOfWorkcell>>,
    parents: Query<&Parent>,
) {
    for (e, name) in &new_names {
        if AncestorIter::new(&parents, e).any(|p| workcells.get(p).is_ok()) {
            commands
                .entity(e)
                .insert(NameInWorkcell(name.0.clone()))
                .remove::<NameInSite>();
        }
    }
}
