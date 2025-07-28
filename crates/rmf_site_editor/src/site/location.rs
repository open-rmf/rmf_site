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

use crate::site::*;
use bevy::{color::palettes, ecs::hierarchy::ChildOf, prelude::*};
use rmf_site_picking::VisualCue;

// TODO(@mxgrey): Consider using recency rankings for Locations so they don't
// experience z-fighting.
pub const LOCATION_LAYER_HEIGHT: f32 = LANE_LAYER_LIMIT + SELECTED_LANE_OFFSET;

#[derive(Component, Clone, Default)]
pub struct BillboardMeshes {
    point: Vec3,
    base_billboard: Option<Entity>,

    charging_billboard: Option<Entity>,
    holding_billboard: Option<Entity>,
    parking_billboard: Option<Entity>,

    charging_text: Option<Entity>,
    holding_text: Option<Entity>,
    parking_text: Option<Entity>,

    charging_hover_mesh: Option<Entity>,
    holding_hover_mesh: Option<Entity>,
    parking_hover_mesh: Option<Entity>,
}
#[derive(Component, Clone, Debug)]
pub struct BillboardTextMarker;

#[derive(Component, Clone, Debug)]
pub struct BillboardMarker {
    pub caption_entity: Entity,
    pub offset: Vec3,
    pub pivot: Vec3,
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
    mut meshes: ResMut<Assets<Mesh>>,
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

        let mut billboard_meshes = BillboardMeshes::default();
        billboard_meshes.point = position;

        // initialise billboard values
        let billboard_length = 0.3;
        let billboard_scale = 0.01;
        let mut mesh_location_y = billboard_length * 0.9 * 2.;
        let mut text_location_y = -12.;
        let mesh_sphere_radius = billboard_length * 0.9 * 0.5 / billboard_scale;

        // if location tags exist, spawn billboard base
        if tags.iter().count() > 0 {
            let base_billboard_id = commands.spawn_empty().id();
                commands.entity(base_billboard_id)
                    .insert((
                        BillboardText::default(),
                        TextLayout::new_with_justify(JustifyText::Center),
                        Transform::from_scale(Vec3::splat(billboard_scale)),
                        Visibility::default(),
                    )).with_children(|parent| {
                        parent.spawn((
                            BillboardTexture(assets.base_billboard_texture.clone()),
                            BillboardMesh(meshes.add(Rectangle::new(billboard_length, billboard_length))),
                            BillboardPivotOffset(Vec2::new(0., mesh_location_y)),
                        ));
                    });

            commands.entity(e).add_child(base_billboard_id);
            billboard_meshes.base_billboard = Some(base_billboard_id);
            mesh_location_y -= billboard_length * 0.9;
        }

