/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use crate::{interaction::IntersectGroundPlaneParams, layers::ZLayer, site::*};
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemState},
    math::Affine3A,
    prelude::*,
    render::{mesh::PrimitiveTopology, render_asset::RenderAssetUsages},
};
use geo::{
    geometry::{LineString, MultiPolygon, Polygon},
    BooleanOps, CoordsIter, TriangulateSpade,
};
use rmf_site_format::{
    compute_polygon_centroid_2d, resolve_guide_preference, Angle, AssociatedGraphs, Edge,
    GridCellCoord, GridCellIndex, NavCell, NavCellEdge, NavCellEdgeMarker, NavCellMarker, NavGrid,
    NavGuidePreference, Path, PibtGuidePreference, PibtNavGraph, PibtPolygon, PreferredDirection,
    RecallPreferredDirection,
};
use rmf_site_mesh::*;
use rmf_site_picking::{PickBlockStatus, Select, Selectable, Selected, Selection};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub const NAV_CELL_EDGE_WIDTH: f32 = 0.08;

#[derive(Default)]
pub struct PolygonNavPlugin;

impl Plugin for PolygonNavPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CellBoxSelectState>()
            .add_event::<SpawnNavGrid>()
            .add_event::<ExpandNavGrid>()
            .add_event::<SnapAndStitchCells>()
            .add_event::<MergeSelectedCells>()
            .add_event::<ConnectSelectedCells>()
            .add_event::<ExportPibtNavGraph>()
            .add_plugins((
                ChangePlugin::<PreferredDirection>::default(),
                RecallPlugin::<RecallPreferredDirection>::default(),
                ChangePlugin::<NavGuidePreference>::default(),
                ChangePlugin::<NavGrid>::default(),
                ChangePlugin::<GridCellCoord<Entity>>::default(),
            ))
            .add_systems(
                Update,
                (
                    handle_spawn_nav_grid,
                    handle_expand_nav_grid,
                    handle_snap_and_stitch_cells,
                    handle_merge_selected_cells,
                    handle_connect_selected_cells,
                    handle_export_pibt_nav_graph,
                    update_cell_box_selection.run_if(crate::AppState::in_displaying_mode()),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    assign_orphan_elements_to_level::<NavCellMarker>,
                    assign_orphan_elements_to_level::<NavCellEdgeMarker>,
                    assign_orphan_elements_to_level::<NavGrid>,
                )
                    .run_if(crate::AppState::in_displaying_mode())
                    .in_set(SiteUpdateSet::AssignOrphans),
            )
            .add_systems(
                PostUpdate,
                (
                    add_nav_cell_visuals,
                    update_nav_cells,
                    update_nav_cells_for_moved_anchors,
                    add_nav_cell_edge_visuals,
                    update_nav_cell_edges,
                )
                    .run_if(crate::AppState::in_displaying_mode())
                    .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
            );
    }
}

/// Stores the child mesh entity for a `NavCell`.
#[derive(Component, Debug, Clone, Copy)]
pub struct NavCellVisuals {
    pub mesh: Entity,
}

/// Stores the child mesh entity for a `NavCellEdge`.
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut)]
pub struct NavCellEdgeVisuals(pub Entity);

/// Direction in which to expand an existing `NavGrid` or extrude selected `NavCell`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridExpansionDirection {
    /// Add row on the North (+Y in local grid coordinates) side.
    North,
    /// Add row on the South (-Y in local grid coordinates) side.
    South,
    /// Add column on the East (+X in local grid coordinates) side.
    East,
    /// Add column on the West (-X in local grid coordinates) side.
    West,
}

impl GridExpansionDirection {
    pub fn delta_row_col(&self) -> (i32, i32) {
        match self {
            Self::North => (1, 0),
            Self::South => (-1, 0),
            Self::East => (0, 1),
            Self::West => (0, -1),
        }
    }

    pub fn unit_offset(&self, yaw: Angle, cell_size: [f32; 2]) -> [f32; 2] {
        let (dr, dc) = self.delta_row_col();
        let dx_local = dc as f32 * cell_size[0];
        let dy_local = dr as f32 * cell_size[1];
        let (s, c) = yaw.radians().sin_cos();
        [dx_local * c - dy_local * s, dx_local * s + dy_local * c]
    }
}

/// Event to spawn a new `NavGrid` on the current level.
#[derive(Event, Debug, Clone)]
pub struct SpawnNavGrid {
    pub grid: NavGrid,
    pub rows: u32,
    pub cols: u32,
    pub initial_direction: PreferredDirection,
}

impl Default for SpawnNavGrid {
    fn default() -> Self {
        Self {
            grid: NavGrid::default(),
            rows: 4,
            cols: 4,
            initial_direction: PreferredDirection::None,
        }
    }
}

/// Event to expand a grid or extrude selected `NavCell`s by one row or column.
#[derive(Event, Debug, Clone)]
pub struct ExpandNavGrid {
    pub cells: Vec<Entity>,
    pub direction: GridExpansionDirection,
}

/// Event to snap nearby boundary anchors across selected `NavCell`s and stitch graph edges.
#[derive(Event, Debug, Clone)]
pub struct SnapAndStitchCells {
    pub cells: Vec<Entity>,
    pub snap_distance: f32,
}

/// Event to merge 2+ selected `NavCell`s into a single polygonal `NavCell`.
#[derive(Event, Debug, Clone)]
pub struct MergeSelectedCells {
    pub cells: Vec<Entity>,
    pub snap_distance: f32,
}

/// Event to connect or disconnect a set of selected `NavCell`s with `NavCellEdge`s.
#[derive(Event, Debug, Clone)]
pub struct ConnectSelectedCells {
    pub cells: Vec<Entity>,
    pub connect: bool,
}

/// Event to export the current level's polygon navigation graph to a `pibt_rs::NavGraph` JSON file.
#[derive(Event, Debug, Clone)]
pub struct ExportPibtNavGraph {
    pub to_file: PathBuf,
}

fn nav_cell_color(direction: PreferredDirection) -> Color {
    match direction {
        PreferredDirection::None => Color::srgba(0.18, 0.58, 0.85, 0.42),
        PreferredDirection::Angle(angle) => {
            // Color-code by angle hue for quick visual distinction in spreadsheet view
            let deg = angle.degrees().rem_euclid(360.0);
            Color::hsla(deg, 0.72, 0.52, 0.50)
        }
    }
}

fn nav_edge_color(guide: PibtGuidePreference) -> Color {
    match guide {
        PibtGuidePreference::FORWARD => Color::srgb(0.15, 0.90, 0.35),
        PibtGuidePreference::BACKWARD => Color::srgb(0.95, 0.55, 0.15),
        PibtGuidePreference::NONE => Color::srgb(0.75, 0.82, 0.88),
    }
}

pub fn compute_nav_cell_vertices_2d(
    entity: Entity,
    anchor_path: &Path<Entity>,
    anchors: &AnchorParams,
) -> Vec<(f32, f32)> {
    let mut pts = Vec::with_capacity(anchor_path.len());
    for anchor in &anchor_path.0 {
        if let Ok(p) = anchors.point_in_parent_frame_of(*anchor, Category::NavCell, entity) {
            pts.push((p.x, p.y));
        }
    }
    pts
}

