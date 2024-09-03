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
    interaction::{Preview, Selectable, VisualCue},
    site::{Dependents, Pending},
};
use bevy::prelude::*;
use rmf_site_format::{
    ModelMarker, NameInSite, NameInWorkcell, NameOfWorkcell, Pose, PrimitiveShape,
};

/// SDFs loaded through site editor wrap all the collisions and visuals into a single Model entity.
/// This doesn't quite work for URDF / workcells since we need to export and edit single visuals
/// and collisions, hence we process the loaded models to flatten them here
pub fn flatten_loaded_model_hierarchy(
    In(old_parent): In<Entity>,
    mut commands: Commands,
    cues: Query<&VisualCue>,
    previews: Query<&Preview>,
    mut poses: Query<&mut Pose>,
    mut dependents: Query<&mut Dependents>,
    parents: Query<&Parent>,
    children: Query<&Children>,
    meshes: Query<(), With<Handle<Mesh>>>,
    models: Query<(), Or<(With<ModelMarker>, With<PrimitiveShape>)>>,
) {
    let Ok(new_parent) = parents.get(old_parent) else {
        warn!(
            "Failed flattening model hierarchy, model {:?} has no parent",
            old_parent
        );
        return;
    };
    let Ok(parent_pose) = poses.get(old_parent).cloned() else {
        return;
    };
    for c in DescendantIter::new(&children, old_parent) {
        if meshes.get(c).is_ok() {
            // Set its selectable to the first parent model, or to itself if none is found
            let mut parent_found = false;
            for p in AncestorIter::new(&parents, c) {
                if models.get(p).is_ok() {
                    commands.entity(c).insert(Selectable::new(p));
                    parent_found = true;
                    break;
                }
            }
            if !parent_found {
                commands.entity(c).insert(Selectable::new(c));
            }
        }
        let Ok(mut child_pose) = poses.get_mut(c) else {
            continue;
        };
        commands.entity(**new_parent).add_child(c);
        if let Ok(mut deps) = dependents.get_mut(**new_parent) {
            deps.insert(c);
        }
        let tf_child = child_pose.transform();
        let tf_parent = parent_pose.transform();
        *child_pose = (tf_parent * tf_child).into();

        // Note: This is wiping out properties that we might try to apply to the
        // original model entity. Because of this, we need to manually push those
        // properties (e.g. VisualCue, Preview) along to the flattened entities.
        // This might not scale well in the long run.
        let mut c_mut = commands.entity(c);
        if let Ok(cue) = cues.get(old_parent) {
            c_mut.insert(cue.clone());
        }
        if let Ok(preview) = previews.get(old_parent) {
            c_mut.insert(preview.clone());
        }
    }
    if let Ok(mut parent_dependents) = dependents.get_mut(**new_parent) {
        parent_dependents.remove(&old_parent);
    }

    // Now despawn the unnecessary model
    commands.entity(old_parent).despawn_recursive();
}
