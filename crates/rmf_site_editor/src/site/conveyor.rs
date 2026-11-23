/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

use crate::{
    interaction::billboard::Billboard,
    recency::RecencyRank,
    site::{
        remove_association_for_deleted_graphs, update_anchor_transforms, Anchor, AnchorBundle,
        AnchorParams, Category, ChangePlugin, Delete, Dependents, EdgeLabels, GraphSelect,
        Original, Pending, SiteAssets, SiteUpdateSet, Subordinate,
    },
    AppState,
};
use bevy::{
    ecs::{hierarchy::ChildOf, relationship::AncestorIter},
    gltf::GltfSceneExtras,
    math::Affine3A,
    prelude::*,
    render::{mesh::VertexAttributeValues, view::RenderLayers},
};
use bevy_rich_text3d::*;
use rmf_site_camera::MODEL_PREVIEW_LAYER;
use rmf_site_format::{
    Affiliation, AssociatedGraphs, ConveyorHeight, ConveyorMarker, ConveyorMetadata, ConveyorPort,
    ConveyorPortDirection, ConveyorThickness, ConveyorWidth, Edge, Group, ModelMarker,
    NavGraphMarker, Scale, SiteID, DEFAULT_CONVEYOR_HEIGHT, DEFAULT_CONVEYOR_THICKNESS,
    DEFAULT_CONVEYOR_WIDTH,
};
use rmf_site_mesh::{make_cuboid, WithOutlineMeshExt};
use rmf_site_picking::{Preview, Selectable};
use serde_json::Value;
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZero,
};
use thiserror::Error;

const CONVEYOR_MID_BELT_THICKNESS: f32 = 0.001;
pub(crate) const CONVEYOR_END_BELT_THICKNESS: f32 = 0.002;
const CONVEYOR_BAR_SIZE: f32 = 0.03;
const CONVEYOR_BAR_ELEVATION: f32 = 0.05;
const CONVEYOR_MARGIN: f32 = 0.0145;
const DEFAULT_CONVEYOR_STAND_HEIGHT: f32 =
    DEFAULT_CONVEYOR_HEIGHT - DEFAULT_CONVEYOR_THICKNESS / 2.0;
const CONVEYOR_PORT_TEXT_OFFSET: f32 = 0.2;
const MAX_CONVEYOR_PORT_SEGMENTS: usize = 1;
const MAX_CONVEYOR_FLOOR_ANCHOR_SEGMENTS: usize = 2;

/// Tracks the flow direction of a conveyor segment derived from connected ports.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConveyorFlow {
    #[default]
    None,
    Forward,
    Backward,
}

pub struct ConveyorPlugin;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ChangePlugin::<ConveyorHeight>::default(),
            ChangePlugin::<ConveyorWidth>::default(),
            ChangePlugin::<ConveyorThickness>::default(),
            ChangePlugin::<ConveyorMetadata>::default(),
            ChangePlugin::<ConveyorPort>::default(),
        ))
        .add_systems(
            PostUpdate,
            add_conveyor_visuals
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
        )
        .add_systems(
            PostUpdate,
            (
                add_conveyor_ports,
                add_conveyor_port_visuals,
                update_conveyor_ports,
                update_conveyor_dimensions,
                update_conveyor_flows,
                update_changed_conveyor,
                update_conveyor_for_moved_anchor,
                update_conveyor_flow_visuals,
            )
                .chain()
                .after(add_conveyor_visuals)
                .after(update_anchor_transforms)
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
        )
        .add_systems(
            PostUpdate,
            update_visibility_for_conveyors
                .after(remove_association_for_deleted_graphs)
                .after(add_conveyor_visuals)
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
        );
    }
}

