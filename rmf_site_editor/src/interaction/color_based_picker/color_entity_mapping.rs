use bevy::{prelude::*, render::view::RenderLayers, utils::hashbrown::HashMap};
use bevy_points::prelude::PointsMaterial;
use bevy_polyline::prelude::*;

use std::collections::BTreeMap;
use std::process::Child;

use crate::interaction::{DontPropagateVisualCue, LINE_PICKING_LAYER, POINT_PICKING_LAYER};
use crate::site::PointAsset;

use super::ImageToSave;

const SCREEN_SPACE_POINT_SIZE_SELECTION: f32 = 40.0;

#[derive(Debug, Clone)]
pub struct ScreenspacePolyline {
    pub start: Vec3,
    pub end: Vec3,
    thickness: f32,
}

/// Label items which need to be selected.
#[derive(Component, Debug, Clone)]
pub enum ScreenSpaceSelection {
    Polyline(ScreenspacePolyline),
    Point,
}

impl ScreenSpaceSelection {
    pub fn polyline(start_anchor: Vec3, end_anchor: Vec3, thickness: f32) -> Self {
        Self::Polyline(ScreenspacePolyline {
            start: start_anchor,
            end: end_anchor,
            thickness: thickness,
        })
    }
}

/// Label items which need to be selected.
#[derive(Resource, Debug, Clone, Default)]
pub struct ColorEntityMap {
    entity_to_polyline_material_map: HashMap<Entity, Handle<PolylineMaterial>>,
    entity_to_point_material_map: HashMap<Entity, Handle<PointsMaterial>>,
    color_to_entity_map: BTreeMap<(u8, u8, u8), Entity>,
}

impl ColorEntityMap {
    fn allocate_new_polyline_color(
        &mut self,
        entity: &Entity,
        polyline_materials: &mut ResMut<Assets<PolylineMaterial>>,
        thickness: f32,
    ) -> Handle<PolylineMaterial> {
        // TODO(arjo): This takes indeterminate amount of time. Just use a counter
        let mut r = rand::random::<u8>();
        let mut g = rand::random::<u8>();
        let mut b = rand::random::<u8>();

        while self.color_to_entity_map.get(&(r, g, b)).is_some()
            || (r == u8::MAX && g == u8::MAX && b == u8::MAX)
        {
            r = rand::random::<u8>();
            g = rand::random::<u8>();
            b = rand::random::<u8>();
        }
        self.color_to_entity_map.insert((r, g, b), *entity);

        let color = Color::rgb_u8(r, g, b);

        let material = polyline_materials.add(PolylineMaterial {
            width: thickness,
            color,
            perspective: false,
            ..default()
        });

        self.entity_to_polyline_material_map
            .insert(*entity, material.clone());

        material
    }

    fn allocate_new_point_material(
        &mut self,
        entity: &Entity,
        point_materials: &mut ResMut<Assets<PointsMaterial>>,
    ) -> Handle<PointsMaterial> {
        let mut r = rand::random::<u8>();
        let mut g = rand::random::<u8>();
        let mut b = rand::random::<u8>();

        while self.color_to_entity_map.get(&(r, g, b)).is_some()
            || (r == u8::MAX && g == u8::MAX && b == u8::MAX)
        {
            r = rand::random::<u8>();
            g = rand::random::<u8>();
            b = rand::random::<u8>();
        }
        self.color_to_entity_map.insert((r, g, b), *entity);

        let color = Color::rgb_u8(r, g, b);

        let material = point_materials.add(PointsMaterial {
            point_size: SCREEN_SPACE_POINT_SIZE_SELECTION, // Defines the size of the points.
            perspective: false, // Specify whether points' size is attenuated by the camera depth.
            circle: true,
            use_vertex_color: false,
            color,
            ..default()
        });

        self.entity_to_point_material_map
            .insert(*entity, material.clone());

        material
    }

    pub fn get_polyline_material(
        &mut self,
        entity: &Entity,
        polyline_materials: &mut ResMut<Assets<PolylineMaterial>>,
        thickness: f32,
    ) -> Handle<PolylineMaterial> {
        if let Some(color) = self.entity_to_polyline_material_map.get(entity) {
            color.clone()
        } else {
            self.allocate_new_polyline_color(entity, polyline_materials, thickness)
        }
    }

