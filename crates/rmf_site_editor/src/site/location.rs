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

use crate::{layers::ZLayer, site::*};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use rmf_site_picking::{Hovered, Select, Selected, VisualCue};

pub const BILLBOARD_LENGTH: f32 = 0.3;
const BILLBOARD_BASE_OFFSET: Vec3 = Vec3::new(0., 0., BILLBOARD_LENGTH / 3. * 0.5);
const BILLBOARD_EMPTY_OFFSET: Vec3 = Vec3::new(0., 0., BILLBOARD_LENGTH * 0.5);
const BILLBOARD_MARGIN: Vec3 = Vec3::new(0., 0., BILLBOARD_LENGTH * 0.9);

#[derive(Component, Clone, Default)]
pub struct BillboardMeshes {
    pub base: Option<Entity>,
    pub charging: Option<Entity>,
    pub holding: Option<Entity>,
    pub parking: Option<Entity>,
    pub mutex_group: Option<Entity>,
    pub empty_billboard: Option<Entity>,
}

#[derive(Component, Clone, Debug)]
pub struct BillboardMarker {
    pub caption_text: Option<String>,
    pub offset: Vec3,
    pub hover_enabled: bool,
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
    mut materials: ResMut<Assets<StandardMaterial>>,
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
            + ZLayer::Location.to_z() * Vec3::Z;

        let only_workcell_tags = !tags.iter().any(|t| !t.is_workcell());
        let mut billboard_meshes = BillboardMeshes::default();

        let base_id = commands.spawn_empty().id();
        commands.entity(e).add_child(base_id);

        if tags.len() == 0 || only_workcell_tags {
            // If no location tags, spawn empty billboard marker
            let new_material = materials
                .get(&assets.empty_billboard_material)
                .unwrap()
                .clone();

            commands.entity(base_id).insert((
                Mesh3d(assets.billboard_mesh.clone()),
                MeshMaterial3d(materials.add(new_material)),
                BillboardMarker {
                    caption_text: None,
                    offset: BILLBOARD_EMPTY_OFFSET,
                    hover_enabled: true,
                },
            ));
            billboard_meshes.empty_billboard = Some(base_id);
        } else {
            // If location tags exist, spawn billboard base
            commands.entity(base_id).insert((
                Mesh3d(assets.billboard_base_mesh.clone()),
                MeshMaterial3d(assets.base_billboard_material.clone()),
                BillboardMarker {
                    caption_text: None,
                    offset: BILLBOARD_BASE_OFFSET,
                    hover_enabled: false,
                },
            ));
            billboard_meshes.base = Some(base_id);
        }

        let mut offset = BILLBOARD_MARGIN - BILLBOARD_BASE_OFFSET;

        for tag in tags.iter() {
            let id = commands.spawn_empty().id();

            let (material_handle, text) = match tag {
                LocationTag::Charger => {
                    billboard_meshes.charging = Some(id);
                    (&assets.charger_material, "charging".to_string())
                }
                LocationTag::ParkingSpot => {
                    billboard_meshes.parking = Some(id);
                    (&assets.parking_material, "parking".to_string())
                }
                LocationTag::HoldingPoint => {
                    billboard_meshes.holding = Some(id);
                    (&assets.holding_point_material, "holding".to_string())
                }
                LocationTag::MutexGroup(name) => {
                    billboard_meshes.mutex_group = Some(id);
                    (&assets.lockpad_material, format!("mutex group: {name}"))
                }
                // Workcells are not visualized
                LocationTag::Workcell(_) => continue,
            };

            // New material instance created for each billboard as the AlphaMode of each billboard is toggled on hover
            let new_material = materials.get(material_handle).unwrap().clone();

            commands.entity(id).insert((
                Mesh3d(assets.billboard_mesh.clone()),
                MeshMaterial3d(materials.add(new_material)),
                BillboardMarker {
                    caption_text: Some(text),
                    offset: offset,
                    hover_enabled: true,
                },
            ));

            commands.entity(e).add_child(id);
            offset += BILLBOARD_MARGIN;
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
            .insert(Category::Location)
            .insert(billboard_meshes)
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
        tf.translation.z = ZLayer::Location.to_z();

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
                tf.translation.z = ZLayer::Location.to_z();
            }
        }
    }
}