fn make_nav_cell_mesh(
    entity: Entity,
    anchor_path: &Path<Entity>,
    preferred_direction: PreferredDirection,
    anchors: &AnchorParams,
) -> Mesh {
    if anchor_path.is_empty() {
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
    }

    let mut reference_positions: Vec<[f32; 3]> = Vec::with_capacity(anchor_path.len());
    for anchor in &anchor_path.0 {
        if let Ok(p) = anchors.point_in_parent_frame_of(*anchor, Category::NavCell, entity) {
            reference_positions.push([p.x, p.y, 0.0]);
        }
    }

    if reference_positions.len() < 3 {
        let p = if reference_positions.is_empty() {
            Vec3::ZERO
        } else {
            let sum = reference_positions
                .iter()
                .fold(Vec3::ZERO, |acc, p| acc + Vec3::from(*p));
            sum / reference_positions.len() as f32
        };
        return make_flat_square_mesh(0.25)
            .transform_by(Affine3A::from_translation(p))
            .into();
    }

    let polygon = Polygon::new(
        LineString::from(
            reference_positions
                .iter()
                .map(|p| [p[0], p[1]])
                .collect::<Vec<_>>(),
        ),
        vec![],
    );

    let outline_buffer = make_closed_path_outline(reference_positions.clone());

    let mut positions: Vec<[f32; 3]> = Vec::new();
    if let Ok(triangles) = polygon.constrained_triangulation(Default::default()) {
        positions.reserve(triangles.len() * 3);
        for triangle in triangles.iter() {
            positions.extend(triangle.coords_iter().map(|v| [v.x, v.y, 0.0]));
        }
    }

    let indices: Vec<u32> = (0..positions.len() as u32).collect();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; positions.len()];

    let mut buffer = MeshBuffer::new(positions, normals, indices).merge_with(outline_buffer);

    // If a PreferredDirection is set, render a directional arrow at the polygon centroid
    if let Some(angle) = preferred_direction.angle() {
        let verts_2d: Vec<(f32, f32)> = reference_positions.iter().map(|p| (p[0], p[1])).collect();
        let (cx, cy) = compute_polygon_centroid_2d(&verts_2d);

        // Scale arrow to fit comfortably inside the cell bounding box
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(x, y) in &verts_2d {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = (max_x - min_x).min(max_y - min_y).max(0.2);
        let arrow_len = (span * 0.55).clamp(0.15, 1.2);
        let handle_len = arrow_len * 0.55;
        let tip_len = arrow_len * 0.45;
        let handle_width = arrow_len * 0.18;
        let tip_width = arrow_len * 0.42;

        let yaw = angle.radians();
        let (sin_yaw, cos_yaw) = yaw.sin_cos();
        // Center the arrow at (cx, cy) slightly above the cell surface
        let start = Vec3::new(
            cx - 0.5 * arrow_len * cos_yaw,
            cy - 0.5 * arrow_len * sin_yaw,
            0.001,
        );
        let arrow_buf = flat_arrow_mesh(handle_len, handle_width, tip_len, tip_width).transform_by(
            Affine3A::from_scale_rotation_translation(Vec3::ONE, Quat::from_rotation_z(yaw), start),
        );
        buffer = buffer.merge_with(arrow_buf);
    }

    buffer.into()
}

pub fn add_nav_cell_visuals(
    mut commands: Commands,
    cells: Query<(Entity, &Path<Entity>, &PreferredDirection), Added<NavCellMarker>>,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let z = ZLayer::Floor.to_z() + 0.0005;
    for (e, path, preferred_direction) in &cells {
        let mesh = make_nav_cell_mesh(e, path, *preferred_direction, &anchors);
        let material = materials.add(StandardMaterial {
            base_color: nav_cell_color(*preferred_direction),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.2,
            metallic: 0.0,
            cull_mode: None,
            ..default()
        });

        let mesh_entity_id = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::default(),
                Visibility::default(),
            ))
            .insert(Selectable::new(e))
            .id();

        commands
            .entity(e)
            .insert((
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::default(),
                NavCellVisuals {
                    mesh: mesh_entity_id,
                },
                Category::NavCell,
                PathBehavior::for_floor(),
                Dependents::default(),
            ))
            .add_child(mesh_entity_id);

        for anchor in &path.0 {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }
    }
}

pub fn update_nav_cells(
    cells: Query<
        (Entity, &NavCellVisuals, &Path<Entity>, &PreferredDirection),
        (
            With<NavCellMarker>,
            Or<(Changed<Path<Entity>>, Changed<PreferredDirection>)>,
        ),
    >,
    anchors: AnchorParams,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mesh_handles: Query<&Mesh3d>,
    material_handles: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    for (e, visuals, path, preferred_direction) in &cells {
        if let Ok(mesh_handle) = mesh_handles.get(visuals.mesh) {
            if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                *mesh = make_nav_cell_mesh(e, path, *preferred_direction, &anchors);
            }
        }
        if let Ok(mat_handle) = material_handles.get(visuals.mesh) {
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.base_color = nav_cell_color(*preferred_direction);
            }
        }
    }
}

pub fn update_nav_cells_for_moved_anchors(
    cells: Query<
        (Entity, &NavCellVisuals, &Path<Entity>, &PreferredDirection),
        With<NavCellMarker>,
    >,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_handles: Query<&Mesh3d>,
) {
    for deps in &changed_anchors {
        for dep in deps.iter() {
            if let Ok((e, visuals, path, preferred_direction)) = cells.get(*dep) {
                if let Ok(mesh_handle) = mesh_handles.get(visuals.mesh) {
                    if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                        *mesh = make_nav_cell_mesh(e, path, *preferred_direction, &anchors);
                    }
                }
            }
        }
    }
}

fn compute_edge_visual_transform_and_color(
    edge: &Edge<Entity>,
    guide: NavGuidePreference,
    cells: &Query<(&Path<Entity>, &PreferredDirection), With<NavCellMarker>>,
    anchors: &AnchorParams,
) -> Option<(Transform, Color)> {
    let (u_path, u_dir) = cells.get(edge.start()).ok()?;
    let (v_path, v_dir) = cells.get(edge.end()).ok()?;

    let u_verts = compute_nav_cell_vertices_2d(edge.start(), u_path, anchors);
    let v_verts = compute_nav_cell_vertices_2d(edge.end(), v_path, anchors);
    if u_verts.is_empty() || v_verts.is_empty() {
        return None;
    }

    let u_centroid = compute_polygon_centroid_2d(&u_verts);
    let v_centroid = compute_polygon_centroid_2d(&v_verts);
    let p_start = Vec3::new(u_centroid.0, u_centroid.1, 0.0);
    let p_end = Vec3::new(v_centroid.0, v_centroid.1, 0.0);

    let mut tf = line_stroke_transform(&p_start, &p_end, NAV_CELL_EDGE_WIDTH);
    tf.translation.z = ZLayer::Lane.to_z() + 0.0005;

    let effective_guide = resolve_guide_preference(u_centroid, v_centroid, *u_dir, *v_dir, guide);
    Some((tf, nav_edge_color(effective_guide)))
}

pub fn add_nav_cell_edge_visuals(
    mut commands: Commands,
    edges: Query<
        (
            Entity,
            &Edge<Entity>,
            &NavGuidePreference,
            Option<&Transform>,
        ),
        Added<NavCellEdgeMarker>,
    >,
    cells: Query<(&Path<Entity>, &PreferredDirection), With<NavCellMarker>>,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents>,
    assets: Res<SiteAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, edge, guide, tf) in &edges {
        let (transform, color) =
            compute_edge_visual_transform_and_color(edge, *guide, &cells, &anchors).unwrap_or((
                Transform::IDENTITY,
                nav_edge_color(PibtGuidePreference::NONE),
            ));

        let mat_handle = materials.add(old_default_material(color));

        let child_id = commands
            .spawn((
                Mesh3d(assets.lane_mid_mesh.clone()),
                MeshMaterial3d(mat_handle),
                transform,
                Visibility::default(),
            ))
            .insert(Selectable::new(e))
            .id();

        if tf.is_none() {
            commands
                .entity(e)
                .insert((Transform::IDENTITY, Visibility::Inherited));
        }

        commands
            .entity(e)
            .insert(Category::NavCellEdge)
            .insert(NavCellEdgeVisuals(child_id))
            .insert(EdgeLabels::StartEnd)
            .add_child(child_id);

        for cell_entity in edge.array() {
            if let Ok(mut deps) = dependents.get_mut(cell_entity) {
                deps.insert(e);
            }
        }
    }
}