    pub fn get_points_material(
        &mut self,
        entity: &Entity,
        points_material: &mut ResMut<Assets<PointsMaterial>>,
    ) -> Handle<PointsMaterial> {
        if let Some(color) = self.entity_to_point_material_map.get(entity) {
            color.clone()
        } else {
            self.allocate_new_point_material(entity, points_material)
        }
    }

    pub fn get_entity(&self, key: &(u8, u8, u8)) -> Option<&Entity> {
        self.color_to_entity_map.get(key)
    }
}

/// Label items which need to be selected.
#[derive(Component, Debug, Clone)]
pub struct MarkAsDrawnToSelectionBuffer<const Layer: u8>;

/// Label entities whic are part of the selection buffer
#[derive(Component, Debug, Clone)]
pub struct SelectionBufferDrawing;

/// This system handles drawing new entities in the selection
pub fn new_objectcolor_entity_mapping<const Layer: u8>(
    mut commands: Commands,
    screen_space_lines: Query<
        (&ScreenSpaceSelection, Entity),
        Without<MarkAsDrawnToSelectionBuffer<Layer>>,
    >,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut point_materials: ResMut<Assets<PointsMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut color_map: ResMut<ColorEntityMap>,
    images_to_save: Query<&ImageToSave<Layer>>,
    point_assets: Res<PointAsset>,
) {
    let scale = if let Ok(image_parameters) = images_to_save.get_single() {
        image_parameters.3
    } else {
        1.0
    };

    // Redraw parameters.
    for (screenspace_shape, entity) in &screen_space_lines {
        match screenspace_shape {
            ScreenSpaceSelection::Polyline(shape) => {
                let thickness = shape.thickness * scale;
                let thickness = if thickness < 10.0 { 10.0 } else { thickness };
                if Layer == LINE_PICKING_LAYER {
                    commands.entity(entity).with_children(|parent| {
                        parent.spawn((
                            PolylineBundle {
                                polyline: polylines.add(Polyline {
                                    vertices: vec![shape.start, shape.end],
                                }),
                                material: color_map.get_polyline_material(
                                    &entity,
                                    &mut polyline_materials,
                                    thickness,
                                ),
                                ..default()
                            },
                            RenderLayers::layer(LINE_PICKING_LAYER),
                            DontPropagateVisualCue,
                            SelectionBufferDrawing,
                        ));
                    });
                    commands
                        .entity(entity)
                        .insert(MarkAsDrawnToSelectionBuffer::<LINE_PICKING_LAYER>);
                }
            }
            ScreenSpaceSelection::Point => {
                if Layer == POINT_PICKING_LAYER {
                    commands.entity(entity).with_children(|parent| {
                        parent.spawn((
                            MaterialMeshBundle {
                                mesh: point_assets.bevy_point_mesh.clone(),
                                material: color_map
                                    .get_points_material(&entity, &mut point_materials)
                                    .clone(),
                                ..default()
                            },
                            RenderLayers::layer(POINT_PICKING_LAYER),
                            DontPropagateVisualCue,
                            SelectionBufferDrawing,
                        ));
                    });
                    commands
                        .entity(entity)
                        .insert(MarkAsDrawnToSelectionBuffer::<POINT_PICKING_LAYER>);
                }
            }
        }
    }
}

/// This system synchronizes the polylines in the selection buffer
/// and the rendering system.
pub fn sync_polyline_selection_buffer(
    screen_space_lines: Query<
        (&ScreenSpaceSelection, &Children),
        With<MarkAsDrawnToSelectionBuffer<LINE_PICKING_LAYER>>,
    >,
    polylines: Query<(&Handle<Polyline>, &SelectionBufferDrawing, &RenderLayers)>,
    mut polyline_assets: ResMut<Assets<Polyline>>,
) {
    for (selection, children) in screen_space_lines.iter() {
        let ScreenSpaceSelection::Polyline(line) = selection else {
            continue;
        };

        for child in children.iter() {
            let Ok((handle, _, layers)) = polylines.get(*child) else {
                continue;
            };
            let Some(mut polyline) = polyline_assets.get_mut(handle) else {
                continue;
            };
            polyline.vertices = vec![line.start, line.end];
        }
    }
}
