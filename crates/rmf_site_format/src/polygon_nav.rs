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

use crate::*;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Reflect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Serialized polygon-based navigation graph data within a `Site`.
/// Uses dense `Vec` storage instead of `BTreeMap` to match `pibt_rs::NavGraph` indexing.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct PolygonNav {
    /// Grid definitions used to easily add rows/columns to existing grids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grids: Vec<Parented<u32, NavGrid>>,
    /// Dense array of polygon navigation cells (`0..cells.len()`).
    /// `parent` is the `SiteID` of the `Level` the cell belongs to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<Parented<u32, NavCell<u32>>>,
    /// Dense array of connections between polygon navigation cells.
    /// `cells` inside `NavCellEdge<u32>` stores `(start_cell_idx, end_cell_idx)` indexing `PolygonNav::cells`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<Parented<u32, NavCellEdge<u32>>>,
}

impl PolygonNav {
    pub fn is_empty(&self) -> bool {
        self.grids.is_empty() && self.cells.is_empty() && self.edges.is_empty()
    }
}

/// Metadata for a 2D navigation grid on a level, allowing one-click row/column expansion.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct NavGrid {
    /// World/level-space origin `(x, y)` corresponding to grid corner `(col = 0, row = 0)`.
    pub origin: [f32; 2],
    /// Width and height `(cell_width, cell_height)` of each grid cell in meters.
    pub cell_size: [f32; 2],
    /// Counter-clockwise rotation of the grid axes in the level plane.
    #[serde(default = "default_zero_angle")]
    pub yaw: Angle,
}

fn default_zero_angle() -> Angle {
    Angle::Deg(0.0)
}

impl Default for NavGrid {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0],
            cell_size: [1.0, 1.0],
            yaw: Angle::Deg(0.0),
        }
    }
}

impl NavGrid {
    /// Computes the level-space `(x, y)` coordinates of grid corner `(col, row)`.
    pub fn corner_position(&self, col: i32, row: i32) -> [f32; 2] {
        let local_x = col as f32 * self.cell_size[0];
        let local_y = row as f32 * self.cell_size[1];
        let (sin_yaw, cos_yaw) = self.yaw.radians().sin_cos();
        [
            self.origin[0] + local_x * cos_yaw - local_y * sin_yaw,
            self.origin[1] + local_x * sin_yaw + local_y * cos_yaw,
        ]
    }

    /// Computes the 4 CCW corner positions of cell `(row, col)`:
    /// `[(col, row), (col + 1, row), (col + 1, row + 1), (col, row + 1)]`.
    pub fn cell_corners(&self, row: i32, col: i32) -> [[f32; 2]; 4] {
        [
            self.corner_position(col, row),
            self.corner_position(col + 1, row),
            self.corner_position(col + 1, row + 1),
            self.corner_position(col, row + 1),
        ]
    }
}

/// Optional grid coordinate `(grid, row, col)` associating a `NavCell` with a `NavGrid`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridCellIndex<T: RefTrait> {
    pub grid: T,
    pub row: i32,
    pub col: i32,
}