fn scale_conveyor_uvs(mesh: &mut Mesh, flow: ConveyorFlow) {
    if flow == ConveyorFlow::None {
        return;
    }

    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };

    // Maintain texture square aspect ratio based on the conveyor belt width.
    let uvs: Vec<[f32; 2]> = positions
        .iter()
        .map(|&[x, y, _]| {
            let half_len = x.abs();
            let half_width = y.abs().max(0.0005);
            let u = if y < 0.0 { 1.0 } else { 0.0 };
            let v = match flow {
                ConveyorFlow::Forward => (half_len - x) / (2.0 * half_width),
                ConveyorFlow::Backward => (half_len + x) / (2.0 * half_width),
                ConveyorFlow::None => 0.0,
            };
            [u, v]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

pub(crate) fn make_conveyor_stand_mesh() -> Mesh {
    let y_offset = (DEFAULT_CONVEYOR_WIDTH - CONVEYOR_BAR_SIZE) / 2.0;
    let leg_z = -DEFAULT_CONVEYOR_STAND_HEIGHT / 2.0;
    let bar_z = CONVEYOR_BAR_ELEVATION + CONVEYOR_BAR_SIZE / 2.0 - DEFAULT_CONVEYOR_STAND_HEIGHT;

    let make_leg = |y: f32| {
        make_cuboid(
            CONVEYOR_BAR_SIZE,
            CONVEYOR_BAR_SIZE,
            DEFAULT_CONVEYOR_STAND_HEIGHT,
        )
        .transform_by(Affine3A::from_translation(Vec3::new(0.0, y, leg_z)))
    };
    let bar = make_cuboid(CONVEYOR_BAR_SIZE, DEFAULT_CONVEYOR_WIDTH, CONVEYOR_BAR_SIZE)
        .transform_by(Affine3A::from_translation(Vec3::new(0.0, 0.0, bar_z)));

    Mesh::from(
        make_leg(-y_offset)
            .merge_with(make_leg(y_offset))
            .merge_with(bar),
    )
    .with_generated_outline_normals()
    .unwrap()
}

/// Marker component indicating that a model instance acts as a conveyor node
/// and can have conveyor ports attached or edited in the Inspector.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConveyorNode;

#[derive(Component, Debug, Clone, Copy)]
pub struct ConveyorPortSegments {
    root: Entity,
    arrow: Entity,
    text: Entity,
}

/// Stored on a model description entity acting as a conveyor node so newly
/// spawned instances are registered as conveyor nodes and inherit configured ports.
#[derive(Component, Clone, Debug, Default)]
pub struct ConveyorNodePortConfigs(pub Option<Vec<ConveyorPort>>);

#[derive(Component, Debug, Clone, Copy)]
pub struct ConveyorSegments {
    start: Entity,
    start_belt: Entity,
    mid: Entity,
    mid_belt: Entity,
    end: Entity,
    end_belt: Entity,
    start_stand: Entity,
    end_stand: Entity,
}

struct ConveyorVisuals {
    root: Transform,
    root_global_tf: GlobalTransform,
    start: Transform,
    start_belt: Transform,
    start_stand: Transform,
    mid: Transform,
    mid_belt: Transform,
    mid_mesh: Mesh,
    mid_belt_mesh: Mesh,
    end: Transform,
    end_belt: Transform,
    end_stand: Transform,
}

fn get_conveyor_anchor_position(
    anchor: Entity,
    label: &str,
    conveyor: Entity,
    anchors: &AnchorParams,
) -> Option<Vec3> {
    match anchors.point_in_parent_frame_of(anchor, Category::Conveyor, conveyor) {
        Ok(p) => Some(p),
        Err(_) => {
            error!("Failed to find {label} anchor {anchor:?} for conveyor {conveyor:?}");
            None
        }
    }
}

fn make_conveyor_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    height: f32,
    width: f32,
    thickness: f32,
    flow: ConveyorFlow,
    anchors: &AnchorParams,
    child_of: &Query<&ChildOf>,
    global_transforms: &Query<&GlobalTransform>,
) -> Option<ConveyorVisuals> {
    let start_anchor = get_conveyor_anchor_position(edge.start(), "start", entity, anchors)?;
    let end_anchor = get_conveyor_anchor_position(edge.end(), "end", entity, anchors)?;
    let center = (start_anchor + end_anchor) / 2.0;
    let local_start = (start_anchor - center).with_z(0.0);
    let local_end = (end_anchor - center).with_z(0.0);
    let root = Transform::from_xyz(center.x, center.y, center.z + height);

    // Sync GlobalTransform to align end caps and resize mid mesh within the same frame.
    let parent_global_tf = child_of
        .get(entity)
        .ok()
        .and_then(|c| global_transforms.get(c.parent()).ok())
        .copied()
        .unwrap_or(GlobalTransform::IDENTITY);
    let root_global_tf = parent_global_tf.mul_transform(root);

    let width = width.max(0.001);
    let thickness = thickness.max(0.001);
    let belt_width = (width - CONVEYOR_MARGIN).max(0.001);
    let mid_belt_z = (thickness + CONVEYOR_MID_BELT_THICKNESS) / 2.0;
    let end_belt_z = (thickness + CONVEYOR_END_BELT_THICKNESS) / 2.0;
    let flat_rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);

    let start_to_end = (end_anchor - start_anchor).with_z(0.0);
    let length = start_to_end.length();
    let direction = if length > 0.001 {
        start_to_end / length
    } else {
        Vec3::X
    };
    let rot = Quat::from_rotation_z(start_to_end.y.atan2(start_to_end.x));

    let stand_height = (height - thickness / 2.0).max(0.001);
    let stand_z = stand_height - height;
    let stand_scale = Vec3::new(
        1.0,
        width / DEFAULT_CONVEYOR_WIDTH,
        stand_height / DEFAULT_CONVEYOR_STAND_HEIGHT,
    );
    let offset = (width / 2.0 + CONVEYOR_BAR_SIZE / 2.0).min(length / 2.0);

    let end_transforms = |anchor_pos: Vec3, stand_center: Vec3| {
        let end_cap = Transform::from_xyz(anchor_pos.x, anchor_pos.y, 0.0)
            .with_rotation(flat_rotation)
            .with_scale(Vec3::new(width, thickness, width));
        let belt = Transform::from_xyz(anchor_pos.x, anchor_pos.y, end_belt_z)
            .with_rotation(flat_rotation)
            .with_scale(Vec3::new(belt_width, 1.0, belt_width));
        let stand = Transform::from_xyz(stand_center.x, stand_center.y, stand_z)
            .with_rotation(rot)
            .with_scale(stand_scale);
        (end_cap, belt, stand)
    };

    let (start, start_belt, start_stand) =
        end_transforms(local_start, local_start + direction * offset);
    let (end, end_belt, end_stand) = end_transforms(local_end, local_end - direction * offset);

    let mid = Transform::from_rotation(rot);
    let mid_belt = Transform::from_xyz(0.0, 0.0, mid_belt_z).with_rotation(rot);

    // Bake dimensions into the cuboid mesh to ensure uniform outline normals are generated.
    let mesh_length = length.max(0.001);
    let make_mid_mesh = |width: f32, thickness: f32| {
        Mesh::from(Cuboid::new(mesh_length, width, thickness))
            .with_generated_outline_normals()
            .inspect_err(|err| error!("Failed to generate conveyor outline normals: {err}"))
            .ok()
    };
    let mid_mesh = make_mid_mesh(width, thickness)?;
    let mut mid_belt_mesh = make_mid_mesh(belt_width, CONVEYOR_MID_BELT_THICKNESS)?;
    scale_conveyor_uvs(&mut mid_belt_mesh, flow);

    Some(ConveyorVisuals {
        root,
        root_global_tf,
        start,
        start_belt,
        start_stand,
        mid,
        mid_belt,
        mid_mesh,
        mid_belt_mesh,
        end,
        end_belt,
        end_stand,
    })
}

pub fn add_conveyor_visuals(
    mut commands: Commands,
    new_conveyors: Query<
        (
            Entity,
            &Edge<Entity>,
            &ConveyorHeight,
            &ConveyorWidth,
            &ConveyorThickness,
            &AssociatedGraphs<Entity>,
        ),
        Added<ConveyorMarker>,
    >,
    graphs: GraphSelect,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    child_of: Query<&ChildOf>,
    global_transforms: Query<&GlobalTransform>,
    anchors: AnchorParams,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if new_conveyors.is_empty() {
        return;
    }

    for (e, edge, height, width, thickness, associated_graphs) in &new_conveyors {
        for anchor in &edge.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }

        let Some(visuals) = make_conveyor_visuals(
            e,
            edge,
            height.0,
            width.0,
            thickness.0,
            ConveyorFlow::None,
            &anchors,
            &child_of,
            &global_transforms,
        ) else {
            continue;
        };

        let mut spawn_segment = |tf, mesh, mat| {
            commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    tf,
                    visuals.root_global_tf.mul_transform(tf),
                    Visibility::Inherited,
                    ChildOf(e),
                    Selectable::new(e),
                ))
                .id()
        };

        let mid = spawn_segment(
            visuals.mid,
            meshes.add(visuals.mid_mesh),
            assets.conveyor_metal_material.clone(),
        );
        let mid_belt = spawn_segment(
            visuals.mid_belt,
            meshes.add(visuals.mid_belt_mesh),
            assets.conveyor_belt_material.clone(),
        );

        let mut spawn_end = |end_cap_tf, belt_tf, stand_tf| {
            (
                spawn_segment(
                    end_cap_tf,
                    assets.conveyor_end_mesh.clone(),
                    assets.conveyor_metal_material.clone(),
                ),
                spawn_segment(
                    belt_tf,
                    assets.conveyor_belt_end_mesh.clone(),
                    assets.conveyor_belt_material.clone(),
                ),
                spawn_segment(
                    stand_tf,
                    assets.conveyor_stand_mesh.clone(),
                    assets.conveyor_metal_material.clone(),
                ),
            )
        };
        let (start, start_belt, start_stand) =
            spawn_end(visuals.start, visuals.start_belt, visuals.start_stand);
        let (end, end_belt, end_stand) =
            spawn_end(visuals.end, visuals.end_belt, visuals.end_stand);

        let visibility = if graphs.should_display(associated_graphs) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        commands.entity(e).insert((
            ConveyorSegments {
                start,
                start_belt,
                mid,
                mid_belt,
                end,
                end_belt,
                start_stand,
                end_stand,
            },
            ConveyorFlow::default(),
            visuals.root,
            visuals.root_global_tf,
            visibility,
            Category::Conveyor,
            EdgeLabels::StartEnd,
        ));
    }
}