pub fn update_nav_cell_edges(
    edges: Query<
        (&Edge<Entity>, &NavGuidePreference, &NavCellEdgeVisuals),
        With<NavCellEdgeMarker>,
    >,
    changed_edges: Query<
        Entity,
        (
            With<NavCellEdgeMarker>,
            Or<(Changed<Edge<Entity>>, Changed<NavGuidePreference>)>,
        ),
    >,
    changed_cells: Query<
        &Dependents,
        (
            With<NavCellMarker>,
            Or<(Changed<Path<Entity>>, Changed<PreferredDirection>)>,
        ),
    >,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    cell_dependents: Query<&Dependents, With<NavCellMarker>>,
    cells: Query<(&Path<Entity>, &PreferredDirection), With<NavCellMarker>>,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
    material_handles: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut dirty_edges: HashSet<Entity> = changed_edges.iter().collect();
    for deps in &changed_cells {
        for dep in deps.iter() {
            if edges.contains(*dep) {
                dirty_edges.insert(*dep);
            }
        }
    }
    for anchor_deps in &changed_anchors {
        for cell_e in anchor_deps.iter() {
            if let Ok(c_deps) = cell_dependents.get(*cell_e) {
                for dep in c_deps.iter() {
                    if edges.contains(*dep) {
                        dirty_edges.insert(*dep);
                    }
                }
            }
        }
    }

    for edge_e in dirty_edges {
        let Ok((edge, guide, visuals)) = edges.get(edge_e) else {
            continue;
        };
        let Some((new_tf, color)) =
            compute_edge_visual_transform_and_color(edge, *guide, &cells, &anchors)
        else {
            continue;
        };
        if let Ok(mut tf) = transforms.get_mut(**visuals) {
            *tf = new_tf;
        }
        if let Ok(mat_handle) = material_handles.get(**visuals) {
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.base_color = color;
            }
        }
    }
}

// ============================================================================
// Grid Spawning, Row/Column Expansion, Snapping & Polygon Merging Operations
// ============================================================================

/// Handles `SpawnNavGrid` events by creating a `NavGrid` entity, shared corner `Anchor`s,
/// `rows * cols` `NavCell` entities, and orthogonal `NavCellEdge`s on the current level.
pub fn handle_spawn_nav_grid(
    mut commands: Commands,
    mut events: EventReader<SpawnNavGrid>,
    mut current_level: ResMut<CurrentLevel>,
) {
    for event in events.read() {
        let rows = event.rows.max(1) as i32;
        let cols = event.cols.max(1) as i32;

        let level = if let Some(level) = current_level.0 {
            level
        } else {
            let new_level_id = commands
                .spawn((Transform::default(), Visibility::default()))
                .insert(LevelProperties {
                    name: NameInSite("<Unnamed>".to_owned()),
                    elevation: LevelElevation(0.0),
                    ..default()
                })
                .insert(Category::Level)
                .id();
            current_level.0 = Some(new_level_id);
            new_level_id
        };

        let grid_entity = commands
            .spawn((
                event.grid,
                Category::NavGrid,
                ChildOf(level),
                Dependents::default(),
            ))
            .id();

        // Spawn (rows + 1) * (cols + 1) shared corner anchors
        let mut corner_anchors: HashMap<(i32, i32), Entity> = HashMap::new();
        for r in 0..=rows {
            for c in 0..=cols {
                let pos = event.grid.corner_position(c, r);
                let anchor_e = commands
                    .spawn(AnchorBundle::new(Anchor::Translate2D(pos)))
                    .insert(ChildOf(level))
                    .id();
                corner_anchors.insert((r, c), anchor_e);
            }
        }

        // Spawn rows * cols NavCell entities
        let mut cell_entities: HashMap<(i32, i32), Entity> = HashMap::new();
        let mut grid_deps = HashSet::new();
        for r in 0..rows {
            for c in 0..cols {
                let a00 = corner_anchors[&(r, c)];
                let a01 = corner_anchors[&(r, c + 1)];
                let a11 = corner_anchors[&(r + 1, c + 1)];
                let a10 = corner_anchors[&(r + 1, c)];

                let mut cell = NavCell::new(Path(vec![a00, a01, a11, a10]));
                cell.preferred_direction = event.initial_direction;
                cell.grid_coord = GridCellCoord(Some(GridCellIndex {
                    grid: grid_entity,
                    row: r,
                    col: c,
                }));

                let mut recall = RecallPreferredDirection::default();
                recall.remember(&event.initial_direction);

                let cell_e = commands
                    .spawn(cell)
                    .insert(recall)
                    .insert(ChildOf(level))
                    .id();
                cell_entities.insert((r, c), cell_e);
                grid_deps.insert(cell_e);
            }
        }

        commands.entity(grid_entity).insert(Dependents(grid_deps));

        // Spawn 4-connected orthogonal NavCellEdges
        for r in 0..rows {
            for c in 0..cols {
                let u = cell_entities[&(r, c)];
                if c + 1 < cols {
                    let v = cell_entities[&(r, c + 1)];
                    commands
                        .spawn(NavCellEdge::new(u, v))
                        .insert(ChildOf(level));
                }
                if r + 1 < rows {
                    let v = cell_entities[&(r + 1, c)];
                    commands
                        .spawn(NavCellEdge::new(u, v))
                        .insert(ChildOf(level));
                }
            }
        }
    }
}