impl<T: RefTrait> GridCellIndex<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<GridCellIndex<U>, T> {
        Ok(GridCellIndex {
            grid: *id_map.get(&self.grid).ok_or(self.grid)?,
            row: self.row,
            col: self.col,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct GridCellCoord<T: RefTrait>(pub Option<GridCellIndex<T>>);

impl<T: RefTrait> GridCellCoord<T> {
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<GridCellCoord<U>, T> {
        match &self.0 {
            Some(idx) => Ok(GridCellCoord(Some(idx.convert(id_map)?))),
            None => Ok(GridCellCoord(None)),
        }
    }
}

/// Preferred travel direction within a polygon `NavCell`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[serde(rename_all = "snake_case")]
pub enum PreferredDirection {
    #[default]
    None,
    Angle(Angle),
}

impl PreferredDirection {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn angle(&self) -> Option<Angle> {
        match self {
            Self::None => None,
            Self::Angle(a) => Some(*a),
        }
    }

    /// Unit vector `(dx, dy)` in level coordinates if a preferred direction is set.
    pub fn unit_vector(&self) -> Option<(f32, f32)> {
        match self {
            Self::None => None,
            Self::Angle(a) => {
                let (s, c) = a.radians().sin_cos();
                Some((c, s))
            }
        }
    }

    pub fn north() -> Self {
        Self::Angle(Angle::Deg(90.0))
    }

    pub fn south() -> Self {
        Self::Angle(Angle::Deg(-90.0))
    }

    pub fn east() -> Self {
        Self::Angle(Angle::Deg(0.0))
    }

    pub fn west() -> Self {
        Self::Angle(Angle::Deg(180.0))
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None (Omnidirectional)",
            Self::Angle(_) => "Directed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallPreferredDirection {
    pub angle: Option<Angle>,
}

impl Recall for RecallPreferredDirection {
    type Source = PreferredDirection;

    fn remember(&mut self, source: &Self::Source) {
        if let PreferredDirection::Angle(a) = source {
            self.angle = Some(*a);
        }
    }
}

/// A polygonal navigation cell defined by shared perimeter `Anchor`s.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct NavCell<T: RefTrait> {
    pub anchors: Path<T>,
    #[serde(default, skip_serializing_if = "PreferredDirection::is_none")]
    pub preferred_direction: PreferredDirection,
    #[serde(default, skip_serializing_if = "AssociatedGraphs::all")]
    pub graphs: AssociatedGraphs<T>,
    #[serde(default, skip_serializing_if = "GridCellCoord::is_none")]
    pub grid_coord: GridCellCoord<T>,
    #[serde(skip)]
    pub marker: NavCellMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct NavCellMarker;

impl<T: RefTrait> NavCell<T> {
    pub fn new(anchors: Path<T>) -> Self {
        Self {
            anchors,
            preferred_direction: PreferredDirection::None,
            graphs: AssociatedGraphs::All,
            grid_coord: GridCellCoord(None),
            marker: NavCellMarker,
        }
    }

    pub fn convert<U: RefTrait>(
        &self,
        anchor_id_map: &HashMap<T, U>,
        graph_id_map: &HashMap<T, U>,
        grid_id_map: &HashMap<T, U>,
    ) -> Result<NavCell<U>, T> {
        Ok(NavCell {
            anchors: self.anchors.convert(anchor_id_map)?,
            preferred_direction: self.preferred_direction,
            graphs: self.graphs.convert(graph_id_map)?,
            grid_coord: self.grid_coord.convert(grid_id_map)?,
            marker: NavCellMarker,
        })
    }
}

impl<T: RefTrait> From<Path<T>> for NavCell<T> {
    fn from(path: Path<T>) -> Self {
        Self::new(path)
    }
}

/// Directionality preference on a connection between two `NavCell`s.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub enum NavGuidePreference {
    /// Automatically derive `FORWARD`, `BACKWARD`, or `NONE` from the connected cells' `PreferredDirection`.
    #[default]
    Auto,
    /// Explicitly prefer `start -> end` (`FORWARD` from start, `BACKWARD` from end).
    #[serde(rename = "FORWARD")]
    Forward,
    /// Explicitly prefer `end -> start` (`BACKWARD` from start, `FORWARD` from end).
    #[serde(rename = "BACKWARD")]
    Backward,
    /// Explicitly bidirectional (`NONE`).
    #[serde(rename = "NONE")]
    None,
}

impl NavGuidePreference {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Auto (from Cell Direction)",
            Self::Forward => "Forward (Start -> End)",
            Self::Backward => "Backward (End -> Start)",
            Self::None => "None (Bidirectional)",
        }
    }
}

/// An edge connecting two `NavCell`s in the polygon navigation graph.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct NavCellEdge<T: RefTrait> {
    pub cells: Edge<T>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub guide: NavGuidePreference,
    #[serde(skip)]
    pub marker: NavCellEdgeMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct NavCellEdgeMarker;

impl<T: RefTrait> NavCellEdge<T> {
    pub fn new(start_cell: T, end_cell: T) -> Self {
        Self {
            cells: Edge::new(start_cell, end_cell),
            guide: NavGuidePreference::Auto,
            marker: NavCellEdgeMarker,
        }
    }

