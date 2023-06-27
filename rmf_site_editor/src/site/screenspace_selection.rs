use bevy::{
    prelude::*,
    render::view::RenderLayers,
    utils::{hashbrown::HashMap, HashSet},
};
use bevy_polyline::prelude::*;

use std::collections::BTreeMap;

use crate::interaction::PICKING_LAYER;

use super::ImageToSave;

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
    entity_to_material_map: HashMap<Entity, Handle<PolylineMaterial>>,
    color_to_entity_map: BTreeMap<(u8, u8, u8), Entity>,
}

impl ColorEntityMap {
    fn allocate_new_color(
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

        self.entity_to_material_map
            .insert(*entity, material.clone());

        material
    }

    pub fn get_material(
        &mut self,
        entity: &Entity,
        polyline_materials: &mut ResMut<Assets<PolylineMaterial>>,
        thickness: f32,
    ) -> Handle<PolylineMaterial> {
        if let Some(color) = self.entity_to_material_map.get(entity) {
            color.clone()
        } else {
            self.allocate_new_color(entity, polyline_materials, thickness)
        }
    }

    pub fn get_entity(&self, key: &(u8, u8, u8)) -> Option<&Entity> {
        self.color_to_entity_map.get(key)
    }
}

/// Label items which need to be selected.
#[derive(Component, Debug, Clone)]
pub struct ScreenSpaceEntity;

pub fn screenspace_selection_system(
    mut commands: Commands,
    screen_space_lines: Query<(&ScreenSpaceSelection, Entity)>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut color_map: ResMut<ColorEntityMap>,
    screen_space_entities: Query<(&ScreenSpaceEntity, Entity)>,
    images_to_save: Query<&ImageToSave>,
) {
    // TODO(arjo) Perhaps perform some form of diff calculation to
    // Reduce ECS-churn.
    for (_, entity) in &screen_space_entities {
        commands.entity(entity).despawn();
    }

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
                commands.spawn((
                    PolylineBundle {
                        polyline: polylines.add(Polyline {
                            vertices: vec![shape.start, shape.end],
                        }),
                        material: color_map.get_material(
                            &entity,
                            &mut polyline_materials,
                            thickness,
                        ),
                        ..default()
                    },
                    RenderLayers::layer(PICKING_LAYER),
                    ScreenSpaceEntity,
                ));
            }
        }
    }
}