/// Count how many conveyor segments are connected to an anchor.
pub fn count_connected_conveyors(
    anchor: Entity,
    exclude_edge: Entity,
    conveyor_markers: &Query<(), With<ConveyorMarker>>,
    dependents: &Query<&Dependents, With<Anchor>>,
) -> usize {
    dependents
        .get(anchor)
        .map(|deps| {
            deps.iter()
                .filter(|&&dep| dep != exclude_edge && conveyor_markers.contains(dep))
                .count()
        })
        .unwrap_or(0)
}

#[derive(Debug, Error)]
pub enum ConveyorConnectionError {
    #[error(
        "Invalid selection: Conveyors cannot have more than 2 connected segments at an anchor."
    )]
    TooManySegments,
    #[error("Invalid selection: A conveyor segment already connects these two anchors.")]
    DuplicateSegment,
    #[error("Invalid selection: A conveyor chain cannot be connected to more than 1 'In' port.")]
    MultipleInPorts,
    #[error("Invalid selection: A conveyor chain cannot be connected to more than 1 'Out' port.")]
    MultipleOutPorts,
}

/// Checks whether connecting 2 anchors via a conveyor edge is valid.
/// Returns `Err` if:
/// - `anchor_b` already has 2 connected conveyor segments (would create a fork)
/// - another conveyor segment already directly connects `anchor_a` and `anchor_b`
/// - the resulting conveyor chain would contain >1 'In' port or >1 'Out' port
pub fn validate_conveyor_connection(
    anchor_a: Entity,
    anchor_b: Entity,
    exclude_edge: Entity,
    conveyor_markers: &Query<(), With<ConveyorMarker>>,
    edges: &Query<&mut Edge<Entity>>,
    conveyor_ports: &Query<&ConveyorPort>,
    dependents: &Query<&Dependents, With<Anchor>>,
) -> Result<(), ConveyorConnectionError> {
    // Reject if a port already has >=1 segment or a floor anchor already has >=2 segments
    let max_segments = |anchor: Entity| {
        if conveyor_ports.contains(anchor) {
            MAX_CONVEYOR_PORT_SEGMENTS
        } else {
            MAX_CONVEYOR_FLOOR_ANCHOR_SEGMENTS
        }
    };
    if count_connected_conveyors(anchor_a, exclude_edge, conveyor_markers, dependents)
        >= max_segments(anchor_a)
        || count_connected_conveyors(anchor_b, exclude_edge, conveyor_markers, dependents)
            >= max_segments(anchor_b)
    {
        return Err(ConveyorConnectionError::TooManySegments);
    }

    if anchor_a == anchor_b {
        return Ok(());
    }

    // Traverse the connected conveyor chains from both endpoints to count 'In' and 'Out' ports
    let mut in_ports = 0;
    let mut out_ports = 0;
    let mut visited_anchors = SmallVec::<[Entity; 16]>::new();

    let mut check_port = |anchor: Entity| -> Result<(), ConveyorConnectionError> {
        if let Ok(port) = conveyor_ports.get(anchor) {
            match port.direction {
                ConveyorPortDirection::In => {
                    in_ports += 1;
                    if in_ports > 1 {
                        return Err(ConveyorConnectionError::MultipleInPorts);
                    }
                }
                ConveyorPortDirection::Out => {
                    out_ports += 1;
                    if out_ports > 1 {
                        return Err(ConveyorConnectionError::MultipleOutPorts);
                    }
                }
            }
        }
        Ok(())
    };

    let mut walk_chain = |start_anchor: Entity| -> Result<(), ConveyorConnectionError> {
        if visited_anchors.contains(&start_anchor) {
            return Ok(());
        }
        visited_anchors.push(start_anchor);
        check_port(start_anchor)?;

        let Ok(deps) = dependents.get(start_anchor) else {
            return Ok(());
        };
        for &first_edge in deps.iter() {
            if first_edge == exclude_edge || !conveyor_markers.contains(first_edge) {
                continue;
            }
            let Ok(edge) = edges.get(first_edge) else {
                continue;
            };
            // Ignore stale dependents whose endpoints no longer reference this anchor.
            if edge.start() != start_anchor && edge.end() != start_anchor {
                continue;
            }

            let mut prev_edge = first_edge;
            let mut curr_anchor = if edge.start() == start_anchor {
                edge.end()
            } else {
                edge.start()
            };
            if start_anchor == anchor_a && curr_anchor == anchor_b {
                return Err(ConveyorConnectionError::DuplicateSegment);
            }

            while !visited_anchors.contains(&curr_anchor) {
                visited_anchors.push(curr_anchor);
                check_port(curr_anchor)?;

                let Some(next_step) = dependents.get(curr_anchor).ok().and_then(|d| {
                    d.iter().find_map(|&dep| {
                        if dep == exclude_edge
                            || dep == prev_edge
                            || !conveyor_markers.contains(dep)
                        {
                            return None;
                        }
                        let e = edges.get(dep).ok()?;
                        if e.start() == curr_anchor {
                            Some((dep, e.end()))
                        } else if e.end() == curr_anchor {
                            Some((dep, e.start()))
                        } else {
                            None
                        }
                    })
                }) else {
                    break;
                };

                prev_edge = next_step.0;
                curr_anchor = next_step.1;
            }
        }
        Ok(())
    };

    walk_chain(anchor_a)?;
    walk_chain(anchor_b)
}

/// Checks if changing port's direction results in multiple ports with the
/// same direction in the connected chain
pub fn validate_conveyor_port_direction(
    port_anchor: Entity,
    new_dir: ConveyorPortDirection,
    conveyors: &Query<
        (
            &Edge<Entity>,
            &ConveyorHeight,
            &ConveyorWidth,
            &ConveyorThickness,
            &ConveyorMetadata,
        ),
        With<ConveyorMarker>,
    >,
    ports: &Query<(&ConveyorPort, &ChildOf)>,
    dependents: &Query<&Dependents, With<Anchor>>,
) -> Result<(), ConveyorConnectionError> {
    let mut queue = VecDeque::new();
    queue.push_back(port_anchor);
    let mut visited: SmallVec<[Entity; 16]> = SmallVec::new();
    visited.push(port_anchor);
    while let Some(curr_anchor) = queue.pop_front() {
        let Ok(deps) = dependents.get(curr_anchor) else {
            continue;
        };
        for &dep in deps.iter() {
            let Ok((edge, ..)) = conveyors.get(dep) else {
                continue;
            };
            let Some(next_anchor) = (edge.start() == curr_anchor)
                .then_some(edge.end())
                .or_else(|| (edge.end() == curr_anchor).then_some(edge.start()))
            else {
                continue;
            };
            if visited.contains(&next_anchor) {
                continue;
            }
            visited.push(next_anchor);
            // Stop at port terminals and reject if another port already has `new_dir`.
            if let Ok((p, _)) = ports.get(next_anchor) {
                if p.direction == new_dir {
                    return Err(match new_dir {
                        ConveyorPortDirection::In => ConveyorConnectionError::MultipleInPorts,
                        ConveyorPortDirection::Out => ConveyorConnectionError::MultipleOutPorts,
                    });
                }
            } else {
                queue.push_back(next_anchor);
            }
        }
    }
    Ok(())
}