/// Handles `ExpandNavGrid` events to add a row/column to an existing `NavGrid` or extrude
/// a set of selected `NavCell`s in the requested direction, reusing shared boundary anchors.
pub fn handle_expand_nav_grid(
    mut commands: Commands,
    mut events: EventReader<ExpandNavGrid>,
    cells_query: Query<
        (
            Entity,
            &Path<Entity>,
            &PreferredDirection,
            &GridCellCoord<Entity>,
            &ChildOf,
        ),
        With<NavCellMarker>,
    >,
    grids_query: Query<&NavGrid>,
    anchors_query: Query<(Entity, &Anchor, &ChildOf)>,
    mut select: EventWriter<Select>,
) {
    for event in events.read() {
        if event.cells.is_empty() {
            continue;
        }

        // Check if any selected cell belongs to a NavGrid
        let mut target_grid: Option<(Entity, NavGrid, Entity)> = None;
        for &cell_e in &event.cells {
            if let Ok((_, _, _, grid_coord, child_of)) = cells_query.get(cell_e) {
                if let Some(idx) = grid_coord.0 {
                    if let Ok(grid) = grids_query.get(idx.grid) {
                        target_grid = Some((idx.grid, *grid, child_of.parent()));
                        break;
                    }
                }
            }
        }

        if let Some((grid_entity, grid, level)) = target_grid {
            // Collect all existing cells on this grid
            let mut existing_cells: HashMap<(i32, i32), (Entity, PreferredDirection)> =
                HashMap::new();
            for (cell_e, _, pref_dir, grid_coord, _) in &cells_query {
                if let Some(idx) = grid_coord.0 {
                    if idx.grid == grid_entity {
                        existing_cells.insert((idx.row, idx.col), (cell_e, *pref_dir));
                    }
                }
            }
            if existing_cells.is_empty() {
                continue;
            }

            // Build lookup of existing anchors on this level by position (within 1cm tolerance)
            let mut level_anchors: Vec<(Entity, [f32; 2])> = Vec::new();
            for (a_e, anchor, a_parent) in &anchors_query {
                if a_parent.parent() == level {
                    level_anchors.push((a_e, anchor.translation_for_category(Category::NavCell)));
                }
            }

            let mut get_or_spawn_corner = |col: i32, row: i32, commands: &mut Commands| -> Entity {
                let pos = grid.corner_position(col, row);
                for &(a_e, a_pos) in &level_anchors {
                    let dx = a_pos[0] - pos[0];
                    let dy = a_pos[1] - pos[1];
                    if dx * dx + dy * dy < 1e-4 {
                        return a_e;
                    }
                }
                let new_a = commands
                    .spawn(AnchorBundle::new(Anchor::Translate2D(pos)))
                    .insert(ChildOf(level))
                    .id();
                level_anchors.push((new_a, pos));
                new_a
            };

            let min_row = existing_cells.keys().map(|&(r, _)| r).min().unwrap();
            let max_row = existing_cells.keys().map(|&(r, _)| r).max().unwrap();
            let min_col = existing_cells.keys().map(|&(_, c)| c).min().unwrap();
            let max_col = existing_cells.keys().map(|&(_, c)| c).max().unwrap();

            // Determine new (row, col) coordinates to spawn along the chosen boundary
            let coords_to_spawn: Vec<(i32, i32, PreferredDirection)> = match event.direction {
                GridExpansionDirection::North => (min_col..=max_col)
                    .map(|c| {
                        let inherited_dir = existing_cells
                            .get(&(max_row, c))
                            .map(|&(_, d)| d)
                            .unwrap_or(PreferredDirection::None);
                        (max_row + 1, c, inherited_dir)
                    })
                    .collect(),
                GridExpansionDirection::South => (min_col..=max_col)
                    .map(|c| {
                        let inherited_dir = existing_cells
                            .get(&(min_row, c))
                            .map(|&(_, d)| d)
                            .unwrap_or(PreferredDirection::None);
                        (min_row - 1, c, inherited_dir)
                    })
                    .collect(),
                GridExpansionDirection::East => (min_row..=max_row)
                    .map(|r| {
                        let inherited_dir = existing_cells
                            .get(&(r, max_col))
                            .map(|&(_, d)| d)
                            .unwrap_or(PreferredDirection::None);
                        (r, max_col + 1, inherited_dir)
                    })
                    .collect(),
                GridExpansionDirection::West => (min_row..=max_row)
                    .map(|r| {
                        let inherited_dir = existing_cells
                            .get(&(r, min_col))
                            .map(|&(_, d)| d)
                            .unwrap_or(PreferredDirection::None);
                        (r, min_col - 1, inherited_dir)
                    })
                    .collect(),
            };

            let mut newly_spawned: Vec<Entity> = Vec::new();
            for (r, c, pref_dir) in coords_to_spawn {
                if existing_cells.contains_key(&(r, c)) {
                    continue;
                }
                let a00 = get_or_spawn_corner(c, r, &mut commands);
                let a01 = get_or_spawn_corner(c + 1, r, &mut commands);
                let a11 = get_or_spawn_corner(c + 1, r + 1, &mut commands);
                let a10 = get_or_spawn_corner(c, r + 1, &mut commands);

                let mut cell = NavCell::new(Path(vec![a00, a01, a11, a10]));
                cell.preferred_direction = pref_dir;
                cell.grid_coord = GridCellCoord(Some(GridCellIndex {
                    grid: grid_entity,
                    row: r,
                    col: c,
                }));

                let mut recall = RecallPreferredDirection::default();
                recall.remember(&pref_dir);

                let cell_e = commands
                    .spawn(cell)
                    .insert(recall)
                    .insert(ChildOf(level))
                    .id();

                existing_cells.insert((r, c), (cell_e, pref_dir));
                newly_spawned.push(cell_e);

                // Connect to any existing orthogonal neighbors
                for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    if let Some(&(neighbor_e, _)) = existing_cells.get(&(r + dr, c + dc)) {
                        commands
                            .spawn(NavCellEdge::new(neighbor_e, cell_e))
                            .insert(ChildOf(level));
                    }
                }
            }

            for (i, cell_e) in newly_spawned.into_iter().enumerate() {
                select.write(Select::new(Some(cell_e)).multi_select(i > 0));
            }
        }
    }
}

/// Checks if two cells share at least one boundary edge (2 anchors) or at least one anchor.
fn cells_share_boundary(path_a: &[Entity], path_b: &[Entity]) -> usize {
    path_a.iter().filter(|a| path_b.contains(a)).count()
}

/// Handles `SnapAndStitchCells` events:
/// 1. Finds boundary anchors among the selected cells that are within `snap_distance` of each other
///    (and do not belong to the same cell).
/// 2. Merges each close anchor pair into a single shared `Anchor` at their midpoint, updating
///    all `Path<Entity>` references and despawning the duplicate anchor.
/// 3. Automatically creates `NavCellEdge`s between any selected cells that now share an edge
///    (or anchor) and are not yet connected.
pub fn handle_snap_and_stitch_cells(
    world: &mut World,
    state: &mut SystemState<(
        EventReader<SnapAndStitchCells>,
        Query<(Entity, &Path<Entity>, &ChildOf), With<NavCellMarker>>,
        Query<&Edge<Entity>, With<NavCellEdgeMarker>>,
        Query<&Anchor>,
    )>,
) {
    let (mut events, cells_q, edges_q, anchors_q) = state.get_mut(world);
    let requests: Vec<SnapAndStitchCells> = events.read().cloned().collect();
    if requests.is_empty() {
        return;
    }

    // Build union-find of anchors to merge
    let mut replacements: HashMap<Entity, Entity> = HashMap::new();
    let mut midpoint_updates: HashMap<Entity, [f32; 2]> = HashMap::new();
    let mut new_edges_to_spawn: Vec<(Entity, Entity, Entity)> = Vec::new();

    for req in &requests {
        if req.cells.len() < 2 {
            continue;
        }
        let threshold_sq = req.snap_distance * req.snap_distance;

        // Collect (cell_entity, level_entity, Vec<(anchor_entity, [x, y])>)
        let mut cell_anchor_info: Vec<(Entity, Entity, Vec<(Entity, [f32; 2])>)> = Vec::new();
        for &cell_e in &req.cells {
            if let Ok((e, path, child_of)) = cells_q.get(cell_e) {
                let mut pts = Vec::new();
                for &a_e in &path.0 {
                    if let Ok(a) = anchors_q.get(a_e) {
                        pts.push((a_e, a.translation_for_category(Category::NavCell)));
                    }
                }
                cell_anchor_info.push((e, child_of.parent(), pts));
            }
        }

        // Compare anchors across different cells
        for i in 0..cell_anchor_info.len() {
            for j in (i + 1)..cell_anchor_info.len() {
                let (cell_a, level_a, ref anchors_a) = cell_anchor_info[i];
                let (cell_b, level_b, ref anchors_b) = cell_anchor_info[j];
                if level_a != level_b {
                    continue;
                }
                let mut shared_after_snap = 0usize;
                for &(mut a_e, a_pos) in anchors_a {
                    while let Some(&rep) = replacements.get(&a_e) {
                        a_e = rep;
                    }
                    for &(mut b_e, b_pos) in anchors_b {
                        while let Some(&rep) = replacements.get(&b_e) {
                            b_e = rep;
                        }
                        if a_e == b_e {
                            shared_after_snap += 1;
                            continue;
                        }
                        let dx = a_pos[0] - b_pos[0];
                        let dy = a_pos[1] - b_pos[1];
                        if dx * dx + dy * dy <= threshold_sq {
                            // Make sure a_e and b_e aren't both in the same cell
                            let same_cell = anchors_a.iter().any(|&(x, _)| x == b_e)
                                || anchors_b.iter().any(|&(x, _)| x == a_e);
                            if !same_cell {
                                replacements.insert(b_e, a_e);
                                midpoint_updates.insert(
                                    a_e,
                                    [(a_pos[0] + b_pos[0]) * 0.5, (a_pos[1] + b_pos[1]) * 0.5],
                                );
                                shared_after_snap += 1;
                            }
                        }
                    }
                }

                if shared_after_snap >= 1 {
                    let already_connected = edges_q.iter().any(|edge| {
                        (edge.start() == cell_a && edge.end() == cell_b)
                            || (edge.start() == cell_b && edge.end() == cell_a)
                    });
                    if !already_connected {
                        new_edges_to_spawn.push((level_a, cell_a, cell_b));
                    }
                }
            }
        }
    }

    // Apply anchor midpoint updates
    for (keep_anchor, mid) in midpoint_updates {
        if let Some(mut anchor) = world.get_mut::<Anchor>(keep_anchor) {
            *anchor = Anchor::Translate2D(mid);
        }
    }

    // Update all Path<Entity> components that reference replaced anchors
    if !replacements.is_empty() {
        let resolve = |mut a: Entity| -> Entity {
            while let Some(&next) = replacements.get(&a) {
                a = next;
            }
            a
        };

        let all_path_entities: Vec<Entity> = world
            .query_filtered::<Entity, With<Path<Entity>>>()
            .iter(world)
            .collect();

        for path_e in all_path_entities {
            let mut changed = false;
            let mut new_anchors: Vec<Entity> = Vec::new();
            if let Some(path) = world.get::<Path<Entity>>(path_e) {
                for &old_a in &path.0 {
                    let new_a = resolve(old_a);
                    if new_a != old_a {
                        changed = true;
                    }
                    if new_anchors.last().copied() != Some(new_a) {
                        new_anchors.push(new_a);
                    }
                }
                if new_anchors.len() > 1 && new_anchors.first() == new_anchors.last() {
                    new_anchors.pop();
                }
            }
            if changed {
                if let Some(mut path) = world.get_mut::<Path<Entity>>(path_e) {
                    path.0 = new_anchors.clone();
                }
                for &new_a in &new_anchors {
                    if let Some(mut deps) = world.get_mut::<Dependents>(new_a) {
                        deps.insert(path_e);
                    }
                }
            }
        }

        // Remove replaced anchors from dependents and despawn them
        for (&old_a, _) in &replacements {
            world.entity_mut(old_a).despawn();
        }
    }

    // Spawn new NavCellEdges between stitched cells
    for (level, u, v) in new_edges_to_spawn {
        world.spawn(NavCellEdge::new(u, v)).insert(ChildOf(level));
    }
}

