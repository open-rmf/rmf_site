/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
    site::*,
    shapes::make_flat_square_mesh,
};
use bevy::{
    ecs::system::EntityCommands,
    math::Affine3A,
    prelude::*
};
use smallvec::SmallVec;

#[derive(Component)]
pub struct PassageSkeleton {
    pub cell_group: Entity,
    pub mesh: Handle<Mesh>,
    pub compass: CompassMaterials,
    pub rows: [i32; 2],
    pub columns: usize,
    /// The furthest reach of rows that this passage ever had. This helps us
    /// know whether we need to generate more cells or if we can just make
    /// previously made cells visible again.
    pub rows_extent: [i32; 2],
    /// The furthest reach of columns that this passage ever had.
    pub columns_extent: usize,
}

impl PassageSkeleton {
    fn empty(
        cell_group: Entity,
        mesh: Handle<Mesh>,
        compass_materials: CompassMaterials,
    ) -> Self {
        Self {
            cell_group,
            mesh,
            rows: [0, 0],
            columns: 0,
            rows_extent: [0, 0],
            columns_extent: 0,
            compass: compass_materials,
        }
    }

    fn new(
        cell_group: Entity,
        mesh: Handle<Mesh>,
        compass_materials: CompassMaterials,
        rows: [i32; 2],
        columns: usize,
    ) -> Self {
        Self {
            cell_group,
            mesh,
            compass: compass_materials,
            rows,
            columns,
            rows_extent: rows,
            columns_extent: columns,
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct CellTag {
    pub for_passage: Entity,
    pub coords: [i32; 2],
}

fn compute_passage_alignment_transform(
    cells: &PassageCells,
    alignment: &PassageAlignment,
) -> Transform {
    let width = cells.lanes as f32 * cells.cell_size;
    let x = alignment.longitudinal;
    let y = match alignment.lateral {
        PassageLateralAlignment::Left(dy) => dy,
        PassageLateralAlignment::Center(dy) => -width/2.0 + dy,
        PassageLateralAlignment::Right(dy) => -width + dy,
    };

    Transform::from_translation(Vec3::new(x, y, 0.0))
}

fn compute_passage_frame_transform(
    p_start: &Vec3,
    p_end: &Vec3,
    height: f32,
) -> Transform {
    let dp = *p_end - *p_start;
    let yaw = f32::atan2(dp.y, dp.x);
    Transform::from_translation(Vec3::new(p_start.x, p_start.y, height))
    .with_rotation(Quat::from_axis_angle(Vec3::Z, yaw))
}

// TODO(@mxgrey): We could potentially share these across many passages instead
// of recreating them per passage if we linked it to the nav graph instead of
// storing them in the PassageSkeleton. That would align better with how lane
// materials work.
pub struct CompassMaterials {
    empty: Handle<StandardMaterial>,
    single: Handle<StandardMaterial>,
    capital_l: Handle<StandardMaterial>,
    polar: Handle<StandardMaterial>,
    triple: Handle<StandardMaterial>,
}

impl CompassMaterials {
    fn new<'a>(
        materials: &'a mut Assets<StandardMaterial>,
        textures: &'a CompassTextures,
        color: Color,
    ) -> Self {
        let mut make_mat = |texture: Handle<Image>| -> Handle<StandardMaterial> {
            materials.add(StandardMaterial {
                base_color: color,
                base_color_texture: Some(texture),
                ..default()
            })
        };

        let empty = make_mat(textures.empty.clone());
        let single = make_mat(textures.single.clone());
        let capital_l = make_mat(textures.capital_l.clone());
        let polar = make_mat(textures.polar.clone());
        let triple = make_mat(textures.triple.clone());

        Self { empty, single, capital_l, polar, triple }
    }

    fn orient(&self, constraints: &CellConstraints) -> (f32, Handle<StandardMaterial>) {
        let dirs: Vec<Vec3> = [
            (Vec3::X, constraints.forward.is_unconstrained()),
            (Vec3::NEG_X, constraints.backward.is_unconstrained()),
            (Vec3::Y, constraints.left.is_unconstrained()),
            (Vec3::NEG_Y, constraints.right.is_unconstrained()),
        ].into_iter()
        .filter(|(_, show)| *show)
        .map(|(v, _)| v)
        .collect();

        let (v, mat) = if dirs.len() >= 4 {
            return (0.0, self.empty.clone());
        } else if dirs.len() == 3 {
            if dirs[0].dot(dirs[1]) < 1e-3 {
                (dirs[0], self.triple.clone())
            } else {
                (dirs[2], self.triple.clone())
            }
        } else if dirs.len() == 2 {
            if dirs[0].dot(dirs[1]) < 1e-3 {
                (dirs[0], self.polar.clone())
            } else if dirs[0].cross(dirs[1]).z > 0.0 {
                (dirs[1], self.capital_l.clone())
            } else {
                (dirs[0], self.capital_l.clone())
            }
        } else if dirs.len() == 1 {
            (dirs[0], self.single.clone())
        } else {
            // TODO(@mxgrey): Somehow visualize the fact that the cell is a
            // permanent inescapable sink.
            return (0.0, self.empty.clone())
        };

        let yaw = f32::atan2(v.y, v.x);
        (yaw, mat)
    }

    fn iter(&self) -> [&Handle<StandardMaterial>; 5] {
        [
            &self.empty,
            &self.single,
            &self.capital_l,
            &self.polar,
            &self.triple,
        ]
    }
}

// TODO(@mxgrey): Come up with a way to incrementally change the cells instead
// of always wiping them clean and redrawing them.
fn create_passage_cells(
    commands: &mut Commands,
    for_passage: Entity,
    length: f32,
    cells: &PassageCells,
    alignment: &PassageAlignment,
    site_assets: &SiteAssets,
    graph_material: &Handle<StandardMaterial>,
    materials: &mut Assets<StandardMaterial>,
    textures: &CompassTextures,
) -> PassageSkeleton {
    let transform = compute_passage_alignment_transform(cells, alignment);
    let mut entity_commands = commands.spawn(SpatialBundle {
        transform,
        ..default()
    });

    let mesh = site_assets.unit_square_flat_mesh.clone();
    // TODO(@mxgrey): Do something smarter than unwrapping here
    let color = materials.get(graph_material).unwrap().base_color;
    let compass = CompassMaterials::new(materials, textures, color);

    if cells.cell_size < 1e-5 {
        // TODO(@mxgrey): Print an error/warning to the console here
        return PassageSkeleton::empty(entity_commands.id(), mesh.clone(), compass);
    }

    let columns = usize::min(cells.lanes, 1);
    let rows = [
        -cells.overflow[0],
        i32::max((length / cells.cell_size) as i32 + cells.overflow[1], 1),
    ];

    entity_commands.add_children(|parent| {
        for column in 0..cells.lanes as i32 {
            for row in rows[0]..rows[1] {
                let x = cells.cell_size * (row as f32 + 0.5);
                let y = cells.cell_size * (column as f32 + 0.5);
                let (yaw, material) = compass.orient(
                    cells.constraints.get(&[row, column])
                    .unwrap_or(&cells.default_constraints)
                );

                parent.spawn(PbrBundle {
                    mesh: mesh.clone(),
                    transform: Transform::from_translation(Vec3::new(x, y, 0.0))
                        .with_rotation(Quat::from_axis_angle(Vec3::Z, yaw))
                        .with_scale(Vec3::splat(cells.cell_size)),
                    material,
                    ..default()
                })
                .insert(CellTag { coords: [row, column], for_passage });
            }
        }
    });

    PassageSkeleton::new(entity_commands.id(), mesh, compass, rows, columns)
}

#[must_use]
fn update_passage_geometry(
    commands: &mut Commands,
    skeleton: &mut PassageSkeleton,
    for_passage: Entity,
    length: f32,
    cells: &PassageCells,
    alignment: &PassageAlignment,
    q_cell: &mut Query<(&CellTag, &mut Visibility, &mut Handle<StandardMaterial>, &mut Transform), (With<CellTag>, Without<PassageCells>)>,
    children: &Query<&Children>,
) -> Transform {
    let transform = compute_passage_alignment_transform(cells, alignment);
    if cells.cell_size < 1e-5 {
        // We don't support such small cells, so just skip changing anything.
        // TODO(@mxgrey): Report this as a dev error. Also introduce some
        // fallback visual to make the situation clear to the user.
        return transform;
    }

    let new_columns = usize::min(cells.lanes, 1);
    let new_rows = [
        -cells.overflow[0],
        i32::max((length / cells.cell_size) as i32 + cells.overflow[1], 1),
    ];

    let create_cell = |column: i32, row: i32, parent: &mut ChildBuilder| {
        let x = cells.cell_size * (row as f32 + 0.5);
        let y = cells.cell_size * (column as f32 + 0.5);
        let (yaw, material) = skeleton.compass.orient(
            cells.constraints.get(&[row, column])
            .unwrap_or(&cells.default_constraints)
        );

        parent.spawn(PbrBundle {
            mesh: skeleton.mesh.clone(),
            transform: Transform::from_translation(Vec3::new(x, y, 0.0))
                .with_rotation(Quat::from_axis_angle(Vec3::Z, yaw))
                .with_scale(Vec3::splat(cells.cell_size)),
            material,
            ..default()
        })
        .insert(CellTag { coords: [row, column], for_passage });
    };

    commands.entity(skeleton.cell_group).add_children(|parent| {
        for column in skeleton.columns_extent as i32..new_columns as i32 {
            for row in skeleton.rows_extent[0]..skeleton.rows_extent[1] {
                create_cell(column, row, parent);
            }
        }

        let previous_rows_extent = skeleton.rows_extent;
        skeleton.columns_extent = usize::max(skeleton.columns_extent, new_columns);
        skeleton.rows_extent[0] = i32::min(skeleton.rows_extent[0], new_rows[0]);
        skeleton.rows_extent[1] = i32::max(skeleton.rows_extent[1], new_rows[1]);
        for column in 0..skeleton.columns_extent as i32 {
            for row in skeleton.rows_extent[0]..previous_rows_extent[0] {
                create_cell(column, row, parent);
            }

            for row in previous_rows_extent[1]..skeleton.rows_extent[1] {
                create_cell(column, row, parent);
            }
        }
    });

    if new_rows != skeleton.rows || new_columns != skeleton.columns {
        if let Ok(children) = children.get(skeleton.cell_group) {
            for child in children {
                let Ok((cell, mut vis, mut mat, mut tf)) = q_cell.get_mut(*child) else { continue };
                let [x, y] = cell.coords;
                let visible = new_rows[0] <= x && x < new_rows[1] && y < new_columns as i32;
                if vis.is_visible != visible {
                    vis.is_visible = visible;
                }

                if !vis.is_visible {
                    continue;
                }

                let (new_yaw, new_material) = skeleton.compass.orient(
                    cells.constraints.get(&[x, y])
                    .unwrap_or(&cells.default_constraints)
                );

                let orientation = Quat::from_axis_angle(Vec3::Z, new_yaw);
                if tf.rotation.angle_between(orientation) > 1e-2 {
                    tf.rotation = orientation;
                }

                if f32::abs(tf.scale[0] - cells.cell_size) > 1e-3 {
                    tf.scale = Vec3::splat(cells.cell_size);
                }

                if mat.id() != new_material.id() {
                    *mat = new_material;
                }
            }
        }
    }

    skeleton.rows = new_rows;
    skeleton.columns = new_columns;

    return transform;
}

pub fn add_passage_visuals(
    mut commands: Commands,
    passages: Query<
        (
            Entity,
            &Edge<Entity>,
            &AssociatedGraphs<Entity>,
            &PassageCells,
            &PassageAlignment,
        ), Added<PassageCells>
    >,
    graphs: GraphSelect,
    should_display: ShouldDisplayGraph,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    textures: Res<CompassTextures>,
    site_assets: Res<SiteAssets>,
) {
    for (e, edge, associated_graphs, cells, alignment) in &passages {
        for anchor in &edge.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }

        let (graph_material, height) = graphs.display_style(associated_graphs);
        let is_visible = should_display.edge(edge, associated_graphs);

        let start_anchor = anchors
            .point_in_parent_frame_of(edge.start(), Category::Passage, e)
            .unwrap();
        let end_anchor = anchors
            .point_in_parent_frame_of(edge.end(), Category::Passage, e)
            .unwrap();
        let skeleton = create_passage_cells(
            &mut commands,
            e,
            (start_anchor - end_anchor).length(),
            cells,
            alignment,
            &site_assets,
            &graph_material,
            &mut materials,
            &textures,
        );
        let mut entity_commands = commands.entity(e);
        entity_commands.insert(SpatialBundle {
            transform: compute_passage_frame_transform(&start_anchor, &end_anchor, height),
            visibility: Visibility { is_visible },
            ..default()
        })
        .add_child(skeleton.cell_group)
        .insert(skeleton)
        .insert(Category::Passage)
        .insert(EdgeLabels::StartEnd);

    }
}

pub fn update_passage_visuals(
    mut passage_transforms: Query<(
        Entity,
        &Edge<Entity>,
        &mut Transform
    ), With<PassageCells>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>
        ),
    >,
    mut commands: Commands,
    changed_cells: Query<Entity, Changed<PassageCells>>,
    mut passages: Query<(&Edge<Entity>, &PassageCells, &PassageAlignment, &mut PassageSkeleton)>,
    mut q_cell: Query<(&CellTag, &mut Visibility, &mut Handle<StandardMaterial>, &mut Transform), (With<CellTag>, Without<PassageCells>)>,
    mut cell_group_transform: Query<&mut Transform, (Without<CellTag>, Without<PassageCells>)>,
    children: Query<&Children>,
) {
    let mut update_for_changed_anchors: SmallVec<[_; 8]> = SmallVec::new();
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, edge, mut tf)) = passage_transforms.get_mut(*dependent) {
                let start_anchor = anchors
                    .point_in_parent_frame_of(edge.start(), Category::Passage, e)
                    .unwrap();
                let end_anchor = anchors
                    .point_in_parent_frame_of(edge.end(), Category::Passage, e)
                    .unwrap();
                *tf = compute_passage_frame_transform(&start_anchor, &end_anchor, tf.translation.z);
                update_for_changed_anchors.push(e);
            }
        }
    }

    for e in changed_cells.iter().chain(update_for_changed_anchors.into_iter()) {
        if let Ok((edge, cells, alignment, mut skeleton)) = passages.get_mut(e) {
            let start_anchor = anchors
                .point_in_parent_frame_of(edge.start(), Category::Passage, e)
                .unwrap();
            let end_anchor = anchors
                .point_in_parent_frame_of(edge.end(), Category::Passage, e)
                .unwrap();

            let new_tf = update_passage_geometry(
                &mut commands,
                &mut skeleton,
                e,
                (start_anchor - end_anchor).length(),
                cells,
                alignment,
                &mut q_cell,
                &children,
            );

            if let Ok(mut tf) = cell_group_transform.get_mut(skeleton.cell_group) {
                *tf = new_tf;
            }
        }
    }
}