    pub fn convert<U: RefTrait>(&self, cell_id_map: &HashMap<T, U>) -> Result<NavCellEdge<U>, T> {
        Ok(NavCellEdge {
            cells: self.cells.convert(cell_id_map)?,
            guide: self.guide,
            marker: NavCellEdgeMarker,
        })
    }
}

impl<T: RefTrait> From<Edge<T>> for NavCellEdge<T> {
    fn from(cells: Edge<T>) -> Self {
        Self {
            cells,
            guide: NavGuidePreference::Auto,
            marker: NavCellEdgeMarker,
        }
    }
}

// ============================================================================
// pibt_rs::NavGraph Compatible Serialization Types & Conversion
// ============================================================================

/// Matches `hetpibt::GuidePreference` in `pibt_rs` for 1:1 JSON compatibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PibtGuidePreference {
    FORWARD,
    BACKWARD,
    NONE,
}

impl PibtGuidePreference {
    pub fn opposite(&self) -> Self {
        match self {
            Self::FORWARD => Self::BACKWARD,
            Self::BACKWARD => Self::FORWARD,
            Self::NONE => Self::NONE,
        }
    }
}

/// Matches `hetpibt::Polygon` in `pibt_rs` for 1:1 JSON compatibility.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PibtPolygon {
    pub vertices: Vec<(f32, f32)>,
    pub draw_index: Vec<usize>,
    pub centroid: (f32, f32),
}

impl PibtPolygon {
    pub fn from_vertices(vertices: Vec<(f32, f32)>) -> Self {
        let n = vertices.len();
        let mut draw_index: Vec<usize> = (0..n).collect();
        if n > 0 {
            draw_index.push(0);
        }
        let centroid = compute_polygon_centroid_2d(&vertices);
        Self {
            vertices,
            draw_index,
            centroid,
        }
    }
}

/// Matches `hetpibt::NavGraph` in `pibt_rs` for 1:1 JSON compatibility.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PibtNavGraph {
    pub polygons: Vec<PibtPolygon>,
    pub connections: Vec<Vec<(usize, PibtGuidePreference)>>,
}

impl PibtNavGraph {
    pub fn add_node(&mut self, polygon: PibtPolygon) -> usize {
        self.polygons.push(polygon);
        self.connections.push(vec![]);
        self.polygons.len() - 1
    }

    pub fn add_edge(&mut self, n1: usize, n2: usize, guide: PibtGuidePreference) {
        if n1 >= self.connections.len() || n2 >= self.connections.len() || n1 == n2 {
            return;
        }
        if !self.connections[n1].iter().any(|&(v, _)| v == n2) {
            self.connections[n1].push((n2, guide));
            self.connections[n2].push((n1, guide.opposite()));
        }
    }
}

/// Computes the 2D centroid of a polygon from its ordered perimeter vertices.
pub fn compute_polygon_centroid_2d(vertices: &[(f32, f32)]) -> (f32, f32) {
    let n = vertices.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    if n == 1 {
        return vertices[0];
    }
    if n == 2 {
        return (
            (vertices[0].0 + vertices[1].0) * 0.5,
            (vertices[0].1 + vertices[1].1) * 0.5,
        );
    }

    let mut signed_area = 0.0f32;
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;

    for i in 0..n {
        let (x0, y0) = vertices[i];
        let (x1, y1) = vertices[(i + 1) % n];
        let cross = x0 * y1 - x1 * y0;
        signed_area += cross;
        cx += (x0 + x1) * cross;
        cy += (y0 + y1) * cross;
    }

    signed_area *= 0.5;
    if signed_area.abs() > 1e-5 {
        let factor = 1.0 / (6.0 * signed_area);
        (cx * factor, cy * factor)
    } else {
        let sum_x: f32 = vertices.iter().map(|v| v.0).sum();
        let sum_y: f32 = vertices.iter().map(|v| v.1).sum();
        (sum_x / n as f32, sum_y / n as f32)
    }
}