/// Simplifies nearly-collinear consecutive vertices of a 2D polygon loop.
fn simplify_collinear_loop(pts: &[[f32; 2]], collinear_eps: f32) -> Vec<[f32; 2]> {
    if pts.len() <= 3 {
        return pts.to_vec();
    }
    let mut cleaned: Vec<[f32; 2]> = Vec::with_capacity(pts.len());
    for &p in pts {
        if let Some(&last) = cleaned.last() {
            let dx = p[0] - last[0];
            let dy = p[1] - last[1];
            if dx * dx + dy * dy < 1e-6 {
                continue;
            }
        }
        cleaned.push(p);
    }
    if cleaned.len() > 1 {
        let first = cleaned[0];
        let last = *cleaned.last().unwrap();
        let dx = first[0] - last[0];
        let dy = first[1] - last[1];
        if dx * dx + dy * dy < 1e-6 {
            cleaned.pop();
        }
    }
    let n = cleaned.len();
    if n <= 3 {
        return cleaned;
    }

    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let prev = cleaned[(i + n - 1) % n];
        let curr = cleaned[i];
        let next = cleaned[(i + 1) % n];
        let v1 = Vec2::new(curr[0] - prev[0], curr[1] - prev[1]).normalize_or_zero();
        let v2 = Vec2::new(next[0] - curr[0], next[1] - curr[1]).normalize_or_zero();
        let cross = (v1.x * v2.y - v1.y * v2.x).abs();
        let dot = v1.dot(v2);
        // Keep vertex unless it is nearly collinear going forward
        if !(cross < collinear_eps && dot > 0.0) {
            result.push(curr);
        }
    }
    if result.len() < 3 {
        cleaned
    } else {
        result
    }
}