pub fn update_passage_for_changed_alignment(
    passages: Query<(&PassageCells, &PassageAlignment, &PassageSkeleton), Or<(Changed<PassageCells>, Changed<PassageAlignment>)>>,
    mut transforms: Query<&mut Transform>,
) {
    for (cells, alignment, skeleton) in &passages {
        let Ok(mut tf) = transforms.get_mut(skeleton.cell_group) else { continue };
        *tf = compute_passage_alignment_transform(cells, alignment);
    }
}

fn update_cell_material(
    skeleton: &PassageSkeleton,
    graph_material: &Handle<StandardMaterial>,
    materials: &mut Assets<StandardMaterial>,
) {
    let color = materials.get(graph_material).unwrap().base_color;
    for handle in skeleton.compass.iter() {
        if let Some(mat) = materials.get_mut(handle) {
            mat.base_color = color;
        }
    }
}

pub fn update_visibility_for_passages(
    mut passages: Query<
        (
            &Edge<Entity>,
            &AssociatedGraphs<Entity>,
            &PassageSkeleton,
            &mut Visibility,
        ),
        Without<NavGraphMarker>
    >,
    should_display: ShouldDisplayGraph,
    passages_with_changed_association: Query<Entity, Changed<AssociatedGraphs<Entity>>>,
    graph_changed_visibility: Query<
        (),
        (
            With<NavGraphMarker>,
            Or<(Changed<Visibility>, Changed<RecencyRank<NavGraphMarker>>)>
        ),
    >,
    mut transforms: Query<&mut Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    removed: RemovedComponents<NavGraphMarker>,
) {
    let graph_change = !graph_changed_visibility.is_empty() || removed.iter().next().is_some();
    let update_all = should_display.current_level.is_changed() || graph_change;
    if update_all {
        for (edge, associated, _, mut visibility) in &mut passages {
            let is_visible = should_display.edge(edge, associated);
            if visibility.is_visible != is_visible {
                visibility.is_visible = is_visible;
            }
        }
    } else {
        for e in &passages_with_changed_association {
            if let Ok((edge, associated, _, mut visibility)) = passages.get_mut(e) {
                let is_visible = should_display.edge(edge, associated);
                if visibility.is_visible != is_visible {
                    visibility.is_visible = is_visible;
                }
            }
        }
    }

    if graph_change {
        for (_, associated_graphs, skeleton, _) in &mut passages {
            let (mat, height) = should_display.graphs.display_style(associated_graphs);
            update_cell_material(
                skeleton,
                &mat,
                &mut materials,
            );

            if let Ok(mut tf) = transforms.get_mut(skeleton.cell_group) {
                tf.translation.z = height;
            }
        }
    } else {
        for e in &passages_with_changed_association {
            let Ok((_, associated_graphs, skeleton, _)) = passages.get_mut(e) else { continue };
            let (mat, height) = should_display.graphs.display_style(associated_graphs);
            update_cell_material(
                skeleton,
                &mat,
                &mut materials,
            );

            if let Ok(mut tf) = transforms.get_mut(skeleton.cell_group) {
                tf.translation.z = height;
            }
        }
    }
}
