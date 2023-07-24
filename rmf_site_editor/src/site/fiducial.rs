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

use crate::interaction::VisualCue;
use crate::{issue::*, site::*, CurrentWorkspace};
use bevy::{prelude::*, utils::Uuid};

pub fn add_fiducial_visuals(
    mut commands: Commands,
    fiducials: Query<(Entity, &Point<Entity>, Option<&Transform>), Added<FiducialMarker>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
) {
    for (e, point, tf) in fiducials.iter() {
        if let Ok(mut deps) = dependents.get_mut(point.0) {
            deps.insert(e);
        }

        if tf.is_none() {
            commands.entity(e).insert(SpatialBundle::VISIBLE_IDENTITY);
        }

        commands
            .entity(e)
            .insert(assets.fiducial_mesh.clone())
            .insert(assets.fiducial_material.clone())
            .insert(Category::Fiducial)
            .insert(VisualCue::outline());
    }
}

pub fn update_changed_fiducial(
    mut fiducials: Query<
        (Entity, &Point<Entity>, &mut Transform),
        (Changed<Point<Entity>>, With<FiducialMarker>),
    >,
    anchors: AnchorParams,
) {
    for (e, point, mut tf) in fiducials.iter_mut() {
        let position = anchors
            .point_in_parent_frame_of(point.0, Category::Fiducial, e)
            .unwrap();
        tf.translation = position;
    }
}

pub fn update_fiducial_for_moved_anchors(
    mut fiducials: Query<(Entity, &Point<Entity>, &mut Transform), With<FiducialMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, point, mut tf)) = fiducials.get_mut(*dependent) {
                let position = anchors
                    .point_in_parent_frame_of(point.0, Category::Fiducial, e)
                    .unwrap();
                tf.translation = position;
            }
        }
    }
}

/// Unique UUID to identify issue of fiducials without names
pub const FIDUCIAL_WITHOUT_LABEL_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x242a655f67cc4d4f9176ed5d64cd87f0u128);

// When triggered by a validation request event, check if there are duplicated door names and
// generate an issue if that is the case
pub fn check_for_fiducials_without_label(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateCurrentWorkspace>,
    current_workspace: Res<CurrentWorkspace>,
    parents: Query<&Parent>,
    fiducial_labels: Query<(Entity, &Label), With<FiducialMarker>>,
) {
    if validate_events.iter().last().is_some() {
        let Some(root) = current_workspace.root else {
            return;
        };
        for (e, label) in &fiducial_labels {
            if AncestorIter::new(&parents, e).any(|p| p == root) {
                if label.is_none() {
                    let issue = Issue {
                        key: IssueKey {
                            entities: [e].into(),
                            kind: FIDUCIAL_WITHOUT_LABEL_ISSUE_UUID,
                        },
                        brief: format!("Fiducial without label found"),
                        hint: "Fiducials names are used by the site editor to map matching \
                            fiducials between different floors or drawings and calculate their \
                            relative transform, fiducials without labels are ignored"
                            .to_string(),
                    };
                    let id = commands.spawn(issue).id();
                    commands.entity(root).add_child(id);
                }
            }
        }
    }
}