/// Handles `MergeSelectedCells` events:
/// 1. First snaps nearby boundary anchors across the selected cells within `snap_distance`.
/// 2. Computes the 2D boolean union (`geo::BooleanOps::union`) of the selected cells' polygons.
/// 3. Simplifies collinear boundary vertices on straight edges and maps exterior ring vertices
///    back to existing `Anchor` entities (or spawns new `Anchor`s where edges intersected).
/// 4. Spawns the merged `NavCell`, rewires external `NavCellEdge`s from the old cells to the
///    merged cell (deduplicating parallel edges and dropping internal edges), deletes the old
///    `NavCell`s, and cleans up orphaned interior `Anchor`s whose `Dependents` become empty.
pub fn handle_merge_selected_cells(world: &mut World) {
    let requests: Vec<MergeSelectedCells> = world
        .resource_mut::<Events<MergeSelectedCells>>()
        .drain()
        .collect();
    if requests.is_empty() {
        return;
    }

    for req in requests {
        if req.cells.len() < 2 {
            continue;
        }

        // Gather cell polygons, preferred direction, graphs, and parent level
        let mut level_entity: Option<Entity> = None;
        let mut merged_pref_dir = PreferredDirection::None;
        let mut merged_graphs = AssociatedGraphs::<Entity>::All;
        let mut valid_cells: Vec<Entity> = Vec::new();
        let mut cell_anchor_paths: Vec<Vec<Entity>> = Vec::new();

        for &cell_e in &req.cells {
            let Some(path) = world.get::<Path<Entity>>(cell_e).cloned() else {
                continue;
            };
            let Some(child_of) = world.get::<ChildOf>(cell_e).cloned() else {
                continue;
            };
            if level_entity.is_none() {
                level_entity = Some(child_of.parent());
            } else if level_entity != Some(child_of.parent()) {
                continue;
            }
            if let Some(pref) = world.get::<PreferredDirection>(cell_e).copied() {
                if merged_pref_dir.is_none() && !pref.is_none() {
                    merged_pref_dir = pref;
                }
            }
            if let Some(graphs) = world.get::<AssociatedGraphs<Entity>>(cell_e).cloned() {
                if matches!(merged_graphs, AssociatedGraphs::All) {
                    merged_graphs = graphs;
                }
            }
            valid_cells.push(cell_e);
            cell_anchor_paths.push(path.0);
        }

        let Some(level) = level_entity else {
            continue;
        };
        if valid_cells.len() < 2 {
            continue;
        }

        // Collect all anchors referenced by valid_cells and their 2D positions
        let mut anchor_positions: HashMap<Entity, [f32; 2]> = HashMap::new();
        for path in &cell_anchor_paths {
            for &a_e in path {
                if let Some(anchor) = world.get::<Anchor>(a_e) {
                    anchor_positions
                        .insert(a_e, anchor.translation_for_category(Category::NavCell));
                }
            }
        }

        // Snap nearby vertex positions across different cells so geo::BooleanOps::union
        // cleanly merges slightly misaligned grids without leaving microscopic slivers.
        let snap_sq = req.snap_distance * req.snap_distance;
        let anchor_keys: Vec<Entity> = anchor_positions.keys().copied().collect();
        for i in 0..anchor_keys.len() {
            for j in (i + 1)..anchor_keys.len() {
                let a_i = anchor_keys[i];
                let a_j = anchor_keys[j];
                let p_i = anchor_positions[&a_i];
                let p_j = anchor_positions[&a_j];
                let dx = p_i[0] - p_j[0];
                let dy = p_i[1] - p_j[1];
                if dx * dx + dy * dy <= snap_sq {
                    let mid = [(p_i[0] + p_j[0]) * 0.5, (p_i[1] + p_j[1]) * 0.5];
                    anchor_positions.insert(a_i, mid);
                    anchor_positions.insert(a_j, mid);
                }
            }
        }

        // Build MultiPolygon union using geo::BooleanOps
        let mut union_poly: Option<MultiPolygon<f32>> = None;
        for path in &cell_anchor_paths {
            let coords: Vec<[f32; 2]> = path
                .iter()
                .filter_map(|a_e| anchor_positions.get(a_e).copied())
                .collect();
            if coords.len() < 3 {
                continue;
            }
            let poly = Polygon::new(LineString::from(coords), vec![]);
            let mp = MultiPolygon::from(poly);
            union_poly = Some(match union_poly {
                None => mp,
                Some(acc) => acc.union(&mp),
            });
        }

        let Some(union_mp) = union_poly else {
            continue;
        };

        // Pick the largest polygon from the union result
        let Some(largest_poly) = union_mp.0.into_iter().max_by(|a, b| {
            let area_a = compute_abs_ring_area(a.exterior());
            let area_b = compute_abs_ring_area(b.exterior());
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) else {
            continue;
        };

        let raw_exterior: Vec<[f32; 2]> = largest_poly
            .exterior()
            .coords_iter()
            .map(|c| [c.x, c.y])
            .collect();

        let simplified_exterior = simplify_collinear_loop(&raw_exterior, 1e-3);
        if simplified_exterior.len() < 3 {
            continue;
        }

        // Map each exterior vertex to an existing Anchor entity if within tolerance, or spawn a new Anchor
        let match_tol_sq = (req.snap_distance.max(0.02)).powi(2);
        let mut new_path_anchors: Vec<Entity> = Vec::with_capacity(simplified_exterior.len());
        for pt in &simplified_exterior {
            let mut best_anchor: Option<(Entity, f32)> = None;
            for (&a_e, a_pos) in &anchor_positions {
                let dx = a_pos[0] - pt[0];
                let dy = a_pos[1] - pt[1];
                let d2 = dx * dx + dy * dy;
                if d2 <= match_tol_sq {
                    if best_anchor.map_or(true, |(_, best_d2)| d2 < best_d2) {
                        best_anchor = Some((a_e, d2));
                    }
                }
            }
            let anchor_e = if let Some((existing_a, _)) = best_anchor {
                if let Some(mut a) = world.get_mut::<Anchor>(existing_a) {
                    *a = Anchor::Translate2D(*pt);
                }
                existing_a
            } else {
                world
                    .spawn(AnchorBundle::new(Anchor::Translate2D(*pt)))
                    .insert(ChildOf(level))
                    .id()
            };
            if !new_path_anchors.contains(&anchor_e) {
                new_path_anchors.push(anchor_e);
            }
        }

        if new_path_anchors.len() < 3 {
            continue;
        }

        let mut merged_cell = NavCell::new(Path(new_path_anchors.clone()));
        merged_cell.preferred_direction = merged_pref_dir;
        merged_cell.graphs = merged_graphs;

        let mut recall = RecallPreferredDirection::default();
        recall.remember(&merged_pref_dir);

        let merged_cell_e = world
            .spawn(merged_cell)
            .insert(recall)
            .insert(ChildOf(level))
            .id();

        let selected_set: HashSet<Entity> = valid_cells.iter().copied().collect();

        // Collect all NavCellEdges incident to any of the merged cells
        let all_edges: Vec<(Entity, Edge<Entity>, NavGuidePreference)> = world
            .query_filtered::<(Entity, &Edge<Entity>, &NavGuidePreference), With<NavCellEdgeMarker>>()
            .iter(world)
            .map(|(e, edge, guide)| (e, *edge, *guide))
            .collect();

        let mut external_neighbors: HashMap<Entity, NavGuidePreference> = HashMap::new();
        let mut edges_to_despawn: Vec<Entity> = Vec::new();

        for (edge_e, edge, guide) in all_edges {
            let s_in = selected_set.contains(&edge.start());
            let e_in = selected_set.contains(&edge.end());
            if s_in && e_in {
                // Internal edge between merged cells -> despawn
                edges_to_despawn.push(edge_e);
            } else if s_in {
                external_neighbors.entry(edge.end()).or_insert(guide);
                edges_to_despawn.push(edge_e);
            } else if e_in {
                let flipped = match guide {
                    NavGuidePreference::Forward => NavGuidePreference::Backward,
                    NavGuidePreference::Backward => NavGuidePreference::Forward,
                    other => other,
                };
                external_neighbors.entry(edge.start()).or_insert(flipped);
                edges_to_despawn.push(edge_e);
            }
        }

        for edge_e in edges_to_despawn {
            if let Some(edge) = world.get::<Edge<Entity>>(edge_e).copied() {
                for cell_end in edge.array() {
                    if let Some(mut deps) = world.get_mut::<Dependents>(cell_end) {
                        deps.remove(&edge_e);
                    }
                }
            }
            world.entity_mut(edge_e).despawn();
        }

        // Spawn deduplicated edges from merged_cell_e to external neighbors
        for (neighbor_e, guide) in external_neighbors {
            let mut new_edge = NavCellEdge::new(merged_cell_e, neighbor_e);
            new_edge.guide = guide;
            world.spawn(new_edge).insert(ChildOf(level));
        }

        // Remove old cells from anchor dependents and despawn old cells
        for &old_cell_e in &valid_cells {
            if let Some(path) = world.get::<Path<Entity>>(old_cell_e).cloned() {
                for a_e in path.0 {
                    if let Some(mut deps) = world.get_mut::<Dependents>(a_e) {
                        deps.remove(&old_cell_e);
                    }
                }
            }
            world.entity_mut(old_cell_e).despawn();
        }

        // Add merged_cell_e to its perimeter anchors' Dependents
        for &a_e in &new_path_anchors {
            if let Some(mut deps) = world.get_mut::<Dependents>(a_e) {
                deps.insert(merged_cell_e);
            }
        }

        // Clean up any interior/orphaned anchors from the old cells that no longer have any dependents
        let new_path_set: HashSet<Entity> = new_path_anchors.iter().copied().collect();
        for (&a_e, _) in &anchor_positions {
            if !new_path_set.contains(&a_e) {
                let is_orphan = world
                    .get::<Dependents>(a_e)
                    .map_or(true, |deps| deps.is_empty());
                if is_orphan {
                    world.entity_mut(a_e).despawn();
                }
            }
        }

        // Update selection to the newly merged cell
        world
            .resource_mut::<Events<Select>>()
            .send(Select::new(Some(merged_cell_e)));
    }
}

fn compute_abs_ring_area(ring: &LineString<f32>) -> f32 {
    let pts: Vec<(f32, f32)> = ring.coords_iter().map(|c| (c.x, c.y)).collect();
    if pts.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0f32;
    for i in 0..pts.len() {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % pts.len()];
        area += x0 * y1 - x1 * y0;
    }
    0.5 * area.abs()
}

