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

use crate::site::AnchorBundle;
use bevy::prelude::*;
use rmf_site_format::{Anchor, ConstraintDependents, MeshConstraint, ModelMarker, Pose};

pub fn update_constraint_dependents(
    updated_models: Query<
        (&ConstraintDependents, &Transform),
        (Changed<Transform>, With<ModelMarker>),
    >,
    mut transforms: Query<&mut Transform, Without<ModelMarker>>,
    mesh_constraints: Query<&MeshConstraint<Entity>>,
) {
    // TODO(luca) Add widget for parent reassignment in models, otherwise Changed<Parent> will
    // never trigger
    // When a mesh constraint is added we need to remove the Pose component and
    // set the transform of the entity according to the entity contained in the MeshConstraint
    // component
    for (deps, model_tf) in updated_models.iter() {
        for dep in deps.iter() {
            if let Ok(mut anchor_tf) = transforms.get_mut(*dep) {
                if let Ok(constraint) = mesh_constraints.get(*dep) {
                    // TODO(luca) should relative_pose be relative to model origin instead?
                    // constraint.relative_pose = tf.into();
                    // Set the transform to be a combination of model's and constraint's relative_pose
                    *anchor_tf = *model_tf * constraint.relative_pose.transform();
                }
            }
        }
    }
}

pub fn add_anchors_for_new_mesh_constraints(
    mut commands: Commands,
    changed_constraints: Query<(Entity, &MeshConstraint<Entity>), Changed<MeshConstraint<Entity>>>,
    transforms: Query<&Transform>,
) {
    for (e, constraint) in changed_constraints.iter() {
        if let Ok(model_tf) = transforms.get(constraint.entity) {
            let tf = *model_tf * constraint.relative_pose.transform();
            let pose = Pose::default().align_with(&tf);
            // TODO(luca) is this OK performance wise or should we detect if the component is
            // already present and change its value?
            commands
                .entity(e)
                .insert(AnchorBundle::new(Anchor::Pose3D(pose)));
        }
    }
}
