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

use crate::{interaction::Selectable, site::*, shapes::*};
use bevy::{prelude::*, render::primitives::Aabb};
use rmf_site_format::{Edge, LiftCabin};

#[derive(Clone, Copy, Debug, Component)]
pub struct LiftSegments {
    pub cabin: Entity,
}

fn make_lift_transform(
    reference_anchors: &Edge<Entity>,
    anchors: &Query<(&Anchor, &GlobalTransform)>,
) -> Transform {
    let p_start = Anchor::point_q(reference_anchors.start(), Category::Lift, anchors).unwrap();
    let p_end = Anchor::point_q(reference_anchors.end(), Category::Lift, anchors).unwrap();
    let (p_start, p_end) = if reference_anchors.left() == reference_anchors.right() {
        (p_start, p_start + DEFAULT_CABIN_WIDTH * Vec3::Y)
    } else {
        (p_start, p_end)
    };

    let dp = p_start - p_end;
    let length = dp.length();
    let yaw = (-dp.x).atan2(dp.y);
    let center = (p_start + p_end) / 2.0;

    Transform {
        translation: Vec3::new(center.x, center.y, 0.),
        rotation: Quat::from_rotation_z(yaw),
        ..default()
    }
}

pub fn add_tags_to_lift(
    mut commands: Commands,
    lifts: Query<(Entity, &Edge<Entity>), Added<LiftCabin<Entity>>>,
    mut dependents: Query<&mut AnchorDependents>,
) {
    for (e, edge) in &lifts {
        commands.entity(e)
            .insert(Category::Lift)
            .insert(EdgeLabels::LeftRight);

        for anchor in edge.array() {
            if let Ok(mut dep) = dependents.get_mut(anchor) {
                dep.dependents.insert(e);
            }
        }
    }
}

pub fn update_lift_cabin(
    mut commands: Commands,
    lifts: Query<(
        Entity,
        &LiftCabin<Entity>,
        &LevelDoors<Entity>,
        Option<&LiftSegments>
    ), Or<(Changed<LiftCabin<Entity>>, Changed<LevelDoors<Entity>>)>>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, cabin, level_doors, segments) in &lifts {
        // Despawn the previous cabin
        if let Some(segments) = segments {
            commands.entity(segments.cabin).despawn_recursive();
        }

        match cabin {
            LiftCabin::Rect(params) => {
                let Aabb { center, half_extents } = params.aabb();
                let cabin_tf = Transform::from_translation(Vec3::new(center.x, center.y, 0.));
                let floor_mesh: Mesh = make_flat_rect_mesh(params.depth, params.width).into();
                let wall_mesh: Mesh = params.cabin_wall_coordinates().into_iter().map(
                    |wall| {
                        make_wall_mesh(wall[0], wall[1], params.thickness(), DEFAULT_LEVEL_HEIGHT/3.0)
                    }
                ).fold(MeshBuffer::default(), |sum, next| {
                    sum.merge_with(next)
                }).into();

                let cabin_entity = commands
                    .spawn_bundle(SpatialBundle::from_transform(cabin_tf))
                    .with_children(|parent| {
                        parent
                            .spawn_bundle(PbrBundle {
                                mesh: meshes.add(floor_mesh),
                                material: assets.default_floor_material.clone(),
                                ..default()
                            })
                            .insert(Selectable::new(e));

                        parent
                            .spawn_bundle(PbrBundle {
                                mesh: meshes.add(wall_mesh),
                                material: assets.lift_wall_material.clone(),
                                ..default()
                            })
                            .insert(Selectable::new(e));
                    })
                    .id();

                commands.entity(e)
                    .insert(LiftSegments{ cabin: cabin_entity })
                    .add_child(cabin_entity);
            }
        }
    }
}

pub fn update_lift_edge(
    mut lifts: Query<(Entity, &Edge<Entity>, &mut Transform), (Changed<Edge<Entity>>, With<LiftCabin<Entity>>)>,
    anchors: Query<(&Anchor, &GlobalTransform)>,
) {
    for (e, edge, mut tf) in &mut lifts {
        *tf = make_lift_transform(edge, &anchors);
    }
}

pub fn update_lift_for_moved_anchors(
    mut lifts: Query<(&Edge<Entity>, &mut Transform), With<LiftCabin<Entity>>>,
    anchors: Query<(&Anchor, &GlobalTransform)>,
    changed_anchors: Query<&AnchorDependents, Changed<GlobalTransform>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Ok((edge, mut tf)) = lifts.get_mut(*dependent) {
                *tf = make_lift_transform(edge, &anchors);
            }
        }
    }
}