fn collect_connected_conveyors(
    start_entity: Entity,
    start_edge: &Edge<Entity>,
    get_edge: impl Fn(Entity) -> Option<Edge<Entity>>,
    is_port: impl Fn(Entity) -> bool,
    dependents: &Query<&Dependents, With<Anchor>>,
) -> Vec<(Entity, bool)> {
    // BFS to find the connected conveyor chain and each segment's orientation,
    // and stop at port terminals so separate chains on a port don't bleed together.
    let mut visited_conveyors = vec![(start_entity, true)];
    let mut visited_conveyor_set = HashSet::from([start_entity]);
    let mut anchor_queue = VecDeque::new();
    for (anchor, away_from_start) in [(start_edge.start(), false), (start_edge.end(), true)] {
        if !is_port(anchor) {
            anchor_queue.push_back((anchor, away_from_start));
        }
    }
    let mut visited_anchors = HashSet::from(start_edge.array());

    while let Some((anchor, away_from_start)) = anchor_queue.pop_front() {
        let Ok(deps) = dependents.get(anchor) else {
            continue;
        };
        for &other_entity in deps.iter() {
            if visited_conveyor_set.contains(&other_entity) {
                continue;
            }
            let Some(other_edge) = get_edge(other_entity) else {
                continue;
            };
            // Ignore stale dependents whose endpoints no longer reference this anchor.
            if other_edge.start() != anchor && other_edge.end() != anchor {
                continue;
            }
            visited_conveyor_set.insert(other_entity);
            let same_dir = (other_edge.start() == anchor) == away_from_start;
            visited_conveyors.push((other_entity, same_dir));
            for next_anchor in other_edge.array() {
                if !is_port(next_anchor) && visited_anchors.insert(next_anchor) {
                    anchor_queue.push_back((next_anchor, away_from_start));
                }
            }
        }
    }
    visited_conveyors
}

pub fn update_conveyor_dimensions(
    mut conveyors: Query<
        (
            Entity,
            Ref<Edge<Entity>>,
            &mut ConveyorHeight,
            &mut ConveyorWidth,
            &mut ConveyorThickness,
            &mut ConveyorMetadata,
            Option<&Pending>,
            Option<&Original<Edge<Entity>>>,
        ),
        With<ConveyorMarker>,
    >,
    dependents: Query<&Dependents, With<Anchor>>,
    mut removed_pending: RemovedComponents<Pending>,
    mut removed_original_edges: RemovedComponents<Original<Edge<Entity>>>,
    conveyor_ports: Query<(Ref<ConveyorPort>, &ChildOf)>,
    model_scales: Query<&Scale, (With<ModelMarker>, Without<Group>)>,
) {
    let placed_conveyors: HashSet<Entity> = removed_pending
        .read()
        .chain(removed_original_edges.read())
        .collect();

    let has_changed_port = |edge: &Edge<Entity>| -> bool {
        edge.array()
            .into_iter()
            .any(|e| conveyor_ports.get(e).is_ok_and(|(p, _)| p.is_changed()))
    };

    // Collect conveyors whose endpoints, dimensions, metadata, ports, or placement state changed.
    let mut changed_properties: HashMap<Entity, bool> = HashMap::new();
    let changed_conveyors: Vec<Entity> = conveyors
        .iter_mut()
        .filter_map(|(e, edge, h, w, t, m, _, original)| {
            // Skip conveyors that are mid-replacement.
            if original.is_some() {
                return None;
            }
            let property_changed =
                h.is_changed() || w.is_changed() || t.is_changed() || m.is_changed();
            if property_changed {
                changed_properties.insert(e, t.is_changed() && !h.is_changed());
            }
            (edge.is_changed()
                || property_changed
                || placed_conveyors.contains(&e)
                || has_changed_port(&edge))
            .then_some(e)
        })
        .collect();

    if changed_conveyors.is_empty() {
        return;
    }

    let snap_height_to_port =
        |port_height: f32, thickness: f32| port_height - thickness.max(0.001) / 2.0;
    let get_port = |port_entity: Entity| -> Option<(Ref<ConveyorPort>, f32)> {
        let (port, child_of) = conveyor_ports.get(port_entity).ok()?;
        let scale = model_scales.get(child_of.parent()).ok()?;
        let height = port.location[2] * scale.0.z;
        Some((port, height))
    };
    // Find a port on either endpoint of the edge, prioritizing a newly edited port.
    let find_port = |edge: &Edge<Entity>| {
        let end_port = get_port(edge.end());
        let start_port = get_port(edge.start());
        if end_port.as_ref().is_some_and(|(p, _)| p.is_changed()) {
            end_port
        } else if start_port.as_ref().is_some_and(|(p, _)| p.is_changed()) {
            start_port
        } else {
            end_port.or(start_port)
        }
    };

    let mut visited: HashSet<Entity> = HashSet::new();
    let mut updates: HashMap<Entity, (f32, f32, f32, ConveyorMetadata)> = HashMap::new();

    for e in changed_conveyors {
        let Ok((_, edge, e_h, e_w, e_t, e_m, pending, original)) = conveyors.get(e) else {
            continue;
        };
        if original.is_some() {
            continue;
        }
        if pending.is_none() && visited.contains(&e) {
            continue;
        }

        let start_edge = if pending.is_some() {
            Edge::new(edge.start(), edge.start())
        } else {
            *edge
        };
        let connected_conveyors = collect_connected_conveyors(
            e,
            &start_edge,
            |other_e| {
                let (_, other_edge, _, _, _, _, other_pending, other_original) =
                    conveyors.get(other_e).ok()?;
                (other_pending.is_none() && other_original.is_none()).then_some(*other_edge)
            },
            |a| conveyor_ports.contains(a),
            &dependents,
        );

        if pending.is_some() {
            // If previewing a new conveyor, adopt port dimensions or fall back to defaults
            let mut port = find_port(&edge);
            let mut neighbor_properties = None;
            for &(member, _) in &connected_conveyors {
                if member == e {
                    continue;
                }
                let Ok((_, member_edge, h, w, t, m, ..)) = conveyors.get(member) else {
                    continue;
                };
                port = port.or_else(|| find_port(&member_edge));
                neighbor_properties.get_or_insert((h.0, w.0, t.0, *m));
            }

            let is_point_preview = edge.start() == edge.end();
            let (default_h, default_w) = neighbor_properties
                .filter(|_| is_point_preview)
                .map(|(h, w, _, _)| (h, w))
                .unwrap_or((DEFAULT_CONVEYOR_HEIGHT, DEFAULT_CONVEYOR_WIDTH));

            // Inherit thickness and metadata from a neighbor (or default), and prioritize a
            // connected port's width and top-aligned height before falling back to defaults.
            let target_t = neighbor_properties
                .map(|(_, _, t, _)| t)
                .unwrap_or(DEFAULT_CONVEYOR_THICKNESS);
            let target_w = port
                .as_ref()
                .and_then(|(p, _)| p.width)
                .unwrap_or(default_w);
            let target_h = port
                .as_ref()
                .map(|(_, h)| snap_height_to_port(*h, target_t))
                .unwrap_or(default_h);
            let target_meta = neighbor_properties
                .map(|(_, _, _, m)| m)
                .unwrap_or_default();

            let properties = (target_h, target_w, target_t, target_meta);
            if is_point_preview {
                updates.insert(e, properties);
            } else {
                visited.extend(connected_conveyors.iter().map(|&(m, _)| m));
                updates.extend(
                    connected_conveyors
                        .into_iter()
                        .map(|(m, _)| (m, properties)),
                );
            }
        } else {
            // If it is a placed chain, prioritize values from inspector edits over
            // existing stationary members, falling back to the newly connected segment.
            let (mut active_port, mut fallback_port) = (None, None);
            let (mut edited_properties, mut existing_properties) = (None, None);
            let mut snap_to_port = false;
            let mut thickness_edited = false;

            for &(member, _) in &connected_conveyors {
                let Ok((_, member_edge, h, w, t, m, member_pending, member_original)) =
                    conveyors.get(member)
                else {
                    continue;
                };
                if member_pending.is_some() || member_original.is_some() {
                    continue;
                }
                visited.insert(member);

                let member_edge_changed =
                    member_edge.is_changed() || placed_conveyors.contains(&member);
                let member_changed = member_edge_changed || has_changed_port(&member_edge);
                if member_changed {
                    snap_to_port = true;
                }

                // Prioritize a newly connected or edited port over stationary ports on the chain.
                if let Some(member_port) = find_port(&member_edge) {
                    if member_changed {
                        active_port.get_or_insert(member_port);
                    } else {
                        fallback_port.get_or_insert(member_port);
                    }
                }

                // Ignore newly placed/reconnected segments when picking edited or stationary
                // chain properties, and track thickness edits to keep the top surface aligned.
                if !member_edge_changed {
                    let properties = (h.0, w.0, t.0, *m);
                    if let Some(&t_changed) = changed_properties.get(&member) {
                        edited_properties = Some(properties);
                        thickness_edited = t_changed;
                    }
                    existing_properties.get_or_insert(properties);
                }
            }

            let (mut target_h, mut target_w, target_t, target_meta) = edited_properties
                .or(existing_properties)
                .unwrap_or((e_h.0, e_w.0, e_t.0, *e_m));

            // Snap chain height/width to a connected conveyor port when a connection,
            // port configuration, or belt thickness changes.
            if let Some((ref port, port_h)) = active_port.or(fallback_port) {
                if snap_to_port || thickness_edited {
                    target_h = snap_height_to_port(port_h, target_t);
                }
                if snap_to_port {
                    if let Some(port_w) = port.width {
                        target_w = port_w;
                    }
                }
            }

            let properties = (target_h, target_w, target_t, target_meta);
            updates.extend(
                connected_conveyors
                    .into_iter()
                    .map(|(m, _)| (m, properties)),
            );
        }
    }

    // Apply dimension and metadata updates only when values differ.
    for (entity, (new_h, new_w, new_t, new_m)) in updates {
        if let Ok((_, _, mut height, mut width, mut thickness, mut metadata, ..)) =
            conveyors.get_mut(entity)
        {
            if height.0 != new_h {
                height.0 = new_h;
            }
            if width.0 != new_w {
                width.0 = new_w;
            }
            if thickness.0 != new_t {
                thickness.0 = new_t;
            }
            if *metadata != new_m {
                *metadata = new_m;
            }
        }
    }
}

