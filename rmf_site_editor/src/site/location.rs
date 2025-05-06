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

use crate::{animate::Spinning, interaction::VisualCue, site::*};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};

// TODO(@mxgrey): Consider using recency rankings for Locations so they don't
// experience z-fighting.
pub const LOCATION_LAYER_HEIGHT: f32 = LANE_LAYER_LIMIT + SELECTED_LANE_OFFSET;

#[derive(Component, Clone, Default)]
pub struct LocationTagMeshes {
    charger: Option<Entity>,
    parking_spot: Option<Entity>,
    holding_point: Option<Entity>,
}

fn location_halo_tf(tag: &LocationTag) -> Transform {
    let position = match tag {
        LocationTag::Charger => 0,
        LocationTag::ParkingSpot => 1,
        LocationTag::HoldingPoint => 2,
        LocationTag::Workcell(_) => 3,
    };
    Transform {
        translation: Vec3::new(0., 0., 0.01),
        rotation: Quat::from_rotation_z((position as f32 / 6.0 * 360.0).to_radians()),
        ..default()
    }
}

// TODO(@mxgrey): Refactor this implementation with should_display_lane using traits and generics
fn should_display_point(
    point: &Point<Entity>,
    associated: &AssociatedGraphs<Entity>,
    child_of: &Query<&ChildOf>,
    levels: &Query<(), With<LevelElevation>>,
    current_level: &Res<CurrentLevel>,
    graphs: &GraphSelect,
) -> bool {
    if let Ok(child_of) = child_of.get(point.0) {
        if levels.contains(child_of.parent()) && Some(child_of.parent()) != ***current_level {
            return false;
        }
    }

    graphs.should_display(associated)
}

pub fn add_location_visuals(
    mut commands: Commands,
    locations: Query<
        (
            Entity,
            &Point<Entity>,
            &AssociatedGraphs<Entity>,
            &LocationTags,
        ),
        Added<LocationTags>,
    >,
    graphs: GraphSelect,
    anchors: AnchorParams,
    child_of: Query<&ChildOf>,
    levels: Query<(), With<LevelElevation>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
) {
    for (e, point, associated_graphs, tags) in &locations {
        if let Ok(mut deps) = dependents.get_mut(point.0) {
            deps.insert(e);
        }

        let material = graphs.display_style(associated_graphs).0;
        let visibility = if should_display_point(
            point,
            associated_graphs,
            &child_of,
            &levels,
            &current_level,
            &graphs,
        ) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        let position = anchors
            .point_in_parent_frame_of(point.0, Category::Location, e)
            .unwrap()
            + LOCATION_LAYER_HEIGHT * Vec3::Z;

        let mut tag_meshes = LocationTagMeshes::default();
        for tag in tags.iter() {
            let id = commands.spawn_empty().id();
            let material = match tag {
                LocationTag::Charger => {
                    tag_meshes.charger = Some(id);
                    assets.charger_material.clone()
                }
                LocationTag::ParkingSpot => {
                    tag_meshes.parking_spot = Some(id);
                    assets.parking_material.clone()
                }
                LocationTag::HoldingPoint => {
                    tag_meshes.holding_point = Some(id);
                    assets.holding_point_material.clone()
                }
                // Workcells are not visualized
                LocationTag::Workcell(_) => continue,
            };
            commands.entity(id).insert((
                Mesh3d(assets.location_tag_mesh.clone()),
                MeshMaterial3d(material),
                location_halo_tf(tag),
                Visibility::default(),
            ));
            commands.entity(e).add_child(id);
        }

        // TODO(MXG): Put icons on the different visual squares based on the location tags
        commands
            .entity(e)
            .insert((
                Mesh3d(assets.location_mesh.clone()),
                Transform::from_translation(position),
                MeshMaterial3d(material),
                visibility,
            ))
            .insert(Spinning::new(-10.0))
            .insert(Category::Location)
            .insert(tag_meshes)
            .insert(VisualCue::outline());
    }
}

pub fn update_changed_location(
    mut locations: Query<
        (
            Entity,
            &Point<Entity>,
            &AssociatedGraphs<Entity>,
            &mut Visibility,
            &mut Transform,
        ),
        (Changed<Point<Entity>>, Without<NavGraphMarker>),
    >,
    anchors: AnchorParams,
    child_of: Query<&ChildOf>,
    levels: Query<(), With<LevelElevation>>,
    graphs: GraphSelect,
    current_level: Res<CurrentLevel>,
) {
    for (e, point, associated, mut visibility, mut tf) in &mut locations {
        let position = anchors
            .point_in_parent_frame_of(point.0, Category::Location, e)
            .unwrap();
        tf.translation = position;
        tf.translation.z = LOCATION_LAYER_HEIGHT;

        let new_visibility = if should_display_point(
            point,
            associated,
            &child_of,
            &levels,
            &current_level,
            &graphs,
        ) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != new_visibility {
            *visibility = new_visibility;
        }
    }
}

pub fn update_location_for_moved_anchors(
    mut locations: Query<(Entity, &Point<Entity>, &mut Transform), With<LocationTags>>,
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
            if let Ok((e, point, mut tf)) = locations.get_mut(*dependent) {
                let position = anchors
                    .point_in_parent_frame_of(point.0, Category::Location, e)
                    .unwrap();
                tf.translation = position;
                tf.translation.z = LOCATION_LAYER_HEIGHT;
            }
        }
    }
}

