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

/// Stores which (child) entity contains the constraint mesh
#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct ConstraintSegment(pub Entity);

// Constraints have to be assigned to levels if both their anchors are on the same level, otherwise
// to the site
pub fn assign_orphan_constraints_to_parent(
    mut commands: Commands,
    constraints: Query<(Entity, &Edge<Entity>), (Without<Parent>, With<ConstraintMarker>)>,
    current_workspace: Res<CurrentWorkspace>,
    parents: Query<&Parent>,
    levels: Query<Entity, With<LevelProperties>>,
    open_sites: Query<Entity, With<SiteProperties<Entity>>>,
) {
    if let Some(current_site) = current_workspace.to_site(&open_sites) {
        for (e, edge) in &constraints {
            let start_parent = parents
                .get(edge.start())
                .expect("Failed fetching anchor parent");
            let end_parent = parents
                .get(edge.start())
                .expect("Failed fetching end parent");
            if **start_parent == **end_parent && levels.contains(**start_parent) {
                commands.entity(**start_parent).add_child(e);
            } else {
                commands.entity(current_site).add_child(e);
            }
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
        let transform = line_stroke_transform(
            &anchors
                .point_in_parent_frame_of(edge.start(), Category::Constraint, e)
                .unwrap(),
            &anchors
                .point_in_parent_frame_of(edge.end(), Category::Constraint, e)
                .unwrap(),
            CONSTRAINT_WIDTH,
        );

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

pub fn update_constraint_for_changed_labels(
    mut commands: Commands,
    changed_labels: Query<(&Point<Entity>, &Label), (With<FiducialMarker>, Changed<Label>)>,
    all_labels: Query<(&Point<Entity>, &Label), With<FiducialMarker>>,
    dependents: Query<&Dependents>,
    fiducials: Query<Entity, With<FiducialMarker>>,
    constraints: Query<(Entity, &Edge<Entity>, &Parent), With<ConstraintMarker>>,
    open_sites: Query<Entity, With<SiteProperties<Entity>>>,
) {
    let get_fiducial_label = |e: Entity| -> Option<&Label> {
        let fiducial = dependents
            .get(e)
            .ok()
            .map(|deps| deps.iter().find_map(|d| fiducials.get(*d).ok()))
            .flatten()?;
        all_labels.get(fiducial).map(|(_, label)| label).ok()
    };
    for (p1, l1) in &changed_labels {
        for (e, edge, parent) in &constraints {
            // Ignore constraints between drawings for now, only apply to multilevel ones
            if open_sites.get(parent.get()).is_err() {
                continue;
            }
            // Despawn if labels don't match anymore
            if edge.start() == **p1 || edge.end() == **p1 {
                if let (Some(start_label), Some(end_label)) = (
                    get_fiducial_label(edge.start()),
                    get_fiducial_label(edge.end()),
                ) {
                    if start_label != end_label {
                        commands.entity(e).despawn_recursive();
                    }
                }
            }
        }
        for (p2, l2) in &all_labels {
            if p2 == p1 {
                continue;
            }
            if l1.is_some() && l1 == l2 {
                // Make sure there isn't a constraint already between the two points
                // A constraint is a dependent of both anchors so we only need to check one
                if dependents
                    .get(**p1)
                    .ok()
                    .and_then(|deps| Some(deps.iter().filter_map(|d| constraints.get(*d).ok())))
                    .and_then(|mut constraints| {
                        constraints.find(|c| c.1.start() == **p2 || c.1.end() == **p2)
                    })
                    .is_none()
                {
                    commands.spawn(Constraint::from(Edge::from([**p1, **p2])));
                }
            }
        }
    }
}