pub fn update_conveyor_flows(
    mut conveyors: Query<(&Edge<Entity>, &mut ConveyorFlow), With<ConveyorMarker>>,
    changed_conveyors: Query<Entity, (With<ConveyorMarker>, Changed<Edge<Entity>>)>,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Dependents>, Changed<ConveyorPort>)>,
        ),
    >,
    dependents: Query<&Dependents, With<Anchor>>,
    conveyor_ports: Query<&ConveyorPort>,
) {
    let mut visited: HashSet<Entity> = HashSet::new();
    for e in changed_conveyors
        .iter()
        .chain(changed_anchors.iter().flat_map(|deps| deps.iter().copied()))
    {
        if !visited.insert(e) {
            continue;
        }
        let Ok((&edge, mut flow)) = conveyors.get_mut(e) else {
            continue;
        };
        if edge.start() == edge.end() {
            if *flow != ConveyorFlow::None {
                *flow = ConveyorFlow::None;
            }
            continue;
        }

        // Traverse the connected chain to determine its overall flow direction from
        // attached port and assign each segment's flow based on its relative orientation.
        let chain_segments = collect_connected_conveyors(
            e,
            &edge,
            |dep| {
                let (other_edge, _) = conveyors.get(dep).ok()?;
                (other_edge.start() != other_edge.end()).then_some(*other_edge)
            },
            |a| conveyor_ports.contains(a),
            &dependents,
        );

        visited.extend(chain_segments.iter().map(|&(seg, _)| seg));
        let chain_forward = chain_segments.iter().find_map(|&(seg, same_dir)| {
            let (seg_edge, _) = conveyors.get(seg).ok()?;
            [
                (seg_edge.start(), ConveyorPortDirection::Out),
                (seg_edge.end(), ConveyorPortDirection::In),
            ]
            .into_iter()
            .find_map(|(anchor, forward_dir)| {
                let port = conveyor_ports.get(anchor).ok()?;
                Some((port.direction == forward_dir) == same_dir)
            })
        });

        for (seg, same_dir) in chain_segments {
            let new_flow = match chain_forward {
                Some(forward) if forward == same_dir => ConveyorFlow::Forward,
                Some(_) => ConveyorFlow::Backward,
                None => ConveyorFlow::None,
            };
            if let Ok((_, mut flow)) = conveyors.get_mut(seg) {
                if *flow != new_flow {
                    *flow = new_flow;
                }
            }
        }
    }
}

fn update_conveyor_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    segments: &ConveyorSegments,
    height: f32,
    width: f32,
    thickness: f32,
    flow: ConveyorFlow,
    child_of: &Query<&ChildOf>,
    tf_params: &mut ParamSet<(
        (AnchorParams, Query<&GlobalTransform>),
        Query<(&mut Transform, &mut GlobalTransform), Without<Anchor>>,
    )>,
    mesh_handles: &Query<&Mesh3d>,
    meshes: &mut Assets<Mesh>,
) {
    let Some(visuals) = ({
        let (anchors, global_transforms) = tf_params.p0();
        make_conveyor_visuals(
            entity,
            edge,
            height,
            width,
            thickness,
            flow,
            &anchors,
            child_of,
            &global_transforms,
        )
    }) else {
        return;
    };

    for (e, new_mesh) in [
        (segments.mid, visuals.mid_mesh),
        (segments.mid_belt, visuals.mid_belt_mesh),
    ] {
        if let Ok(mesh_handle) = mesh_handles.get(e) {
            if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                *mesh = new_mesh;
            }
        }
    }

    let mut transforms = tf_params.p1();
    if let Ok((mut tf, mut global_tf)) = transforms.get_mut(entity) {
        // Update Transform to ensure that entity is not overwritten with stale
        // local coordinates on next frame
        *tf = visuals.root;

        // Override GlobalTransform to prevent mesh from being rendered at old
        // coordinates during delayed frame propagation
        *global_tf = visuals.root_global_tf;
    }
    for (e, seg_tf) in [
        (segments.start, visuals.start),
        (segments.start_belt, visuals.start_belt),
        (segments.start_stand, visuals.start_stand),
        (segments.mid, visuals.mid),
        (segments.mid_belt, visuals.mid_belt),
        (segments.end, visuals.end),
        (segments.end_belt, visuals.end_belt),
        (segments.end_stand, visuals.end_stand),
    ] {
        if let Ok((mut tf, mut global_tf)) = transforms.get_mut(e) {
            *tf = seg_tf;
            *global_tf = visuals.root_global_tf.mul_transform(seg_tf);
        }
    }
}