/// Handles `ConnectSelectedCells` events to connect or disconnect selected `NavCell`s.
pub fn handle_connect_selected_cells(
    mut commands: Commands,
    mut events: EventReader<ConnectSelectedCells>,
    cells_q: Query<(&Path<Entity>, &ChildOf), With<NavCellMarker>>,
    edges_q: Query<(Entity, &Edge<Entity>), With<NavCellEdgeMarker>>,
    mut dependents_q: Query<&mut Dependents>,
) {
    for event in events.read() {
        if event.cells.len() < 2 {
            continue;
        }
        let selected_set: HashSet<Entity> = event.cells.iter().copied().collect();

        if event.connect {
            for i in 0..event.cells.len() {
                for j in (i + 1)..event.cells.len() {
                    let u = event.cells[i];
                    let v = event.cells[j];
                    let (Ok((path_u, parent_u)), Ok((path_v, parent_v))) =
                        (cells_q.get(u), cells_q.get(v))
                    else {
                        continue;
                    };
                    if parent_u.parent() != parent_v.parent() {
                        continue;
                    }
                    // If more than 2 cells are selected, only connect pairs that share at least 1 anchor;
                    // if exactly 2 cells are selected, connect them unconditionally.
                    if event.cells.len() > 2 && cells_share_boundary(&path_u.0, &path_v.0) == 0 {
                        continue;
                    }
                    let exists = edges_q.iter().any(|(_, e)| {
                        (e.start() == u && e.end() == v) || (e.start() == v && e.end() == u)
                    });
                    if !exists {
                        commands
                            .spawn(NavCellEdge::new(u, v))
                            .insert(ChildOf(parent_u.parent()));
                    }
                }
            }
        } else {
            for (edge_e, edge) in &edges_q {
                if selected_set.contains(&edge.start()) && selected_set.contains(&edge.end()) {
                    for cell_end in edge.array() {
                        if let Ok(mut deps) = dependents_q.get_mut(cell_end) {
                            deps.remove(&edge_e);
                        }
                    }
                    commands.entity(edge_e).despawn();
                }
            }
        }
    }
}

// ============================================================================
// PIBT NavGraph Compilation & Export
// ============================================================================

/// Compiles all `NavCell`s and `NavCellEdge`s on `level` (or across the current site if `level` is `None`)
/// into a `PibtNavGraph` ready for `pibt_rs`.
pub fn compile_pibt_nav_graph(world: &mut World, level: Option<Entity>) -> PibtNavGraph {
    let mut state: SystemState<(
        Query<
            (
                Entity,
                &Path<Entity>,
                &PreferredDirection,
                Option<&SiteID>,
                &ChildOf,
            ),
            (With<NavCellMarker>, Without<Pending>),
        >,
        Query<(&Edge<Entity>, &NavGuidePreference), (With<NavCellEdgeMarker>, Without<Pending>)>,
        AnchorParams,
    )> = SystemState::new(world);

    let (cells_q, edges_q, anchors) = state.get(world);

    let mut cell_entries: Vec<(u32, Entity, Vec<(f32, f32)>, PreferredDirection)> = Vec::new();
    for (e, path, pref_dir, site_id, child_of) in &cells_q {
        if let Some(target_level) = level {
            if child_of.parent() != target_level {
                continue;
            }
        }
        let verts = compute_nav_cell_vertices_2d(e, path, &anchors);
        if verts.len() < 3 {
            continue;
        }
        let sort_key = site_id.map(|s| s.0).unwrap_or(e.index());
        cell_entries.push((sort_key, e, verts, *pref_dir));
    }

    cell_entries.sort_by_key(|(k, e, _, _)| (*k, e.index()));

    let mut graph = PibtNavGraph::default();
    let mut entity_to_idx: HashMap<Entity, (usize, (f32, f32), PreferredDirection)> =
        HashMap::new();

    for (_, cell_e, verts, pref_dir) in cell_entries {
        let poly = PibtPolygon::from_vertices(verts);
        let centroid = poly.centroid;
        let idx = graph.add_node(poly);
        entity_to_idx.insert(cell_e, (idx, centroid, pref_dir));
    }

    for (edge, guide) in &edges_q {
        let Some(&(u_idx, u_centroid, u_dir)) = entity_to_idx.get(&edge.start()) else {
            continue;
        };
        let Some(&(v_idx, v_centroid, v_dir)) = entity_to_idx.get(&edge.end()) else {
            continue;
        };
        let effective_guide =
            resolve_guide_preference(u_centroid, v_centroid, u_dir, v_dir, *guide);
        graph.add_edge(u_idx, v_idx, effective_guide);
    }

    graph
}

