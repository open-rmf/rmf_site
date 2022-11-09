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

#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct ChildLiftCabinGroup(pub Entity);

#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct ChildCabinAnchorGroup(pub Entity);

#[derive(Clone, Copy, Debug, Component, Default)]
pub struct CabinAnchorGroup;

#[derive(Clone, Copy, Debug, Bundle)]
pub struct CabinAnchorGroupBundle {
    tag: CabinAnchorGroup,
    category: Category,
}

impl Default for CabinAnchorGroupBundle {
    fn default() -> Self {
        Self {
            tag: Default::default(),
            category: Category::Lift,
        }
    }
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
        Option<&ChildCabinAnchorGroup>,
        Option<&ChildLiftCabinGroup>,
    ), Or<(Changed<LiftCabin<Entity>>, Changed<LevelDoors<Entity>>)>>,
    mut cabin_anchor_groups: Query<&mut Transform, With<CabinAnchorGroup>>,
    children: Query<&Children>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, cabin, level_doors, child_anchor_group, child_cabin_group) in &lifts {
        // Despawn the previous cabin
        if let Some(cabin_group) = child_cabin_group {
            commands.entity(cabin_group.0).despawn_recursive();
        }

        let cabin_tf = match cabin {
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
                    .insert(ChildLiftCabinGroup(cabin_entity))
                    .add_child(cabin_entity);

                cabin_tf
            }
        };

        let cabin_anchor_group = if let Some(child_anchor_group) = child_anchor_group {
            Some(**child_anchor_group)
        } else if let Ok(children) = children.get(e) {
            let found_group = children.iter().find(|c| {
                cabin_anchor_groups.contains(**c)
            }).copied();

            if let Some(group) = found_group {
                commands.entity(e).insert(ChildCabinAnchorGroup(group));
            }

            found_group
        } else {
            None
        };

        match cabin_anchor_group {
            Some(group) => {
                *cabin_anchor_groups.get_mut(group).unwrap() = cabin_tf;
            },
            None => {
                let group = commands.entity(e).add_children(
                    |p| p
                        .spawn_bundle(SpatialBundle::from_transform(cabin_tf))
                        .insert_bundle(CabinAnchorGroupBundle::default())
                        .id()
                );
                commands.entity(e).insert(ChildCabinAnchorGroup(group));
            }
        };
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