pub fn update_conveyor_for_moved_anchor(
    conveyors: Query<
        (
            Entity,
            &Edge<Entity>,
            &ConveyorSegments,
            &ConveyorHeight,
            &ConveyorWidth,
            &ConveyorThickness,
            &ConveyorFlow,
        ),
        With<ConveyorMarker>,
    >,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    child_of: Query<&ChildOf>,
    mut tf_params: ParamSet<(
        (AnchorParams, Query<&GlobalTransform>),
        Query<(&mut Transform, &mut GlobalTransform), Without<Anchor>>,
    )>,
    mesh_handles: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, edge, segments, height, width, thickness, flow)) =
                conveyors.get(*dependent)
            {
                update_conveyor_visuals(
                    e,
                    edge,
                    segments,
                    height.0,
                    width.0,
                    thickness.0,
                    *flow,
                    &child_of,
                    &mut tf_params,
                    &mesh_handles,
                    &mut meshes,
                );
            }
        }
    }
}

pub fn update_changed_conveyor(
    conveyors: Query<
        (
            Entity,
            &Edge<Entity>,
            &ConveyorSegments,
            &ConveyorHeight,
            &ConveyorWidth,
            &ConveyorThickness,
            &ConveyorFlow,
        ),
        (
            With<ConveyorMarker>,
            Or<(
                Changed<Edge<Entity>>,
                Changed<ConveyorHeight>,
                Changed<ConveyorWidth>,
                Changed<ConveyorThickness>,
            )>,
        ),
    >,
    child_of: Query<&ChildOf>,
    mut tf_params: ParamSet<(
        (AnchorParams, Query<&GlobalTransform>),
        Query<(&mut Transform, &mut GlobalTransform), Without<Anchor>>,
    )>,
    mesh_handles: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge, segments, height, width, thickness, flow) in &conveyors {
        update_conveyor_visuals(
            e,
            edge,
            segments,
            height.0,
            width.0,
            thickness.0,
            *flow,
            &child_of,
            &mut tf_params,
            &mesh_handles,
            &mut meshes,
        );
    }
}

pub fn update_conveyor_flow_visuals(
    conveyors: Query<
        (&ConveyorSegments, &ConveyorFlow),
        (With<ConveyorMarker>, Changed<ConveyorFlow>),
    >,
    mesh_handles: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    assets: Res<SiteAssets>,
) {
    for (segments, &flow) in &conveyors {
        let belt_material = match flow {
            ConveyorFlow::None => &assets.conveyor_belt_material,
            ConveyorFlow::Forward | ConveyorFlow::Backward => &assets.conveyor_belt_arrow_material,
        };
        if let Ok(mut mat) = materials.get_mut(segments.mid_belt) {
            if mat.0 != *belt_material {
                mat.0 = belt_material.clone();
            }
        }
        if flow == ConveyorFlow::None {
            continue;
        }
        if let Some(mesh) = mesh_handles
            .get(segments.mid_belt)
            .ok()
            .and_then(|h| meshes.get_mut(&h.0))
        {
            scale_conveyor_uvs(mesh, flow);
        }
    }
}

pub fn update_visibility_for_conveyors(
    mut conveyors: Query<
        (&AssociatedGraphs<Entity>, &mut Visibility),
        (With<ConveyorMarker>, Without<NavGraphMarker>),
    >,
    graphs: GraphSelect,
    conveyors_with_changed_association: Query<
        Entity,
        (With<ConveyorMarker>, Changed<AssociatedGraphs<Entity>>),
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
    let update_visibility =
        |(associated, mut visibility): (&AssociatedGraphs<Entity>, Mut<Visibility>)| {
            let new_visibility = if graphs.should_display(associated) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != new_visibility {
                *visibility = new_visibility;
            }
        };
    if !graph_changed_visibility.is_empty() || removed.read().next().is_some() {
        conveyors.iter_mut().for_each(update_visibility);
    } else {
        for e in &conveyors_with_changed_association {
            if let Ok(conveyor) = conveyors.get_mut(e) {
                update_visibility(conveyor);
            }
        }
    }
}

pub(crate) fn spawn_conveyor_port(
    commands: &mut Commands,
    model_entity: Entity,
    model_global_tf: &GlobalTransform,
    model_scale: Vec3,
    port: ConveyorPort,
) -> Entity {
    let scaled_pos = Vec3::from_array(port.location) * model_scale;
    let anchor = Anchor::Translate2D([scaled_pos.x, scaled_pos.y]);

    commands
        .spawn(AnchorBundle::new(anchor).parent_transform(model_global_tf))
        .insert((Subordinate(Some(model_entity)), port, ChildOf(model_entity)))
        .id()
}

pub fn add_conveyor_ports(
    mut commands: Commands,
    mut delete: EventWriter<Delete>,
    new_scene_extras: Query<(Entity, &GltfSceneExtras), Added<GltfSceneExtras>>,
    child_of: Query<&ChildOf>,
    models: Query<
        (
            Entity,
            Option<&Scale>,
            &GlobalTransform,
            Option<&RenderLayers>,
            &Affiliation<Entity>,
            Option<&Children>,
            Option<&SiteID>,
            Option<&ConveyorNode>,
        ),
        (
            With<ModelMarker>,
            Without<Group>,
            Without<Pending>,
            Without<Preview>,
        ),
    >,
    conveyor_node_port_configs: Query<Ref<ConveyorNodePortConfigs>>,
    mut existing_ports: Query<&mut ConveyorPort>,
) {
    // Sync ports configured on the model description for newly placed instances or updated descriptions
    for (
        model_entity,
        scale,
        model_global_tf,
        render_layers,
        affiliation,
        children,
        _,
        conveyor_node,
    ) in &models
    {
        if render_layers.is_some_and(|r| r.iter().all(|l| l == MODEL_PREVIEW_LAYER)) {
            continue;
        }
        let Some(port_configs) = affiliation
            .0
            .and_then(|desc| conveyor_node_port_configs.get(desc).ok())
        else {
            continue;
        };
        if conveyor_node.is_some() && !port_configs.is_changed() {
            continue;
        }

        commands.entity(model_entity).insert(ConveyorNode);

        let Some(configs) = &port_configs.0 else {
            continue;
        };

        let port_entities: Vec<Entity> = children
            .into_iter()
            .flatten()
            .copied()
            .filter(|&child| existing_ports.contains(child))
            .collect();
        let common_len = port_entities.len().min(configs.len());
        // Update existing ports in place to preserve connected conveyors
        for (&port_entity, port_config) in port_entities[..common_len]
            .iter()
            .zip(&configs[..common_len])
        {
            if let Ok(mut port) = existing_ports.get_mut(port_entity) {
                if *port != *port_config {
                    *port = port_config.clone();
                }
            }
        }
        // Delete excess ports and any conveyors connected to them
        for &port_entity in &port_entities[common_len..] {
            delete.write(Delete::new(port_entity).and_dependents());
        }
        let model_scale = if let Some(s) = scale { s.0 } else { Vec3::ONE };
        // Spawn any newly added ports from the description config
        for port_config in &configs[common_len..] {
            spawn_conveyor_port(
                &mut commands,
                model_entity,
                &model_global_tf,
                model_scale,
                port_config.clone(),
            );
        }
    }

    // Spawn ports defined in embedded GLTF extras when a model scene finishes loading
    for (entity, extras) in &new_scene_extras {
        let Some(ports) = serde_json::from_str::<Value>(&extras.value)
            .ok()
            .and_then(|v| v.get("conveyor_ports").cloned())
            .and_then(|v| serde_json::from_value::<Vec<ConveyorPort>>(v).ok())
        else {
            continue;
        };

        // Find the owning model instance for the spawned scene root
        let Some((
            model_entity,
            scale,
            model_global_tf,
            render_layers,
            affiliation,
            _,
            site_id,
            conveyor_node,
        )) = AncestorIter::new(&child_of, entity).find_map(|ancestor| models.get(ancestor).ok())
        else {
            continue;
        };

        // Skip models rendered only in the offscreen asset preview camera
        if render_layers.is_some_and(|r| r.iter().all(|l| l == MODEL_PREVIEW_LAYER)) {
            continue;
        }

        // If ports were already loaded from the site file, skip spawning from GLTF
        if site_id.is_some() && conveyor_node.is_some() {
            continue;
        }

        // Skip GLTF ports if the model description already has a port configuration
        let has_desc_override = affiliation
            .0
            .and_then(|desc| conveyor_node_port_configs.get(desc).ok())
            .is_some_and(|cfg| cfg.0.is_some());
        if has_desc_override {
            continue;
        }

        commands.entity(model_entity).insert(ConveyorNode);
        if let Some(desc_entity) = affiliation.0 {
            if !conveyor_node_port_configs.contains(desc_entity) {
                commands
                    .entity(desc_entity)
                    .insert(ConveyorNodePortConfigs(None));
            }
        }

        let model_scale = if let Some(s) = scale { s.0 } else { Vec3::ONE };
        for port in ports {
            spawn_conveyor_port(
                &mut commands,
                model_entity,
                &model_global_tf,
                model_scale,
                port,
            );
        }
    }
}

