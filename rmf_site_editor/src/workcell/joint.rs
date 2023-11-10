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

use crate::site::{Delete, Dependents};
use bevy::prelude::*;
use rmf_site_format::{FrameMarker, Joint, JointProperties, NameInWorkcell};

/// Event used  to request the creation of a joint between a parent and a child frame
#[derive(Event)]
pub struct CreateJoint {
    pub parent: Entity,
    pub child: Entity,
    // TODO(luca) Add different properties here such as JointType
}

pub fn handle_create_joint_events(
    mut commands: Commands,
    mut events: EventReader<CreateJoint>,
    mut dependents: Query<&mut Dependents>,
    frames: Query<(), With<FrameMarker>>,
) {
    for req in events.iter() {
        if frames.get(req.parent).is_err() {
            error!(
                "Requested to create a joint with a parent that is not a frame, \
                   this is not valid and will be ignored"
            );
            continue;
        }
        if frames.get(req.child).is_err() {
            error!(
                "Requested to create a joint with a child that is not a frame, \
                   this is not valid and will be ignored"
            );
            continue;
        }
        let joint = Joint {
            name: NameInWorkcell("new_joint".into()),
            properties: JointProperties::Fixed,
        };
        let mut cmd = commands.spawn(Dependents::single(req.child));
        let joint_id = cmd.id();
        joint.add_bevy_components(&mut cmd);
        // Now place the joint between the parent and child in the hierarchy
        commands.entity(req.child).set_parent(joint_id);
        commands.entity(joint_id).set_parent(req.parent);
        if let Ok(mut deps) = dependents.get_mut(req.parent) {
            deps.remove(&req.child);
            deps.insert(joint_id);
        }
    }
}

/// This system cleans up joints which don't have a child anymore because it was despawned
pub fn cleanup_orphaned_joints(
    changed_joints: Query<(Entity, &Children), (Changed<Children>, With<JointProperties>)>,
    mut delete: EventWriter<Delete>,
) {
    for (e, children) in &changed_joints {
        if children.is_empty() {
            delete.send(Delete::new(e));
        }
    }
}
