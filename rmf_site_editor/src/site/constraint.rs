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

use crate::interaction::Selectable;
use crate::site::*;
use crate::CurrentWorkspace;
use bevy::prelude::*;
use rmf_site_format::Edge;

// TODO(luca) proper recency ranking, this will break for > 10 drawings
pub const CONSTRAINT_LAYER_START: f32 =
    FLOOR_LAYER_START - (FLOOR_LAYER_START - DRAWING_LAYER_START) / 10.0;
const CONSTRAINT_WIDTH: f32 = 0.2 * LANE_WIDTH;

/// Stores which (child) entity contains the measurement mesh
#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct ConstraintSegment(pub Entity);

// Constraints have to be assigned to levels if both their anchors are on the same level, otherwise
// to the site
// TODO*luca) Implement logic above
pub fn assign_orphan_constraints_to_site(
    mut commands: Commands,
    constraints: Query<(Entity, &Edge<Entity>), (Without<Parent>, With<ConstraintMarker>)>,
    current_workspace: Res<CurrentWorkspace>,
    parents: Query<&Parent>,
    levels: Query<Entity, With<LevelProperties>>,
    open_sites: Query<Entity, With<SiteProperties>>,
) {
    if let Some(current_site) = current_workspace.to_site(&open_sites) {
        for (e, edge) in &constraints {
            commands.entity(current_site).add_child(e);
        }
    }
}

pub fn add_constraint_visuals(
    mut commands: Commands,
    constraints: Query<(Entity, &Edge<Entity>), Added<ConstraintMarker>>,
    anchors: AnchorParams,
    assets: Res<SiteAssets>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
) {
    for (e, edge) in &constraints {
        // TODO(luca) calculate this based on current level, site and anchor parent
        let is_visible = true;

        let mut transform = line_stroke_transform(
            &anchors
                .point_in_parent_frame_of(edge.start(), Category::Constraint, e)
                .unwrap(),
            &anchors
                .point_in_parent_frame_of(edge.end(), Category::Constraint, e)
                .unwrap(),
            CONSTRAINT_WIDTH,
        );
        // TODO(luca) proper layering rather than hardcoded
        transform.translation.z = CONSTRAINT_LAYER_START;

        let child_id = commands
            .spawn(PbrBundle {
                mesh: assets.lane_mid_mesh.clone(),
                material: assets.fiducial_material.clone(),
                transform,
                ..default()
            })
            .insert(Selectable::new(e))
            .id();

        commands
            .entity(e)
            .insert(SpatialBundle {
                transform: Transform::from_translation([0., 0., CONSTRAINT_LAYER_START].into()),
                visibility: Visibility { is_visible },
                ..default()
            })
            .insert(Category::Constraint)
            .insert(ConstraintSegment(child_id))
            .add_child(child_id)
            .insert(EdgeLabels::StartEnd);

        for anchor in &edge.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }
    }
}

fn update_constraint_visual(
    entity: Entity,
    edge: &Edge<Entity>,
    anchors: &AnchorParams,
    transform: &mut Transform,
) {
    let start_anchor = anchors
        .point_in_parent_frame_of(edge.start(), Category::Measurement, entity)
        .unwrap();
    let end_anchor = anchors
        .point_in_parent_frame_of(edge.end(), Category::Measurement, entity)
        .unwrap();
    *transform = line_stroke_transform(&start_anchor, &end_anchor, CONSTRAINT_WIDTH);
    transform.translation.z = CONSTRAINT_LAYER_START;
}

pub fn update_changed_constraint(
    constraints: Query<
        (&Edge<Entity>, &ConstraintSegment),
        (Changed<Edge<Entity>>, With<ConstraintMarker>),
    >,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
) {
    for (edge, segment) in &constraints {
        if let Ok(mut tf) = transforms.get_mut(**segment) {
            update_constraint_visual(**segment, edge, &anchors, tf.as_mut());
        }
    }
}

pub fn update_constraint_for_moved_anchors(
    constraints: Query<(&Edge<Entity>, &ConstraintSegment), With<ConstraintMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut transforms: Query<&mut Transform>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in changed_anchor.iter() {
            if let Some((edge, segment)) = constraints.get(*dependent).ok() {
                if let Ok(mut tf) = transforms.get_mut(**segment) {
                    update_constraint_visual(**segment, edge, &anchors, tf.as_mut());
                }
            }
        }
    }
}
