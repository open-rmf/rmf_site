use bevy::{prelude::*, render::view::RenderLayers, utils::hashbrown::HashMap};
use bevy_points::prelude::PointsMaterial;
use bevy_polyline::prelude::*;

use std::collections::BTreeMap;

use crate::interaction::{LINE_PICKING_LAYER, POINT_PICKING_LAYER};
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
        println!("Handling new color {} {} {}", r, g, b);
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
pub struct ScreenSpaceEntity<const Layer: u8>;

// TODO(arjo): Split off into 2 systems?
pub fn screenspace_selection_system<const Layer: u8>(
    mut commands: Commands,
    screen_space_lines: Query<(&ScreenSpaceSelection, Entity, Option<&GlobalTransform>)>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut point_materials: ResMut<Assets<PointsMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut color_map: ResMut<ColorEntityMap>,
    screen_space_entities: Query<(&ScreenSpaceEntity<Layer>, Entity)>,
    images_to_save: Query<&ImageToSave<Layer>>,
    point_assets: Res<PointAsset>,
) {
    // TODO(arjo) Make these children of relevent entities instead of
    // this expensive operation
    for (_, entity) in &screen_space_entities {
        commands.entity(entity).despawn();
    }

    let scale = if let Ok(image_parameters) = images_to_save.get_single() {
        image_parameters.3
    } else {
        1.0
    };

    // Redraw parameters.
    for (screenspace_shape, entity, tf) in &screen_space_lines {
        match screenspace_shape {
            ScreenSpaceSelection::Polyline(shape) => {
                let thickness = shape.thickness * scale;
                let thickness = if thickness < 10.0 { 10.0 } else { thickness };
                commands.spawn((
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
                    ScreenSpaceEntity::<LINE_PICKING_LAYER>,
                ));
            }
            ScreenSpaceSelection::Point => {
                let Some(tf) = tf else {
                    continue;
                };

                commands.spawn((
                    MaterialMeshBundle {
                        mesh: point_assets.bevy_point_mesh.clone(),
                        material: color_map
                            .get_points_material(&entity, &mut point_materials)
                            .clone(),
                        transform: Transform::from_translation(tf.translation()), // TODO(arjo): Remove after parenting
                        ..default()
                    },
                    RenderLayers::layer(POINT_PICKING_LAYER),
                    ScreenSpaceEntity::<POINT_PICKING_LAYER>,
                ));
            }
        }
    }
}