pub fn add_conveyor_port_visuals(
    mut commands: Commands,
    new_ports: Query<(Entity, &ConveyorPort, &ChildOf), Added<ConveyorPort>>,
    models: Query<Option<&Scale>, With<ModelMarker>>,
    assets: Res<SiteAssets>,
) {
    for (e, port, child_of) in &new_ports {
        let model_scale = if let Ok(Some(s)) = models.get(child_of.parent()) {
            s.0
        } else {
            Vec3::ONE
        };
        let z = port.location[2] * model_scale.z;

        let arrow_material = match port.direction {
            ConveyorPortDirection::In => assets.conveyor_port_in_material.clone(),
            ConveyorPortDirection::Out => assets.conveyor_port_out_material.clone(),
        };

        let text = commands
            .spawn((
                Text3d::new(port.direction.label()),
                Text3dStyling {
                    size: 250.0,
                    weight: Weight(900),
                    stroke: NonZero::new(15),
                    color: Srgba::BLACK,
                    stroke_color: Srgba::WHITE,
                    world_scale: Some(Vec2::splat(0.1)),
                    layer_offset: 0.001,
                    align: TextAlign::Center,
                    anchor: TextAnchor::BOTTOM_CENTER,
                    ..Default::default()
                },
                Mesh3d::default(),
                MeshMaterial3d(assets.text3d_material.clone()),
                Transform::from_xyz(0.0, CONVEYOR_PORT_TEXT_OFFSET, 0.0),
            ))
            .id();

        let arrow = commands
            .spawn((
                Mesh3d(assets.conveyor_port_arrow_mesh.clone()),
                MeshMaterial3d(arrow_material),
                Billboard::default(),
            ))
            .add_child(text)
            .id();

        let root = commands
            .spawn((
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::default(),
                ChildOf(e),
            ))
            .add_child(arrow)
            .id();

        commands
            .entity(e)
            .insert(ConveyorPortSegments { root, arrow, text });
    }
}

