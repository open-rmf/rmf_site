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
use std::collections::{BTreeSet, btree_map::Entry};

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

#[derive(Clone, Copy, Debug)]
pub enum CabinDoorId {
    Entity(Entity),
    RectFace(RectFace),
}

#[derive(Clone, Copy, Debug, Component)]
pub struct LiftDoorPlacemat {
    pub for_lift: Entity,
    pub on_level: Entity,
    pub cabin_door: CabinDoorId,
    pub door_available: bool,
}

impl LiftDoorPlacemat {
    pub fn toggle_availability(&self) -> ToggleLiftDoorAvailability {
        ToggleLiftDoorAvailability {
            for_lift: self.for_lift,
            on_level: self.on_level,
            cabin_door: self.cabin_door,
            door_available: !self.door_available
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ToggleLiftDoorAvailability {
    pub for_lift: Entity,
    pub on_level: Entity,
    pub cabin_door: CabinDoorId,
    pub door_available: bool,
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
        Option<&RecallLiftCabin<Entity>>,
        &LevelDoors<Entity>,
        Option<&ChildCabinAnchorGroup>,
        Option<&ChildLiftCabinGroup>,
        &Parent,
    ), Or<(Changed<LiftCabin<Entity>>, Changed<LevelDoors<Entity>>)>>,
    mut cabin_anchor_groups: Query<&mut Transform, With<CabinAnchorGroup>>,
    children: Query<&Children>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    levels: Query<(Entity, &Parent), With<LevelProperties>>,
    current_level: Res<CurrentLevel>,
) {
    for (e, cabin, recall, level_doors, child_anchor_group, child_cabin_group, site) in &lifts {
        // Despawn the previous cabin
        if let Some(cabin_group) = child_cabin_group {
            commands.entity(cabin_group.0).despawn_recursive();
        }

        let cabin_tf = match cabin {
            LiftCabin::Rect(params) => {
                let Aabb { center, half_extents } = params.aabb();
                let cabin_tf = Transform::from_translation(Vec3::new(center.x, center.y, 0.));
                let floor_mesh: Mesh = make_flat_rect_mesh(
                    params.depth + 2.0*params.thickness(),
                    params.width + 2.0*params.thickness(),
                ).into();
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

                        for (level, level_site) in &levels {
                            if level_site.get() != site.get() {
                                continue;
                            }

                            for (face, door, mut aabb) in params.level_door_placemats(0.3, recall) {
                                let door_available = door.filter(
                                    |d| level_doors.visit.get(&level)
                                        .unwrap_or(&BTreeSet::new()).contains(&d)
                                ).is_some();
                                aabb.center.z = PASSIVE_LANE_HEIGHT/2.0;
                                let mesh = make_flat_mesh_for_aabb(aabb);
                                parent
                                    .spawn_bundle(PbrBundle{
                                        mesh: meshes.add(mesh.into()),
                                        // Placemats are not visible by default.
                                        // Other plugins should make them visible
                                        // if using them as a visual cue.
                                        visibility: Visibility{ is_visible: false },
                                        ..default()
                                    })
                                    .insert(LiftDoorPlacemat {
                                        for_lift: e,
                                        on_level: level,
                                        cabin_door: CabinDoorId::RectFace(face),
                                        door_available,
                                    });
                            }
                        }
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

pub fn update_lift_door_availability(
    mut commands: Commands,
    mut toggles: EventReader<ToggleLiftDoorAvailability>,
    mut lifts: Query<(&mut LevelDoors<Entity>, &mut LiftCabin<Entity>, Option<&RecallLiftCabin<Entity>>, &ChildCabinAnchorGroup)>,
    dependents: Query<&AnchorDependents>,
) {
    for toggle in toggles.iter() {
        let (mut level_doors, mut cabin, mut recall, anchor_group) = match lifts.get_mut(toggle.for_lift) {
            Ok(lift) => lift,
            Err(_) => continue,
        };

        if toggle.door_available {
            let cabin_door = match toggle.cabin_door {
                CabinDoorId::Entity(e) => e,
                CabinDoorId::RectFace(face) => {
                    match cabin.as_mut() {
                        LiftCabin::Rect(params) => {
                            if let Some(cabin_door) = params.door(face).map(|p| p.door)
                            {
                                cabin_door
                            } else if let Some(old_cabin_door) = recall.map(|r| r.rect_door(face).as_ref()).flatten() {
                                // A cabin door used to exist but was removed by
                                // the user in the past. We should revive it
                                // instead of creating a whole new one.
                                *params.door_mut(face) = Some(old_cabin_door.clone());
                                old_cabin_door.door
                            } else {
                                // Create a new door
                                let new_door = commands
                                    .spawn_bundle(LiftCabinDoor {
                                        kind: DoubleSlidingDoor::default().into(),
                                        marker: Default::default(),
                                    })
                                    .id();
                                commands.entity(toggle.for_lift).add_child(new_door);

                                *params.door_mut(face) = Some(LiftCabinDoorPlacement::new(
                                    new_door, params.width.min(params.depth)/2.0
                                ));
                                new_door
                            }
                        },
                        _ => continue,
                    }
                }
            };

            if !level_doors.reference_anchors.contains_key(&cabin_door) {
                let anchors = cabin.level_door_anchors(cabin_door).unwrap().map(
                    |anchor| {
                        commands
                            .spawn_bundle(AnchorBundle::new(anchor))
                            .insert(PreventDeletion::because("used for a lift level door".to_string()))
                            .id()
                    });
                for anchor in &anchors {
                    commands.entity(anchor_group.0).add_child(*anchor);
                }
                level_doors.reference_anchors.insert(cabin_door, anchors.into());
            }

            level_doors.visit.entry(toggle.on_level).or_default().insert(cabin_door);
        } else {
            let cabin_door = match toggle.cabin_door {
                CabinDoorId::Entity(e) => Some(e),
                CabinDoorId::RectFace(face) => {
                    match &*cabin {
                        LiftCabin::Rect(params) => params.door(face).map(|p| p.door),
                        _ => None,
                    }
                }
            };

            // If the cabin door that's being removed cannot be found then there
            // is nothing for us to do on this loop.
            let cabin_door = match cabin_door {
                Some(e) => e,
                None => continue,
            };

            match level_doors.visit.entry(toggle.on_level) {
                Entry::Occupied(mut doors) => {
                    doors.get_mut().remove(&cabin_door);
                },
                _ => { }
            }

            // Check if there are no floors using this door anymore. If there
            // aren't then remove this cabin door from the lift.
            let remove_door = {
                let mut keep_door = false;
                'outer: for (_, doors) in &level_doors.visit {
                    for door in doors {
                        if *door == cabin_door {
                            keep_door = true;
                            break 'outer;
                        }
                    }
                }
                !keep_door
            };

            if remove_door {
                cabin.remove_door(cabin_door);

                // Clear out the anchors if nothing depends on them
                let remove_anchors = if let Some(anchors) = level_doors.reference_anchors.get(&cabin_door) {
                    let mut remove_anchors = true;
                    for anchor in anchors.array() {
                        if let Ok(deps) = dependents.get(anchor) {
                            if !deps.dependents.is_empty() {
                                remove_anchors = false;
                            }
                        }
                    }

                    if remove_anchors {
                        Some(*anchors)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(anchors) = remove_anchors {
                    level_doors.reference_anchors.remove(&cabin_door);
                    for anchor in anchors.array() {
                        commands.entity(anchor).despawn_recursive();
                    }
                }
            }
        }
    }
}

pub fn update_anchors_for_level_doors(
    mut commands: Commands,
    changed_lifts: Query<(Entity, &LevelDoors<Entity>), Changed<LevelDoors<Entity>>>,
) {
    for (lift, level_doors) in &changed_lifts {
        for (_, edge) in &level_doors.reference_anchors {
            for anchor in edge.array() {
                commands.entity(anchor).insert(Subordinate(Some(lift)));
            }
        }
    }
}