pub fn update_location_for_changed_location_tags(
    mut commands: Commands,
    mut locations: Query<(Entity, &LocationTags, &mut LocationTagMeshes), Changed<LocationTags>>,
    assets: Res<SiteAssets>,
) {
    for (e, tags, mut tag_meshes) in &mut locations {
        // Despawn the removed tags first
        if let Some(id) = tag_meshes.charger {
            if !tags.iter().any(|t| t.is_charger()) {
                commands.entity(id).despawn();
                tag_meshes.charger = None;
            }
        }
        if let Some(id) = tag_meshes.parking_spot {
            if !tags.iter().any(|t| t.is_parking_spot()) {
                commands.entity(id).despawn();
                tag_meshes.parking_spot = None;
            }
        }
        if let Some(id) = tag_meshes.holding_point {
            if !tags.iter().any(|t| t.is_holding_point()) {
                commands.entity(id).despawn();
                tag_meshes.holding_point = None;
            }
        }
        // Spawn the new tags
        for tag in tags.iter() {
            let (id, material) = match tag {
                LocationTag::Charger => {
                    if tag_meshes.charger.is_none() {
                        let id = commands.spawn_empty().id();
                        tag_meshes.charger = Some(id);
                        (id, assets.charger_material.clone())
                    } else {
                        continue;
                    }
                }
                LocationTag::ParkingSpot => {
                    if tag_meshes.parking_spot.is_none() {
                        let id = commands.spawn_empty().id();
                        tag_meshes.parking_spot = Some(id);
                        (id, assets.parking_material.clone())
                    } else {
                        continue;
                    }
                }
                LocationTag::HoldingPoint => {
                    if tag_meshes.holding_point.is_none() {
                        let id = commands.spawn_empty().id();
                        tag_meshes.holding_point = Some(id);
                        (id, assets.holding_point_material.clone())
                    } else {
                        continue;
                    }
                }
                // Workcells are not visualized
                LocationTag::Workcell(_) => continue,
            };
            commands.entity(id).insert((
                Mesh3d(assets.location_tag_mesh.clone()),
                MeshMaterial3d(material),
                location_halo_tf(tag),
                Visibility::default(),
            ));
            commands.entity(e).add_child(id);
        }
    }
}

pub fn update_visibility_for_locations(
    mut locations: Query<
        (
            &Point<Entity>,
            &AssociatedGraphs<Entity>,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
            // &mut
        ),
        (With<LocationTags>, Without<NavGraphMarker>),
    >,
    child_of: Query<&ChildOf>,
    levels: Query<(), With<LevelElevation>>,
    current_level: Res<CurrentLevel>,
    graphs: GraphSelect,
    locations_with_changed_association: Query<
        Entity,
        (With<LocationTags>, Changed<AssociatedGraphs<Entity>>),
    >,
    graph_changed_visibility: Query<
        (),
        (
            With<NavGraphMarker>,
            Or<(Changed<Visibility>, Changed<RecencyRank<NavGraphMarker>>)>,
        ),
    >,
    mut removed: RemovedComponents<NavGraphMarker>,
) {
    let graph_change = !graph_changed_visibility.is_empty() || removed.read().next().is_some();
    let update_all = current_level.is_changed() || graph_change;
    if update_all {
        for (point, associated, mut visibility, _) in &mut locations {
            let new_visibility = if should_display_point(
                point,
                associated,
                &child_of,
                &levels,
                &current_level,
                &graphs,
            ) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != new_visibility {
                *visibility = new_visibility;
            }
        }
    } else {
        for e in &locations_with_changed_association {
            if let Ok((point, associated, mut visibility, _)) = locations.get_mut(e) {
                let new_visibility = if should_display_point(
                    point,
                    associated,
                    &child_of,
                    &levels,
                    &current_level,
                    &graphs,
                ) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != new_visibility {
                    *visibility = new_visibility;
                }
            }
        }
    }

    if graph_change {
        for (_, associated_graphs, _, mut m) in &mut locations {
            *m = MeshMaterial3d(graphs.display_style(associated_graphs).0);
        }
    } else {
        for e in &locations_with_changed_association {
            if let Ok((_, associated_graphs, _, mut m)) = locations.get_mut(e) {
                *m = MeshMaterial3d(graphs.display_style(associated_graphs).0);
            }
        }
    }
}

#[derive(Debug, Clone, Event)]
pub struct ConsiderLocationTag {
    pub tag: Option<LocationTag>,
    pub for_element: Entity,
}

impl ConsiderLocationTag {
    pub fn new(tag: Option<LocationTag>, for_element: Entity) -> Self {
        Self { tag, for_element }
    }
}

// TODO(MXG): Consider refactoring into a generic plugin, alongside ConsiderAssociatedGraph
pub fn handle_consider_location_tag(
    mut recalls: Query<&mut RecallLocationTags>,
    mut considerations: EventReader<ConsiderLocationTag>,
) {
    for consider in considerations.read() {
        if let Ok(mut recall) = recalls.get_mut(consider.for_element) {
            recall.consider_tag = consider.tag.clone();
            let r = recall.as_mut();
            if let Some(LocationTag::Workcell(model)) = &r.consider_tag {
                r.consider_tag_asset_source_recall.remember(&model.source);
            }
        }
    }
}
