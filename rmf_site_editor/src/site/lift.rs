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

use crate::{interaction::Selectable, shapes::*, site::*, CurrentWorkspace};
use bevy::{prelude::*, render::primitives::Aabb};
use rmf_site_format::{Edge, LiftCabin};
use std::collections::BTreeSet;

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
    #[allow(dead_code)]
    Entity(Entity),
    RectFace(RectFace),
}

#[derive(Clone, Copy, Debug, Component)]
pub struct LiftDoormat {
    pub for_lift: Entity,
    pub on_level: Entity,
    pub cabin_door: CabinDoorId,
    pub door_available: bool,
}

impl LiftDoormat {
    pub fn toggle_availability(&self) -> ToggleLiftDoorAvailability {
        ToggleLiftDoorAvailability {
            for_lift: self.for_lift,
            on_level: self.on_level,
            cabin_door: self.cabin_door,
            door_available: !self.door_available,
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
    entity: Entity,
    reference_anchors: &Edge<Entity>,
    anchors: &AnchorParams,
) -> Transform {
    let p_start = anchors
        .point_in_parent_frame_of(reference_anchors.start(), Category::Lift, entity)
        .unwrap();
    let p_end = anchors
        .point_in_parent_frame_of(reference_anchors.end(), Category::Lift, entity)
        .unwrap();
    let (p_start, p_end) = if reference_anchors.left() == reference_anchors.right() {
        (p_start, p_start - DEFAULT_CABIN_WIDTH * Vec3::Y)
    } else {
        (p_start, p_end)
    };

    let dp = p_start - p_end;
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
    new_lifts: Query<(Entity, &Edge<Entity>), Added<LiftCabin<Entity>>>,
    orphan_lifts: Query<Entity, (With<LiftCabin<Entity>>, Without<Parent>)>,
    open_sites: Query<Entity, With<SiteProperties>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    for (e, edge) in &new_lifts {
        let mut lift_cmds = commands.entity(e);
        lift_cmds
            .insert(SpatialBundle::default())
            .insert(EdgeLabels::LeftRight)
            .insert(Category::Lift);

        if orphan_lifts.contains(e) {
            // Assume that a newly created lift that doesn't have a parent
            // belongs in whatever the current site happens to be.
            if let Some(current_site) = current_workspace.to_site(&open_sites) {
                commands.entity(current_site).add_child(e);
            } else {
                error!("Could not find a current site to put a newly created lift inside of!");
            }
        }

        for anchor in edge.array() {
            if let Ok(mut deps) = dependents.get_mut(anchor) {
                deps.insert(e);
            }
        }
    }
}

pub fn update_lift_cabin(
    mut commands: Commands,
    lifts: Query<
        (
            Entity,
            &LiftCabin<Entity>,
            Option<&RecallLiftCabin<Entity>>,
            Option<&ChildCabinAnchorGroup>,
            Option<&ChildLiftCabinGroup>,
            &Parent,
        ),
        Or<(Changed<LiftCabin<Entity>>, Changed<Parent>)>,
    >,
    mut cabin_anchor_groups: Query<&mut Transform, With<CabinAnchorGroup>>,
    level_visits: Query<&LevelVisits<Entity>>,
    children: Query<&Children>,
    doors: Query<&Edge<Entity>, With<LiftCabinDoorMarker>>,
    mut anchors: Query<&mut Anchor>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    levels: Query<(Entity, &Parent), With<LevelProperties>>,
) {
    for (e, cabin, recall, child_anchor_group, child_cabin_group, site) in &lifts {
        // Despawn the previous cabin
        if let Some(cabin_group) = child_cabin_group {
            commands.entity(cabin_group.0).despawn_recursive();
        }

        let cabin_tf = match cabin {
            LiftCabin::Rect(params) => {
                let Aabb { center, .. } = params.aabb();
                let cabin_tf = Transform::from_translation(Vec3::new(center.x, center.y, 0.));
                let floor_mesh: Mesh = make_flat_rect_mesh(
                    params.depth + 2.0 * params.thickness(),
                    params.width + 2.0 * params.thickness(),
                )
                .into();
                let wall_mesh: Mesh = params
                    .cabin_wall_coordinates()
                    .into_iter()
                    .map(|wall| {
                        make_wall_mesh(
                            wall[0],
                            wall[1],
                            params.thickness(),
                            DEFAULT_LEVEL_HEIGHT / 3.0,
                        )
                    })
                    .fold(MeshBuffer::default(), |sum, next| sum.merge_with(next))
                    .into();

                let cabin_entity = commands
                    .spawn(SpatialBundle::from_transform(cabin_tf))
                    .with_children(|parent| {
                        parent
                            .spawn(PbrBundle {
                                mesh: meshes.add(floor_mesh),
                                material: assets.lift_floor_material.clone(),
                                ..default()
                            })
                            .insert(Selectable::new(e));

                        parent
                            .spawn(PbrBundle {
                                mesh: meshes.add(wall_mesh),
                                material: assets.lift_wall_material.clone(),
                                ..default()
                            })
                            .insert(Selectable::new(e));

                        for (level, level_site) in &levels {
                            if level_site.get() != site.get() {
                                continue;
                            }

                            for (face, door, mut aabb) in params.level_doormats(0.3, recall) {
                                let door_available = door
                                    .filter(|d| {
                                        level_visits
                                            .get(*d)
                                            .ok()
                                            .unwrap_or(&LevelVisits::default())
                                            .contains(&level)
                                    })
                                    .is_some();
                                aabb.center.z = LANE_LAYER_LIMIT;
                                let mesh = make_flat_mesh_for_aabb(aabb);
                                parent
                                    .spawn(PbrBundle {
                                        mesh: meshes.add(mesh.into()),
                                        // Doormats are not visible by default.
                                        // Other plugins should make them visible
                                        // if using them as a visual cue.
                                        visibility: Visibility { is_visible: false },
                                        ..default()
                                    })
                                    .insert(LiftDoormat {
                                        for_lift: e,
                                        on_level: level,
                                        cabin_door: CabinDoorId::RectFace(face),
                                        door_available,
                                    });
                            }
                        }
                    })
                    .id();

                commands
                    .entity(e)
                    .insert(ChildLiftCabinGroup(cabin_entity))
                    .add_child(cabin_entity);

                // Update transforms for door anchors
                for face in RectFace::iter_all() {
                    if let (Some(p), Some(new_edge)) =
                        (params.door(face), params.level_door_anchors(face))
                    {
                        if let Ok(edge) = doors.get(p.door) {
                            for (a, new_anchor) in
                                edge.array().into_iter().zip(new_edge.into_iter())
                            {
                                if let Ok(mut anchor) = anchors.get_mut(a) {
                                    *anchor = new_anchor;
                                }
                            }
                        }
                    }
                }

                cabin_tf
            }
        };

        let cabin_anchor_group = if let Some(child_anchor_group) = child_anchor_group {
            Some(**child_anchor_group)
        } else if let Ok(children) = children.get(e) {
            let found_group = children
                .iter()
                .find(|c| cabin_anchor_groups.contains(**c))
                .copied();

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
            }
            None => {
                let group = commands.entity(e).add_children(|p| {
                    p.spawn(SpatialBundle::from_transform(cabin_tf))
                        .insert(CabinAnchorGroupBundle::default())
                        .id()
                });
                commands.entity(e).insert(ChildCabinAnchorGroup(group));
            }
        };
    }
}

pub fn update_lift_edge(
    mut lifts: Query<
        (Entity, &Edge<Entity>, &mut Transform),
        (Changed<Edge<Entity>>, With<LiftCabin<Entity>>),
    >,
    anchors: AnchorParams,
) {
    for (e, edge, mut tf) in &mut lifts {
        *tf = make_lift_transform(e, edge, &anchors);
    }
}

pub fn update_lift_for_moved_anchors(
    mut lifts: Query<(Entity, &Edge<Entity>, &mut Transform), With<LiftCabin<Entity>>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
) {
    for changed_anchor in &changed_anchors {
        for dependent in changed_anchor.iter() {
            if let Ok((e, edge, mut tf)) = lifts.get_mut(*dependent) {
                *tf = make_lift_transform(e, edge, &anchors);
            }
        }
    }
}

pub fn update_lift_door_availability(
    mut commands: Commands,
    mut toggles: EventReader<ToggleLiftDoorAvailability>,
    mut lifts: Query<(
        &mut LiftCabin<Entity>,
        Option<&RecallLiftCabin<Entity>>,
        &ChildCabinAnchorGroup,
    )>,
    mut doors: Query<(Entity, &Edge<Entity>, &mut LevelVisits<Entity>), With<LiftCabinDoorMarker>>,
    dependents: Query<&Dependents, With<Anchor>>,
    current_level: Res<CurrentLevel>,
    new_levels: Query<(), Added<LevelProperties>>,
    all_levels: Query<(), With<LevelProperties>>,
    removed_levels: RemovedComponents<LevelProperties>,
    parents: Query<&Parent>,
) {
    for toggle in toggles.iter() {
        let (mut cabin, recall_cabin, anchor_group) = match lifts.get_mut(toggle.for_lift) {
            Ok(lift) => lift,
            Err(_) => continue,
        };

        if toggle.door_available {
            if !all_levels.contains(toggle.on_level) {
                // If we're being asked to toggle availability on for something
                // that isn't a level, then ignore this request.
                error!(
                    "Asking to turn on lift {:?} door {:?} availability \
                    for a level {:?} that does not exist.",
                    toggle.for_lift, toggle.cabin_door, toggle.on_level,
                );
                continue;
            }
            let cabin_door = match toggle.cabin_door {
                CabinDoorId::Entity(e) => e,
                CabinDoorId::RectFace(face) => {
                    match cabin.as_mut() {
                        LiftCabin::Rect(params) => {
                            if let Some(cabin_door) = params.door(face).map(|p| p.door) {
                                cabin_door
                            } else if let Some(old_cabin_door) =
                                recall_cabin.map(|r| r.rect_door(face).as_ref()).flatten()
                            {
                                // A cabin door used to exist but was removed by
                                // the user in the past. We should revive it
                                // instead of creating a whole new one.
                                *params.door_mut(face) = Some(old_cabin_door.clone());
                                old_cabin_door.door
                            } else {
                                // Create a new door with new anchors
                                let new_door = commands.spawn_empty().id();
                                *params.door_mut(face) = Some(LiftCabinDoorPlacement::new(
                                    new_door,
                                    params.width.min(params.depth) / 2.0,
                                ));
                                let anchors =
                                    params.level_door_anchors(face).unwrap().map(|anchor| {
                                        commands
                                            .spawn(AnchorBundle::new(anchor))
                                            .insert(Subordinate(Some(toggle.for_lift)))
                                            .id()
                                    });

                                for anchor in anchors {
                                    commands.entity(**anchor_group).add_child(anchor);
                                }

                                commands
                                    .entity(new_door)
                                    .insert(LiftCabinDoor {
                                        kind: DoorType::DoubleSliding(DoubleSlidingDoor::default()),
                                        reference_anchors: anchors.into(),
                                        visits: LevelVisits(BTreeSet::from_iter([toggle.on_level])),
                                        marker: Default::default(),
                                    })
                                    .insert(Dependents::single(toggle.for_lift));
                                commands.entity(toggle.for_lift).add_child(new_door);

                                new_door
                            }
                        } //_ => continue,
                    }
                }
            };

            if let Ok((_, _, mut visits)) = doors.get_mut(cabin_door) {
                visits.insert(toggle.on_level);
                if let Some(current_level) = **current_level {
                    commands.entity(cabin_door).insert(Visibility {
                        is_visible: visits.contains(&current_level),
                    });
                }
            }

            commands.entity(cabin_door).remove::<Pending>();

            if let Ok((_, existing_anchors, _)) = doors.get(cabin_door) {
                // Make sure visibility is turned on for the anchors and
                // the Pending is removed.
                for anchor in existing_anchors.array() {
                    commands
                        .entity(anchor)
                        .remove::<Pending>()
                        .insert(Visibility { is_visible: true });
                }
            }
        } else {
            let cabin_door = match toggle.cabin_door {
                CabinDoorId::Entity(e) => Some(e),
                CabinDoorId::RectFace(face) => match &*cabin {
                    LiftCabin::Rect(params) => params.door(face).map(|p| p.door),
                    //_ => None,
                },
            };

            // If the cabin door that's being removed cannot be found then there
            // is nothing for us to do on this loop.
            let cabin_door = match cabin_door {
                Some(e) => e,
                None => continue,
            };

            let need_to_remove_door = if let Ok((_, _, mut visits)) = doors.get_mut(cabin_door) {
                visits.remove(&toggle.on_level);
                if let Some(current_level) = **current_level {
                    commands.entity(cabin_door).insert(Visibility {
                        is_visible: visits.contains(&current_level),
                    });
                }
                visits.is_empty()
            } else {
                false
            };

            if need_to_remove_door {
                remove_door(
                    cabin_door,
                    &mut commands,
                    cabin.as_mut(),
                    &doors,
                    &dependents,
                );
            }

            // This is a silly hack to dirty the change tracker for this
            // lift and force it to be refreshed in the update_lift_cabin
            // system. We do that so the lift door doormats can be updated
            // to reflect their new state. When time allows, it would be worth
            // considering a more efficient strategy for updating the doormats.
            cabin.set_changed();
        }
    }

    if current_level.is_changed() {
        // Loop through all the cabin doors to check if their visibility needs
        // to change.
        if let Some(current_level) = **current_level {
            for (e, _, visits) in &doors {
                commands.entity(e).insert(Visibility {
                    is_visible: visits.contains(&current_level),
                });
            }
        }
    }

    if !new_levels.is_empty() {
        // A silly dirty hack to force lift cabins to update their doormats
        // when a new level is added.
        for (mut cabin, _, _) in &mut lifts {
            cabin.set_changed();
        }
    }

    for removed_level in removed_levels.iter() {
        // When a level is removed, we should clear it from all visitation
        // information and redo the cabin rendering.
        let mut doors_to_remove = Vec::new();
        for (e_door, _, mut visits) in &mut doors {
            let mut need_to_remove_door = false;
            if visits.remove(&removed_level) {
                if visits.is_empty() {
                    need_to_remove_door = true;
                }
            }

            if need_to_remove_door {
                doors_to_remove.push(e_door);
            }
        }

        for e_door in doors_to_remove {
            let e_lift = match parents.get(e_door) {
                Ok(e_lift) => e_lift,
                Err(_) => {
                    error!(
                        "Unable to find parent for lift door \
                        {e_door:?} while handling a removed level"
                    );
                    continue;
                }
            };
            let (mut cabin, _, _) = match lifts.get_mut(e_lift.get()) {
                Ok(cabin) => cabin,
                Err(_) => {
                    error!("Unable to find cabin for lift {e_lift:?}");
                    continue;
                }
            };
            remove_door(e_door, &mut commands, cabin.as_mut(), &doors, &dependents);
        }
    }
}

fn remove_door(
    cabin_door: Entity,
    commands: &mut Commands,
    cabin: &mut LiftCabin<Entity>,
    doors: &Query<(Entity, &Edge<Entity>, &mut LevelVisits<Entity>), With<LiftCabinDoorMarker>>,
    dependents: &Query<&Dependents, With<Anchor>>,
) {
    cabin.remove_door(cabin_door);
    commands
        .entity(cabin_door)
        .insert(Pending)
        .insert(Visibility { is_visible: false });

    // Clear out the anchors if nothing besides the cabin door depends on them
    let remove_anchors = if let Ok((_, anchors, _)) = doors.get(cabin_door) {
        let mut remove_anchors = true;
        'outer: for anchor in anchors.array() {
            if let Ok(deps) = dependents.get(anchor) {
                for dependent in deps.iter() {
                    if *dependent != cabin_door {
                        remove_anchors = false;
                        break 'outer;
                    }
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
        for anchor in anchors.array() {
            commands
                .entity(anchor)
                .insert(Pending)
                .insert(Visibility { is_visible: false });
        }
    }
}