/// Resolves the effective `PibtGuidePreference` for an edge from cell `u` to cell `v`.
///
/// If `edge_guide` is `NavGuidePreference::Auto`, projects the unit edge vector
/// `normalize(v_centroid - u_centroid)` onto the preferred direction vector(s) of `u` and `v`.
/// A projection `> cos(45°)` (`~0.707`) yields `FORWARD`; `< -cos(45°)` yields `BACKWARD`.
pub fn resolve_guide_preference(
    u_centroid: (f32, f32),
    v_centroid: (f32, f32),
    u_dir: PreferredDirection,
    v_dir: PreferredDirection,
    edge_guide: NavGuidePreference,
) -> PibtGuidePreference {
    match edge_guide {
        NavGuidePreference::Forward => return PibtGuidePreference::FORWARD,
        NavGuidePreference::Backward => return PibtGuidePreference::BACKWARD,
        NavGuidePreference::None => return PibtGuidePreference::NONE,
        NavGuidePreference::Auto => {}
    }

    let dx = v_centroid.0 - u_centroid.0;
    let dy = v_centroid.1 - u_centroid.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-5 {
        return PibtGuidePreference::NONE;
    }
    let ux = dx / len;
    let uy = dy / len;

    // Combine preferred direction vectors of u and v if present
    let mut sum_px = 0.0f32;
    let mut sum_py = 0.0f32;
    if let Some((px, py)) = u_dir.unit_vector() {
        sum_px += px;
        sum_py += py;
    }
    if let Some((px, py)) = v_dir.unit_vector() {
        sum_px += px;
        sum_py += py;
    }

    let p_len = (sum_px * sum_px + sum_py * sum_py).sqrt();
    if p_len < 1e-4 {
        return PibtGuidePreference::NONE;
    }

    let dot = ux * (sum_px / p_len) + uy * (sum_py / p_len);
    // cos(45 deg) ~= 0.7071
    const THRESHOLD: f32 = 0.7071;
    if dot >= THRESHOLD {
        PibtGuidePreference::FORWARD
    } else if dot <= -THRESHOLD {
        PibtGuidePreference::BACKWARD
    } else {
        PibtGuidePreference::NONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferred_direction_to_pibt_guide() {
        let u = (0.0, 0.0);
        let v_east = (1.0, 0.0);
        let v_north = (0.0, 1.0);

        // Cell prefers East (0 deg): edge going East is FORWARD, West is BACKWARD, North is NONE
        assert_eq!(
            resolve_guide_preference(
                u,
                v_east,
                PreferredDirection::east(),
                PreferredDirection::east(),
                NavGuidePreference::Auto,
            ),
            PibtGuidePreference::FORWARD
        );
        assert_eq!(
            resolve_guide_preference(
                v_east,
                u,
                PreferredDirection::east(),
                PreferredDirection::east(),
                NavGuidePreference::Auto,
            ),
            PibtGuidePreference::BACKWARD
        );
        assert_eq!(
            resolve_guide_preference(
                u,
                v_north,
                PreferredDirection::east(),
                PreferredDirection::east(),
                NavGuidePreference::Auto,
            ),
            PibtGuidePreference::NONE
        );
    }

    #[test]
    fn test_pibt_nav_graph_json_roundtrip() {
        let mut graph = PibtNavGraph::default();
        let n0 = graph.add_node(PibtPolygon::from_vertices(vec![
            (0.0, 0.0),
            (1.0, 0.0),
            (1.0, 1.0),
            (0.0, 1.0),
        ]));
        let n1 = graph.add_node(PibtPolygon::from_vertices(vec![
            (1.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
            (1.0, 1.0),
        ]));
        graph.add_edge(n0, n1, PibtGuidePreference::FORWARD);

        let json = serde_json::to_string(&graph).unwrap();
        let roundtrip: PibtNavGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph, roundtrip);
        assert_eq!(
            roundtrip.connections[0],
            vec![(1, PibtGuidePreference::FORWARD)]
        );
        assert_eq!(
            roundtrip.connections[1],
            vec![(0, PibtGuidePreference::BACKWARD)]
        );
    }
}