pub fn update_conveyor_ports(
    models: Query<Ref<Scale>, With<ModelMarker>>,
    mut ports: Query<(
        Entity,
        &ChildOf,
        &mut Anchor,
        &mut ConveyorPort,
        &ConveyorPortSegments,
    )>,
    mut transforms: Query<&mut Transform>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut texts: Query<&mut Text3d>,
    assets: Res<SiteAssets>,
) {
    // Update port anchor, transform, and visual indicators when a port or its model scale changes
    for (port_entity, child_of, mut anchor, mut port, segments) in &mut ports {
        let Ok(scale) = models.get(child_of.parent()) else {
            continue;
        };
        if !port.is_changed() {
            if scale.is_changed() {
                port.set_changed();
            } else {
                continue;
            }
        }
        let scaled_pos = Vec3::from_array(port.location) * scale.0;
        *anchor = Anchor::Translate2D([scaled_pos.x, scaled_pos.y]);
        if let Ok(mut port_tf) = transforms.get_mut(port_entity) {
            port_tf.translation.x = scaled_pos.x;
            port_tf.translation.y = scaled_pos.y;
        }
        if let Ok(mut visual_tf) = transforms.get_mut(segments.root) {
            visual_tf.translation.z = scaled_pos.z;
        }
        let arrow_material = match port.direction {
            ConveyorPortDirection::In => assets.conveyor_port_in_material.clone(),
            ConveyorPortDirection::Out => assets.conveyor_port_out_material.clone(),
        };
        if let Ok(mut mat) = materials.get_mut(segments.arrow) {
            if mat.0 != arrow_material {
                mat.0 = arrow_material;
            }
        }
        let dir_label = port.direction.label();
        if let Ok(mut text) = texts.get_mut(segments.text) {
            if let Some((Text3dSegment::String(s), _)) = text.segments.first_mut() {
                if *s != dir_label {
                    *s = dir_label.to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    use rmf_site_format::Conveyor;

    fn spawn_test_port(world: &mut World, direction: ConveyorPortDirection) -> Entity {
        world
            .spawn((
                Anchor::Translate2D([0.0, 0.0]),
                Dependents::default(),
                ConveyorPort {
                    direction,
                    location: [0.0, 0.0, 0.5],
                    width: None,
                },
            ))
            .id()
    }

    fn spawn_test_edge(world: &mut World, u: Entity, v: Entity) -> Entity {
        let edge = world.spawn(Conveyor::from(Edge::new(u, v))).id();
        world.get_mut::<Dependents>(u).unwrap().insert(edge);
        world.get_mut::<Dependents>(v).unwrap().insert(edge);
        edge
    }

    #[test]
    fn port_direction_validation_terminates_on_closed_chain() {
        let mut world = World::new();
        let model = world.spawn_empty().id();
        let port_p = spawn_test_port(&mut world, ConveyorPortDirection::In);
        world.entity_mut(port_p).insert(ChildOf(model));
        let floor_a = world
            .spawn((Anchor::Translate2D([1.0, 0.0]), Dependents::default()))
            .id();
        let floor_b = world
            .spawn((Anchor::Translate2D([1.0, 1.0]), Dependents::default()))
            .id();

        for (u, v) in [(port_p, floor_a), (floor_a, floor_b), (floor_b, port_p)] {
            spawn_test_edge(&mut world, u, v);
        }

        let mut state: SystemState<(
            Query<
                (
                    &Edge<Entity>,
                    &ConveyorHeight,
                    &ConveyorWidth,
                    &ConveyorThickness,
                    &ConveyorMetadata,
                ),
                With<ConveyorMarker>,
            >,
            Query<(&ConveyorPort, &ChildOf)>,
            Query<&Dependents, With<Anchor>>,
        )> = SystemState::new(&mut world);
        let (conveyors, ports, dependents) = state.get(&world);

        assert!(validate_conveyor_port_direction(
            port_p,
            ConveyorPortDirection::Out,
            &conveyors,
            &ports,
            &dependents,
        )
        .is_ok());
    }

    #[test]
    fn validate_connection_rejects_fork_and_duplicate_ports() {
        let mut world = World::new();
        let a0 = world
            .spawn((Anchor::Translate2D([0.0, 0.0]), Dependents::default()))
            .id();
        let a1 = world
            .spawn((Anchor::Translate2D([1.0, 0.0]), Dependents::default()))
            .id();
        let a2 = world
            .spawn((Anchor::Translate2D([2.0, 0.0]), Dependents::default()))
            .id();
        let a3 = world
            .spawn((Anchor::Translate2D([1.0, 1.0]), Dependents::default()))
            .id();
        spawn_test_edge(&mut world, a0, a1);
        spawn_test_edge(&mut world, a1, a2);
        let fork_preview = world.spawn(Conveyor::from(Edge::new(a3, a3))).id();

        let in1 = spawn_test_port(&mut world, ConveyorPortDirection::In);
        let mid = world
            .spawn((Anchor::Translate2D([3.0, 0.0]), Dependents::default()))
            .id();
        let in2 = spawn_test_port(&mut world, ConveyorPortDirection::In);
        let out = spawn_test_port(&mut world, ConveyorPortDirection::Out);
        spawn_test_edge(&mut world, in1, mid);
        let chain_preview = world.spawn(Conveyor::from(Edge::new(mid, mid))).id();

        let mut state: SystemState<(
            Query<(), With<ConveyorMarker>>,
            Query<&mut Edge<Entity>>,
            Query<&ConveyorPort>,
            Query<&Dependents, With<Anchor>>,
        )> = SystemState::new(&mut world);
        let (markers, edges, ports, dependents) = state.get_mut(&mut world);

        assert!(matches!(
            validate_conveyor_connection(
                a3,
                a1,
                fork_preview,
                &markers,
                &edges,
                &ports,
                &dependents
            ),
            Err(ConveyorConnectionError::TooManySegments)
        ));
        assert!(matches!(
            validate_conveyor_connection(
                mid,
                in2,
                chain_preview,
                &markers,
                &edges,
                &ports,
                &dependents
            ),
            Err(ConveyorConnectionError::MultipleInPorts)
        ));
        assert!(validate_conveyor_connection(
            mid,
            out,
            chain_preview,
            &markers,
            &edges,
            &ports,
            &dependents
        )
        .is_ok());
    }

    #[test]
    fn validate_port_direction_rejects_conflicting_port() {
        let mut world = World::new();
        let model = world.spawn_empty().id();
        let port_out = spawn_test_port(&mut world, ConveyorPortDirection::Out);
        let port_in = spawn_test_port(&mut world, ConveyorPortDirection::In);
        world.entity_mut(port_out).insert(ChildOf(model));
        world.entity_mut(port_in).insert(ChildOf(model));
        let mid = world
            .spawn((Anchor::Translate2D([1.0, 0.0]), Dependents::default()))
            .id();
        spawn_test_edge(&mut world, port_out, mid);
        spawn_test_edge(&mut world, mid, port_in);

        let mut state: SystemState<(
            Query<
                (
                    &Edge<Entity>,
                    &ConveyorHeight,
                    &ConveyorWidth,
                    &ConveyorThickness,
                    &ConveyorMetadata,
                ),
                With<ConveyorMarker>,
            >,
            Query<(&ConveyorPort, &ChildOf)>,
            Query<&Dependents, With<Anchor>>,
        )> = SystemState::new(&mut world);
        let (conveyors, ports, dependents) = state.get(&world);

        assert!(matches!(
            validate_conveyor_port_direction(
                port_out,
                ConveyorPortDirection::In,
                &conveyors,
                &ports,
                &dependents,
            ),
            Err(ConveyorConnectionError::MultipleInPorts)
        ));
        assert!(validate_conveyor_port_direction(
            port_out,
            ConveyorPortDirection::Out,
            &conveyors,
            &ports,
            &dependents,
        )
        .is_ok());
    }

    #[test]
    fn validate_port_direction_checks_all_branches_on_port() {
        let mut world = World::new();
        let model = world.spawn_empty().id();
        let port_p = spawn_test_port(&mut world, ConveyorPortDirection::Out);
        let port_in = spawn_test_port(&mut world, ConveyorPortDirection::In);
        world.entity_mut(port_p).insert(ChildOf(model));
        world.entity_mut(port_in).insert(ChildOf(model));
        let floor_a = world
            .spawn((Anchor::Translate2D([1.0, 0.0]), Dependents::default()))
            .id();

        spawn_test_edge(&mut world, port_p, floor_a);
        spawn_test_edge(&mut world, port_p, port_in);

        let mut state: SystemState<(
            Query<
                (
                    &Edge<Entity>,
                    &ConveyorHeight,
                    &ConveyorWidth,
                    &ConveyorThickness,
                    &ConveyorMetadata,
                ),
                With<ConveyorMarker>,
            >,
            Query<(&ConveyorPort, &ChildOf)>,
            Query<&Dependents, With<Anchor>>,
        )> = SystemState::new(&mut world);
        let (conveyors, ports, dependents) = state.get(&world);

        assert!(matches!(
            validate_conveyor_port_direction(
                port_p,
                ConveyorPortDirection::In,
                &conveyors,
                &ports,
                &dependents,
            ),
            Err(ConveyorConnectionError::MultipleInPorts)
        ));
    }

    #[test]
    fn validate_connection_rejects_second_segment_on_port() {
        let mut world = World::new();
        let port_out = spawn_test_port(&mut world, ConveyorPortDirection::Out);
        let port_in_1 = spawn_test_port(&mut world, ConveyorPortDirection::In);
        let port_in_2 = spawn_test_port(&mut world, ConveyorPortDirection::In);

        spawn_test_edge(&mut world, port_out, port_in_1);
        let preview = world
            .spawn(Conveyor::from(Edge::new(port_out, port_out)))
            .id();

        let mut state: SystemState<(
            Query<(), With<ConveyorMarker>>,
            Query<&mut Edge<Entity>>,
            Query<&ConveyorPort>,
            Query<&Dependents, With<Anchor>>,
        )> = SystemState::new(&mut world);
        let (markers, edges, ports, dependents) = state.get_mut(&mut world);

        assert!(matches!(
            validate_conveyor_connection(
                port_out,
                port_in_2,
                preview,
                &markers,
                &edges,
                &ports,
                &dependents,
            ),
            Err(ConveyorConnectionError::TooManySegments)
        ));
    }
}