        for tag in tags.iter() {
            let mesh_id = commands.spawn_empty().id();
            let text_id = commands.spawn_empty().id();
            let hover_mesh_id = commands.spawn_empty().id();

            let (texture, text) = match tag {
                LocationTag::Charger => {
                    billboard_meshes.charging_billboard = Some(mesh_id);
                    billboard_meshes.charging_text = Some(text_id);
                    billboard_meshes.charging_hover_mesh = Some(hover_mesh_id);
                    (assets.charging_billboard_texture.clone(), "charging")
                }
                LocationTag::ParkingSpot => {
                    billboard_meshes.parking_billboard = Some(mesh_id);
                    billboard_meshes.parking_text = Some(text_id);
                    billboard_meshes.parking_hover_mesh = Some(hover_mesh_id);
                    (assets.parking_billboard_texture.clone(), "parking")
                }
                LocationTag::HoldingPoint => {
                    billboard_meshes.holding_billboard = Some(mesh_id);
                    billboard_meshes.holding_text = Some(text_id);
                    billboard_meshes.holding_hover_mesh = Some(hover_mesh_id);
                    (assets.holding_billboard_texture.clone(), "holding")
                }
                // Workcells are not visualized
                LocationTag::Workcell(_) => continue,
            };

            commands.entity(text_id)
                .insert((
                    BillboardText::default(),
                    TextLayout::new_with_justify(JustifyText::Left),
                    Transform::from_scale(Vec3::splat(billboard_scale)),
                    BillboardPivotOffset(Vec2::new(-20., text_location_y)),
                    Visibility::Hidden,
                    BillboardTextMarker,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextSpan::new(text),
                        TextFont::from_font_size(8.0),
                        TextColor(Color::Srgba(palettes::css::WHITE)),
                    )); 
                });
            
            commands.entity(mesh_id)
                .insert((
                    BillboardText::default(),
                    TextLayout::new_with_justify(JustifyText::Center),
                    Transform::from_scale(Vec3::splat(billboard_scale)),
                    Visibility::default(),
                )).with_children(|parent| {
                    parent.spawn((
                        BillboardTexture(texture),
                        BillboardMesh(meshes.add(Rectangle::new(billboard_length, billboard_length))),
                        BillboardPivotOffset(Vec2::new(0., mesh_location_y)),
                    ));
                });
            
            //spawn hover mesh
            commands.entity(hover_mesh_id)
                .insert((
                    Transform::from_scale(Vec3::splat(billboard_scale)),
                    Mesh3d(meshes.add(Sphere::new(mesh_sphere_radius))),
                    BillboardMarker {
                        caption_entity: text_id,
                        pivot: position,
                        offset: position - Vec3::new(0., 0., mesh_location_y - mesh_sphere_radius * billboard_scale * 3.8),
                    },
                ));

            commands.entity(e).add_child(text_id);
            commands.entity(e).add_child(mesh_id);
            commands.entity(e).add_child(hover_mesh_id);

            mesh_location_y -= billboard_length * 0.9;
            text_location_y -= mesh_sphere_radius;
            
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
    mut billboards: Query<(Entity, &LocationTags, &mut BillboardMeshes), Changed<LocationTags>>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {

    for (e, tags, mut billboard_meshes) in &mut billboards {
        //Despawn all billboards
        if let Some(id) = billboard_meshes.charging_billboard {
            commands.entity(id).despawn();
            billboard_meshes.charging_billboard = None;
        }
        if let Some(id) = billboard_meshes.holding_billboard {
            commands.entity(id).despawn();
            billboard_meshes.holding_billboard = None;
        }
        if let Some(id) = billboard_meshes.parking_billboard {
            commands.entity(id).despawn();
            billboard_meshes.parking_billboard = None;
        }
        //despawn all texts
        if let Some(id) = billboard_meshes.charging_text {
            commands.entity(id).despawn();
            billboard_meshes.charging_text = None;
        }
        if let Some(id) = billboard_meshes.holding_text {
            commands.entity(id).despawn();
            billboard_meshes.holding_text = None;
        }
        if let Some(id) = billboard_meshes.parking_text {
            commands.entity(id).despawn();
            billboard_meshes.parking_text = None;
        }
        //despawn all hover meshes
        if let Some(id) = billboard_meshes.charging_hover_mesh {
            commands.entity(id).despawn();
            billboard_meshes.charging_hover_mesh = None;
        }
        if let Some(id) = billboard_meshes.holding_hover_mesh {
            commands.entity(id).despawn();
            billboard_meshes.holding_hover_mesh = None;
        }
        if let Some(id) = billboard_meshes.parking_hover_mesh {
            commands.entity(id).despawn();
            billboard_meshes.parking_hover_mesh = None;
        }
        if let Some(id) = billboard_meshes.base_billboard {
            commands.entity(id).despawn();
            billboard_meshes.base_billboard = None;
        }

        //initialise billboard values
        let billboard_length = 0.3;
        let billboard_scale = 0.01;
        let mut mesh_location_y = billboard_length * 0.9 * 2.;
        let mut text_location_y = -12.;
        let mesh_sphere_radius = billboard_length * 0.9 * 0.5 / billboard_scale;

        let position = billboard_meshes.point;

        // if location tags exist, spawn billboard base
        if tags.iter().count() > 0 {
            let base_billboard_id = commands.spawn_empty().id();
                commands.entity(base_billboard_id)
                    .insert((
                        BillboardText::default(),
                        TextLayout::new_with_justify(JustifyText::Center),
                        Transform::from_scale(Vec3::splat(billboard_scale)),
                        Visibility::default(),
                    )).with_children(|parent| {
                        parent.spawn((
                            BillboardTexture(assets.base_billboard_texture.clone()),
                            BillboardMesh(meshes.add(Rectangle::new(billboard_length, billboard_length))),
                            BillboardPivotOffset(Vec2::new(0., mesh_location_y)),
                        ));
                    });
            
            commands.entity(e).add_child(base_billboard_id);
            billboard_meshes.base_billboard = Some(base_billboard_id);
            mesh_location_y -= billboard_length * 0.9;
        }

        for tag in tags.iter() {
            let mesh_id = commands.spawn_empty().id();
            let text_id = commands.spawn_empty().id();
            let hover_mesh_id = commands.spawn_empty().id();

            let (texture, text) = match tag {
                LocationTag::Charger => {
                    billboard_meshes.charging_billboard = Some(mesh_id);
                    billboard_meshes.charging_text = Some(text_id);
                    billboard_meshes.charging_hover_mesh = Some(hover_mesh_id);
                    (assets.charging_billboard_texture.clone(), "charging")
                }
                LocationTag::ParkingSpot => {
                    billboard_meshes.parking_billboard = Some(mesh_id);
                    billboard_meshes.parking_text = Some(text_id);
                    billboard_meshes.parking_hover_mesh = Some(hover_mesh_id);
                    (assets.parking_billboard_texture.clone(), "parking")
                }
                LocationTag::HoldingPoint => {
                    billboard_meshes.holding_billboard = Some(mesh_id);
                    billboard_meshes.holding_text = Some(text_id);
                    billboard_meshes.holding_hover_mesh = Some(hover_mesh_id);
                    (assets.holding_billboard_texture.clone(), "holding")
                }
                // Workcells are not visualized
                LocationTag::Workcell(_) => continue,
            };

            commands.entity(text_id)
                .insert((
                    BillboardText::default(),
                    TextLayout::new_with_justify(JustifyText::Left),
                    Transform::from_scale(Vec3::splat(billboard_scale)),
                    BillboardPivotOffset(Vec2::new(-20., text_location_y)),
                    Visibility::Hidden,
                    BillboardTextMarker,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextSpan::new(text),
                        TextFont::from_font_size(8.0),
                        TextColor(Color::Srgba(palettes::css::WHITE)),
                    )); 
                });
            
            commands.entity(mesh_id)
                .insert((
                    BillboardText::default(),
                    TextLayout::new_with_justify(JustifyText::Center),
                    Transform::from_scale(Vec3::splat(billboard_scale)),
                    Visibility::default(),
                )).with_children(|parent| {
                    parent.spawn((
                        BillboardTexture(texture),
                        BillboardMesh(meshes.add(Rectangle::new(billboard_length, billboard_length))),
                        BillboardPivotOffset(Vec2::new(0., mesh_location_y)),
                    ));
                });
            
            //spawn hover mesh
            commands.entity(hover_mesh_id)
                .insert((
                    Transform::from_scale(Vec3::splat(billboard_scale)),
                    Mesh3d(meshes.add(Sphere::new(mesh_sphere_radius))),
                    BillboardMarker {
                        caption_entity: text_id,
                        pivot: position,
                        offset: position - Vec3::new(0., 0., mesh_location_y - mesh_sphere_radius * billboard_scale * 3.8),
                    },
                ))
                .insert(VisualCue::no_outline());

            commands.entity(e).add_child(text_id);
            commands.entity(e).add_child(mesh_id);
            commands.entity(e).add_child(hover_mesh_id);

            mesh_location_y -= billboard_length * 0.9;
            text_location_y -= mesh_sphere_radius;
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