pub fn update_location_for_changed_location_tags(
    mut commands: Commands,
    mut select: EventWriter<Select>,
    mut locations: Query<
        (
            Entity,
            &LocationTags,
            &mut BillboardMeshes,
            &mut Hovered,
            &mut Selected,
        ),
        Changed<LocationTags>,
    >,
    mut billboards: Query<&mut BillboardMarker, With<BillboardMarker>>,
    assets: Res<SiteAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, tags, mut billboard_meshes, mut hovered, mut selected) in &mut locations {
        select.write(Select::new(Some(e)));

        let only_workcell_tags = !tags.iter().any(|t| !t.is_workcell());

        // Despawn unused billboards
        if let Some(id) = billboard_meshes.empty_billboard {
            if tags.len() > 0 || !only_workcell_tags {
                commands.entity(id).despawn();
                billboard_meshes.empty_billboard = None;

                hovered.support_hovering.remove(&id);
                selected.support_selected.remove(&id);
            }
        }
        if let Some(id) = billboard_meshes.charging {
            if !tags.iter().any(|t| t.is_charger()) {
                commands.entity(id).despawn();
                billboard_meshes.charging = None;

                hovered.support_hovering.remove(&id);
                selected.support_selected.remove(&id);
            }
        }
        if let Some(id) = billboard_meshes.holding {
            if !tags.iter().any(|t| t.is_holding_point()) {
                commands.entity(id).despawn();
                billboard_meshes.holding = None;

                hovered.support_hovering.remove(&id);
                selected.support_selected.remove(&id);
            }
        }
        if let Some(id) = billboard_meshes.parking {
            if !tags.iter().any(|t| t.is_parking_spot()) {
                commands.entity(id).despawn();
                billboard_meshes.parking = None;

                hovered.support_hovering.remove(&id);
                selected.support_selected.remove(&id);
            }
        }
        if tags.len() == 0 || only_workcell_tags {
            if let Some(id) = billboard_meshes.base {
                commands.entity(id).despawn();
                billboard_meshes.base = None;
            }
        }

        if (tags.len() == 0 || only_workcell_tags) && billboard_meshes.empty_billboard.is_none() {
            // If no location tags exist and no empty billboard marker spawned, spawn empty billboard marker
            let id = commands.spawn_empty().id();
            let new_material = materials
                .get(&assets.empty_billboard_material)
                .unwrap()
                .clone();

            commands.entity(id).insert((
                Mesh3d(assets.billboard_mesh.clone()),
                MeshMaterial3d(materials.add(new_material)),
                BillboardMarker {
                    caption_text: None,
                    offset: BILLBOARD_EMPTY_OFFSET,
                    hover_enabled: true,
                },
            ));
            commands.entity(e).add_child(id);
            billboard_meshes.empty_billboard = Some(id);
        } else if billboard_meshes.base.is_none() {
            // If location tags exist and no billboard base spawned, spawn billboard base
            let id = commands.spawn_empty().id();

            commands.entity(id).insert((
                Mesh3d(assets.billboard_base_mesh.clone()),
                MeshMaterial3d(assets.base_billboard_material.clone()),
                BillboardMarker {
                    caption_text: None,
                    offset: BILLBOARD_BASE_OFFSET,
                    hover_enabled: false,
                },
            ));
            commands.entity(e).add_child(id);
            billboard_meshes.base = Some(id);
        }

        let mut offset = BILLBOARD_MARGIN - BILLBOARD_BASE_OFFSET;

        for tag in tags.iter() {
            let existing_billboard_id = match tag {
                LocationTag::Charger => billboard_meshes.charging,
                LocationTag::HoldingPoint => billboard_meshes.holding,
                LocationTag::ParkingSpot => billboard_meshes.parking,
                LocationTag::MutexGroup(_) => billboard_meshes.mutex_group,
                // Workcells are not visualized
                LocationTag::Workcell(_) => continue,
            };

            // If there exists a spawned billboard for this tag, shift existing billboard
            if let Some(billboard_id) = existing_billboard_id {
                let Ok(mut marker) = billboards.get_mut(billboard_id) else {
                    warn!("Billboard not found");
                    return;
                };
                marker.offset = offset;
                offset += BILLBOARD_MARGIN;

                if let LocationTag::MutexGroup(name) = tag {
                    // Update the caption text in case the mutex group name changed
                    marker.caption_text = Some(format!("mutex group: {name}"));
                }

                continue;
            }

            // There is no existing billboard for this tag, hence spawn new billboard
            let id = commands.spawn_empty().id();

            let (material_handle, text) = match tag {
                LocationTag::Charger => {
                    billboard_meshes.charging = Some(id);
                    (&assets.charger_material, "charging".to_string())
                }
                LocationTag::ParkingSpot => {
                    billboard_meshes.parking = Some(id);
                    (&assets.parking_material, "parking".to_string())
                }
                LocationTag::HoldingPoint => {
                    billboard_meshes.holding = Some(id);
                    (&assets.holding_point_material, "holding".to_string())
                }
                LocationTag::MutexGroup(name) => {
                    billboard_meshes.mutex_group = Some(id);
                    (&assets.lockpad_material, format!("mutex group: {name}"))
                }
                // Workcells are not visualized
                LocationTag::Workcell(_) => continue,
            };

            // New material instance created for each billboard as the AlphaMode of each billboard is toggled on hover
            let new_material = materials.get(material_handle).unwrap().clone();

            commands.entity(id).insert((
                Mesh3d(assets.billboard_mesh.clone()),
                MeshMaterial3d(materials.add(new_material)),
                BillboardMarker {
                    caption_text: Some(text),
                    offset: offset,
                    hover_enabled: true,
                },
            ));

            commands.entity(e).add_child(id);
            offset += BILLBOARD_MARGIN;
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