pub fn handle_export_pibt_nav_graph(world: &mut World) {
    let events: Vec<ExportPibtNavGraph> = world
        .resource_mut::<Events<ExportPibtNavGraph>>()
        .drain()
        .collect();
    if events.is_empty() {
        return;
    }

    let current_level = world.get_resource::<CurrentLevel>().and_then(|l| l.0);
    for event in events {
        let graph = compile_pibt_nav_graph(world, current_level);
        match std::fs::File::create(&event.to_file) {
            Ok(file) => {
                if let Err(err) = serde_json::to_writer_pretty(file, &graph) {
                    error!(
                        "Failed to serialize PIBT NavGraph to {:?}: {err}",
                        event.to_file
                    );
                } else {
                    info!(
                        "Exported PIBT Polygon NavGraph ({} cells) to {:?}",
                        graph.polygons.len(),
                        event.to_file
                    );
                }
            }
            Err(err) => {
                error!(
                    "Failed to create PIBT NavGraph file {:?}: {err}",
                    event.to_file
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<CurrentLevel>();
        app.init_resource::<Events<Select>>();
        app.init_resource::<Events<SpawnNavGrid>>()
            .init_resource::<Events<ExpandNavGrid>>()
            .init_resource::<Events<SnapAndStitchCells>>()
            .init_resource::<Events<MergeSelectedCells>>()
            .init_resource::<Events<ConnectSelectedCells>>()
            .add_systems(
                Update,
                (
                    handle_spawn_nav_grid,
                    handle_expand_nav_grid,
                    handle_snap_and_stitch_cells,
                    handle_merge_selected_cells,
                    handle_connect_selected_cells,
                ),
            );
        app
    }

    #[test]
    fn test_spawn_and_expand_grid_and_merge_cells() {
        let mut app = setup_test_app();

        // Spawn a 2x2 grid at origin (0, 0) with 1m x 1m cells and East preferred direction
        app.world_mut().send_event(SpawnNavGrid {
            grid: NavGrid {
                origin: [0.0, 0.0],
                cell_size: [1.0, 1.0],
                yaw: Angle::Deg(0.0),
            },
            rows: 2,
            cols: 2,
            initial_direction: PreferredDirection::east(),
        });
        app.update();

        // Verify 4 cells, 9 shared anchors, and 4 orthogonal edges were spawned
        let cells: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NavCellMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(cells.len(), 4);

        let anchors: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<Anchor>>()
            .iter(app.world())
            .collect();
        assert_eq!(anchors.len(), 9);

        let edges: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NavCellEdgeMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(edges.len(), 4);

        // Expand grid by 1 row North -> should add 2 cells and 3 new corner anchors (reusing 3 boundary anchors)
        app.world_mut().send_event(ExpandNavGrid {
            cells: vec![cells[0]],
            direction: GridExpansionDirection::North,
        });
        app.update();

        let cells_after_expand: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NavCellMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(cells_after_expand.len(), 6);

        let anchors_after_expand: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<Anchor>>()
            .iter(app.world())
            .collect();
        assert_eq!(anchors_after_expand.len(), 12);

        // Merge the first 2 bottom cells into a single 2m x 1m polygon cell
        let bottom_cells: Vec<Entity> = app
            .world_mut()
            .query_filtered::<(Entity, &GridCellCoord<Entity>), With<NavCellMarker>>()
            .iter(app.world())
            .filter_map(|(e, gc)| gc.0.filter(|idx| idx.row == 0).map(|_| e))
            .collect();
        assert_eq!(bottom_cells.len(), 2);

        app.world_mut().send_event(MergeSelectedCells {
            cells: bottom_cells,
            snap_distance: 0.1,
        });
        app.update();

        let cells_after_merge: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NavCellMarker>>()
            .iter(app.world())
            .collect();
        // 6 cells - 2 merged + 1 new = 5 cells
        assert_eq!(cells_after_merge.len(), 5);
    }

    #[test]
    fn test_snap_and_stitch_misaligned_grids() {
        let mut app = setup_test_app();

        // Grid A: 1x1 at (0, 0) -> [0, 1] x [0, 1]
        app.world_mut().send_event(SpawnNavGrid {
            grid: NavGrid {
                origin: [0.0, 0.0],
                cell_size: [1.0, 1.0],
                yaw: Angle::Deg(0.0),
            },
            rows: 1,
            cols: 1,
            initial_direction: PreferredDirection::east(),
        });
        // Grid B: 1x1 slightly misaligned at (1.15, 0.05)
        app.world_mut().send_event(SpawnNavGrid {
            grid: NavGrid {
                origin: [1.15, 0.05],
                cell_size: [1.0, 1.0],
                yaw: Angle::Deg(0.0),
            },
            rows: 1,
            cols: 1,
            initial_direction: PreferredDirection::east(),
        });
        app.update();

        let cells: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NavCellMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(cells.len(), 2);

        // Snap & stitch with snap_distance = 0.3m
        app.world_mut().send_event(SnapAndStitchCells {
            cells: cells.clone(),
            snap_distance: 0.3,
        });
        app.update();

        // The 2 adjacent corners should have merged (8 -> 6 anchors) and 1 NavCellEdge created
        let anchors: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<Anchor>>()
            .iter(app.world())
            .collect();
        assert_eq!(anchors.len(), 6);

        let edges: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NavCellEdgeMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_compile_pibt_nav_graph() {
        let mut app = setup_test_app();

        // Spawn a 1x2 grid (1 row, 2 columns) with East preferred direction
        app.world_mut().send_event(SpawnNavGrid {
            grid: NavGrid {
                origin: [0.0, 0.0],
                cell_size: [2.0, 1.0],
                yaw: Angle::Deg(0.0),
            },
            rows: 1,
            cols: 2,
            initial_direction: PreferredDirection::east(),
        });
        app.update();

        // Propagate transforms so AnchorParams can read level-frame coordinates
        let anchor_entities: Vec<(Entity, Anchor)> = app
            .world_mut()
            .query::<(Entity, &Anchor)>()
            .iter(app.world())
            .map(|(e, a)| (e, a.clone()))
            .collect();
        for (e, a) in anchor_entities {
            let pos = a.translation_for_category(Category::NavCell);
            app.world_mut()
                .entity_mut(e)
                .insert(GlobalTransform::from_xyz(pos[0], pos[1], 0.0));
        }

        let level = app.world().resource::<CurrentLevel>().0;
        let pibt_graph = compile_pibt_nav_graph(app.world_mut(), level);

        assert_eq!(pibt_graph.polygons.len(), 2);
        assert_eq!(pibt_graph.connections.len(), 2);
        // Cell 0 is at x in [0, 2], Cell 1 is at x in [2, 4]; preferred direction is East (+X)
        // Edge 0 -> 1 should be FORWARD, and 1 -> 0 should be BACKWARD
        assert_eq!(
            pibt_graph.connections[0],
            vec![(1, PibtGuidePreference::FORWARD)]
        );
        assert_eq!(
            pibt_graph.connections[1],
            vec![(0, PibtGuidePreference::BACKWARD)]
        );
    }
}

// ============================================================================
// Spreadsheet-Style Drag-Box / Marquee Cell Selection
// ============================================================================

/// Resource tracking spreadsheet-style marquee / drag-box selection for `NavCell`s.
#[derive(Resource, Default, Debug, Clone)]
pub struct CellBoxSelectState {
    /// Whether dedicated box-select mode is toggled on from the toolbar.
    /// Note: holding `Shift + Left-Drag` also activates box-selection at any time.
    pub enabled: bool,
    /// World/level-plane `(x, y)` start point of an active drag gesture.
    pub drag_start: Option<Vec2>,
    /// Current world/level-plane `(x, y)` point of an active drag gesture.
    pub drag_current: Option<Vec2>,
}

pub fn update_cell_box_selection(
    mut state: ResMut<CellBoxSelectState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    block_status: Option<Res<PickBlockStatus>>,
    ground_plane: IntersectGroundPlaneParams,
    current_level: Res<CurrentLevel>,
    cells_q: Query<(Entity, &Path<Entity>, &ChildOf), With<NavCellMarker>>,
    anchors: AnchorParams,
    mut selection: ResMut<Selection>,
    mut selected_q: Query<&mut Selected>,
    mut gizmos: Gizmos,
) {
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let can_start = state.enabled || shift_held;

    if block_status.as_ref().is_some_and(|b| b.blocked()) && state.drag_start.is_none() {
        return;
    }

    let current_pt = ground_plane
        .ground_plane_intersection()
        .map(|tf| Vec2::new(tf.translation.x, tf.translation.y));

    if mouse_button.just_pressed(MouseButton::Left) && can_start {
        if let Some(pt) = current_pt {
            state.drag_start = Some(pt);
            state.drag_current = Some(pt);
        }
    }

    if mouse_button.pressed(MouseButton::Left) {
        if let (Some(start), Some(curr)) = (state.drag_start, current_pt) {
            state.drag_current = Some(curr);
            let min = start.min(curr);
            let max = start.max(curr);
            if (max - min).length_squared() > 0.01 {
                let z = ZLayer::HoveredLane.to_z() + 0.002;
                let p00 = Vec3::new(min.x, min.y, z);
                let p10 = Vec3::new(max.x, min.y, z);
                let p11 = Vec3::new(max.x, max.y, z);
                let p01 = Vec3::new(min.x, max.y, z);
                let box_color = Color::srgb(0.2, 0.95, 1.0);
                gizmos.line(p00, p10, box_color);
                gizmos.line(p10, p11, box_color);
                gizmos.line(p11, p01, box_color);
                gizmos.line(p01, p00, box_color);
            }
        }
    }

    if mouse_button.just_released(MouseButton::Left) {
        if let (Some(start), Some(curr)) = (state.drag_start.take(), state.drag_current.take()) {
            let min = start.min(curr);
            let max = start.max(curr);
            // Only treat as a drag-box selection if dragged at least 15 cm
            if (max - min).length_squared() > 0.0225 {
                let Some(level) = current_level.0 else {
                    return;
                };
                if !shift_held {
                    for prev in selection.selected.drain() {
                        if let Ok(mut sel) = selected_q.get_mut(prev) {
                            sel.is_selected = false;
                        }
                    }
                }

                for (cell_e, path, child_of) in &cells_q {
                    if child_of.parent() != level {
                        continue;
                    }
                    let verts = compute_nav_cell_vertices_2d(cell_e, path, &anchors);
                    if verts.is_empty() {
                        continue;
                    }
                    let (cx, cy) = compute_polygon_centroid_2d(&verts);
                    let centroid_in_box = cx >= min.x && cx <= max.x && cy >= min.y && cy <= max.y;
                    let any_vertex_in_box = verts
                        .iter()
                        .any(|&(vx, vy)| vx >= min.x && vx <= max.x && vy >= min.y && vy <= max.y);

                    if centroid_in_box || any_vertex_in_box {
                        if let Ok(mut sel) = selected_q.get_mut(cell_e) {
                            sel.is_selected = true;
                            selection.selected.insert(cell_e);
                        }
                    }
                }
            }
        }
    }
}
