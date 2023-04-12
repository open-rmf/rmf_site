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

#[derive(Component)]
pub struct PassageSkeleton {
    pub cell_group: Entity,
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

struct CompassMaterials<'a> {
    materials: &'a mut Assets<StandardMaterial>,
    textures: &'a CompassTextures,
    color: Color,
    empty: Option<Handle<StandardMaterial>>,
    single: Option<Handle<StandardMaterial>>,
    capital_l: Option<Handle<StandardMaterial>>,
    polar: Option<Handle<StandardMaterial>>,
    triple: Option<Handle<StandardMaterial>>,
}

impl<'a> CompassMaterials<'a> {
    fn new(
        materials: &'a mut Assets<StandardMaterial>,
        textures: &'a CompassTextures,
        color: Color,
    ) -> Self {
        Self {
            materials,
            textures,
            color,
            empty: None,
            single: None,
            capital_l: None,
            polar: None,
            triple: None,
        }
    }

    fn orient(&mut self, constraints: &CellConstraints) -> (f32, Handle<StandardMaterial>) {
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
            return (0.0, self.get_empty());
        } else if dirs.len() == 3 {
            if dirs[0].dot(dirs[1]) < 1e-3 {
                (dirs[0], self.get_triple())
            } else {
                (dirs[2], self.get_triple())
            }
        } else if dirs.len() == 2 {
            if dirs[0].dot(dirs[1]) < 1e-3 {
                (dirs[0], self.get_polar())
            } else if dirs[0].cross(dirs[1]).z > 0.0 {
                (dirs[1], self.get_capital_l())
            } else {
                (dirs[0], self.get_capital_l())
            }
        } else if dirs.len() == 1 {
            (dirs[0], self.get_single())
        } else {
            // TODO(@mxgrey): Somehow visualize the fact that the cell is a
            // permanent inescapable sink.
            return (0.0, self.get_empty())
        };

        let yaw = f32::atan2(v.y, v.x);
        (yaw, mat)
    }

    fn get_empty(&mut self) -> Handle<StandardMaterial> {
        if let Some(empty) = &self.empty {
            return empty.clone();
        }

        let mat = self.make_mat(self.textures.empty.clone());
        self.empty = Some(mat.clone());
        mat
    }

    fn get_single(&mut self) -> Handle<StandardMaterial> {
        if let Some(single) = &self.single {
            return single.clone();
        }

        let mat = self.make_mat(self.textures.single.clone());
        self.single = Some(mat.clone());
        mat
    }

    fn get_polar(&mut self) -> Handle<StandardMaterial> {
        if let Some(polar) = &self.polar {
            return polar.clone();
        }

        let mat = self.make_mat(self.textures.polar.clone());
        self.polar = Some(mat.clone());
        mat
    }

    fn get_capital_l(&mut self) -> Handle<StandardMaterial> {
        if let Some(capital_l) = &self.capital_l {
            return capital_l.clone();
        }

        let mat = self.make_mat(self.textures.capital_l.clone());
        self.capital_l = Some(mat.clone());
        mat
    }

    fn get_triple(&mut self) -> Handle<StandardMaterial> {
        if let Some(triple) = &self.triple {
            return triple.clone();
        }

        let mat = self.make_mat(self.textures.triple.clone());
        self.triple = Some(mat.clone());
        mat
    }

    fn make_mat(&mut self, texture: Handle<Image>) -> Handle<StandardMaterial> {
        self.materials.add(StandardMaterial {
            base_color: self.color,
            base_color_texture: Some(texture.clone()),
            ..default()
        })
    }
}

fn create_passage_cells(
    commands: &mut Commands,
    length: f32,
    cells: &PassageCells,
    alignment: &PassageAlignment,
    graph_material: &Handle<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    textures: &CompassTextures,
) -> Option<Entity> {
    if cells.cell_size < 1e-5 {
        // TODO(@mxgrey): Print an error/warning to the console here
        return None;
    }

    let color = if let Some(graph_material) = materials.get(&graph_material) {
        graph_material.base_color
    } else {
        return None;
    };

    let transform = compute_passage_alignment_transform(cells, alignment);

    let rows = (length / cells.cell_size) as i32;
    let mut entity_commands = commands.spawn(SpatialBundle {
        transform,
        ..default()
    });

    let mut compass = CompassMaterials::new(materials, textures, color);

    entity_commands.add_children(|parent| {
        let mesh = meshes.add(
            make_flat_square_mesh(cells.cell_size)
            .transform_by(Affine3A::from_translation(Vec3::new(
                cells.cell_size/2.0,
                cells.cell_size/2.0,
                0.0,
            )))
            .into()
        );
        for column in 0..cells.lanes as i32 {
            for row in -cells.overflow[0]..(rows + cells.overflow[1]) {
                let x = cells.cell_size * row as f32;
                let y = cells.cell_size * column as f32;
                let (yaw, material) = compass.orient(
                    cells.constraints.get(&[row, column])
                    .unwrap_or(&cells.default_constraints)
                );

                parent.spawn(PbrBundle {
                    mesh: mesh.clone(),
                    transform: Transform::from_translation(Vec3::new(x, y, 0.0))
                        .with_rotation(Quat::from_axis_angle(Vec3::Z, yaw)),
                    material,
                    ..default()
                });
            }
        }
    });

    Some(entity_commands.id())
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    textures: Res<CompassTextures>,
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
        let mut entity_commands = commands.entity(e);
        entity_commands.insert(SpatialBundle {
            visibility: Visibility { is_visible },
            ..default()
        });
        create_passage_cells(
            &mut commands,
            (start_anchor - end_anchor).length(),
            cells,
            alignment,
            &graph_material,
            &mut meshes,
            &mut materials,
            &textures,
        );

    }
}
