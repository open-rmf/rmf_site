use bevy::{prelude::*, render::mesh::VertexAttributeValues, ecs::world};
use bevy_egui::{
    egui::{self, Button, CollapsingHeader, Sense, panel, Slider},
    EguiContext,
};
use bevy_mod_raycast::Ray3d;
use bevy_rapier3d::na::Rotation;
use camera_controls::{CameraControls, ProjectionMode};
use rmf_site_format::{Anchor, GeoReference, geo_reference, AssetSource};
use utm::*;
use std::{f32::consts::PI, collections::HashSet, ops::RangeInclusive};

use crate::{interaction::{Selected, PickingBlockers, camera_controls}, OSMTile, generate_map_tiles};
pub struct GeoReferenceEvent{}

enum SelectionMode {
    AnchorSelected(Entity),
    AnchorSelect,
    NoSelection
}

impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::NoSelection
    }
}

#[derive(Component, Clone, Eq, PartialEq, Hash)]
pub struct MapTile(OSMTile);

fn selection_mode_labels(mode: &SelectionMode) -> String {
    match mode {
        SelectionMode::AnchorSelected(entity) => {
            format!("Anchor {:?}", entity)
        },
        SelectionMode::AnchorSelect => {
            "Click the anchor you want to use".to_owned()
        },
        SelectionMode::NoSelection => {
            "Select Anchor".to_owned()
        }
    }
}

#[derive(Default, Resource)]
pub struct GeoReferencePanelState {
    enabled: bool,
    latitude: f32,
    longitude: f32,
    selection_mode1: SelectionMode,
    selection_mode2: SelectionMode
}

#[derive(Clone, Resource)]
pub struct GeoReferencePreviewState {
    anchor: (f32, f32),
    zoom: i32,
    enabled: bool
}

impl Default for GeoReferencePreviewState {
    fn default() -> Self {
        Self { 
            anchor: Default::default(), 
            zoom: 15, 
            enabled: Default::default() 
        }
    }
}

pub fn add_georeference(
    mut georef_anchors: Query<(&Anchor, &GeoReference<Entity>, Entity)>,
    selected_anchors: Query<(&Anchor, &Selected, &GlobalTransform, Entity)>,
    mut panel_state: Local<GeoReferencePanelState>,
    mut egui_context: ResMut<EguiContext>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut preview_state: ResMut<GeoReferencePreviewState>,
    asset_server: Res<AssetServer>,
    mut geo_events: EventReader<GeoReferenceEvent>,
    mut commands: Commands) {

    for _event in geo_events.iter() {
        panel_state.enabled = true;
    }

    let selected: Vec<_> = selected_anchors.iter().filter(|(_anchor, selected, _transform, _entity)| {
        selected.is_selected
    }).collect();

    if panel_state.enabled
    {
        // Draw UI
        egui::Window::new("Geographic Reference").show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Reference Anchor: ");
                if ui.button(selection_mode_labels(&panel_state.selection_mode1)).clicked() {
                    if selected.len() == 0 {
                        panel_state.selection_mode1 = SelectionMode::AnchorSelect;
                    }
                    else {
                        panel_state.selection_mode1 = SelectionMode::AnchorSelected(selected[0].3);
                        let translation = selected[0].2.translation();
                        let (lat, lon) = 
                            world_to_latlon(translation, preview_state.anchor).unwrap();
                        println!("Anchor at {:?}", (lat, lon));
                        panel_state.latitude = lat as f32;
                        panel_state.longitude = lon as f32;
                    }
                }
                ui.label("Latitude: ");
                ui.add(egui::DragValue::new(&mut panel_state.latitude).speed(1e-16));
                ui.label("Longitude: ");
                ui.add(egui::DragValue::new(&mut panel_state.longitude).speed(1e-16));
                if ui.button("Make Reference").clicked() {
                    // Recalculate reference point
                    if selected.len() == 1 {
                        let global_transform = selected[0].2;
                        let translation = global_transform.translation();
                        let zone = lat_lon_to_zone_number(panel_state.latitude as f64, panel_state.longitude as f64);
                        let (northing, easting, _)= to_utm_wgs84(panel_state.latitude as f64, panel_state.longitude as f64, zone);
                        let utm_origin = (easting - translation.x as f64, northing - translation.x as f64);
                        let (lat, lon) = wsg84_utm_to_lat_lon(utm_origin.0, utm_origin.1, zone, lat_to_zone_letter(panel_state.latitude.into()).unwrap()).unwrap();
                        preview_state.anchor = (lat as f32, lon as f32);
                    }
                }
                if ui.button("Move to lat/lon").clicked() {
                    panel_state.selection_mode1 = SelectionMode::AnchorSelected(selected[0].3);
                    let translation = selected[0].2.translation();
                }
            });

            if selected.len() != 0 && matches!(panel_state.selection_mode2, SelectionMode::AnchorSelect) {
                panel_state.selection_mode1 = SelectionMode::AnchorSelected(selected[0].3);
                let translation = selected[0].2.translation();
                let (lat, lon) = 
                    world_to_latlon(translation, preview_state.anchor).unwrap();
                println!("Anchor at {:?}", (lat, lon));
                panel_state.latitude = lat as f32;
                panel_state.longitude = lon as f32;
            }

            ui.horizontal(|ui| {
                ui.label("Map Resolution: ");
                ui.add(Slider::new(&mut preview_state.zoom,RangeInclusive::new(13,19)));
                if ui.button("Preview Map").clicked() {
                    preview_state.enabled = true;
                }
            });
            
        });
    }
}

fn spawn_tile(mut meshes: &mut ResMut<Assets<Mesh>>,
    mut materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    mut commands: &mut Commands,
    coordinates: (f32, f32),
    reference: (f32, f32),
    zoom: i32
    ) {
    let tile = OSMTile::from_latlon(zoom, coordinates.0, coordinates.1);
    let tile_size = tile.tile_size();

    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        tile_size.0,
        tile_size.1,
    ))));
    
    let texture_handle: Handle<Image> = asset_server.load(String::from(
        &AssetSource::OSMSlippyMap(tile.zoom(), coordinates.0, coordinates.1)));
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let tile_offset = latlon_to_world(coordinates.0, coordinates.1, reference);
    commands.spawn(PbrBundle {
        mesh: quad_handle,
        material: material_handle,
        transform: Transform::from_xyz(
            tile_offset.x,
            tile_offset.y,
            0.0,
        ),
        ..default()
    }).insert(MapTile(tile));
}

pub fn world_to_latlon(world_coordinates: Vec3, anchor: (f32, f32)) -> Result<(f64, f64), WSG84ToLatLonError> {
    let mut zone = lat_lon_to_zone_number(anchor.0.into(), anchor.1.into());
    let mut zone_letter = lat_to_zone_letter(anchor.0.into());
    let utm_offset = to_utm_wgs84(anchor.0.into(), anchor.1.into(), zone);
    let mut easting = world_coordinates.x as f64 + utm_offset.1;
    let mut northing = world_coordinates.y as f64 + utm_offset.0;

    // A really wrong way of measuring stuff. TODO: Handle case where easting
    // and northing are out of bounds. TBH I have no idea how to correctly
    // handle such cases.
    while northing < 0. {
        northing = 10000000. + northing;
        zone_letter = Some((zone_letter.unwrap() as u8 - 1) as char);
    }
    while northing > 10000000. {
        northing = northing - 10000000.;
        zone_letter = Some((zone_letter.unwrap() as u8 + 1) as char);
    }

    while easting < 100000. {
        easting =  1000000. + (100000. - easting);
        zone += 1;
    }

    while easting > 1000000. {
        easting -= 1000000.;
        zone -= 1;
    }
    return wsg84_utm_to_lat_lon(easting, northing, zone, zone_letter.unwrap());
}

pub fn latlon_to_world(lat: f32, lon: f32, anchor: (f32, f32)) -> Vec3 {
    let zone = lat_lon_to_zone_number(anchor.0.into(), anchor.1.into());
    let utm_offset = to_utm_wgs84(anchor.0.into(), anchor.1.into(), zone);
    let utm_candidate = to_utm_wgs84(lat as f64, lon as f64, zone);
    Vec3::new((utm_candidate.1 - utm_offset.1) as f32, (utm_candidate.0 - utm_offset.0) as f32, 0.0)
}

pub fn render_map_tiles(
    mut map_tiles: Query<(Entity, &MapTile)>,
    mut cameras: Query<(&Camera, &GlobalTransform)>,
    camera_controls: Res<CameraControls>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    preview_state: Res<GeoReferencePreviewState>,
    mut commands: Commands
)
{
    if !preview_state.enabled {
        for (entity, _tile) in &map_tiles {
            commands.entity(entity).despawn();
        }
        return;
    }

    let cam_entity = match camera_controls.mode() {
        ProjectionMode::Perspective => {
            camera_controls.perspective_camera_entities[0]
        },
        ProjectionMode::Orthographic => {
            camera_controls.orthographic_camera_entities[0]
        }
    };

    let mut zoom_changed = false;
    let mut existing_tiles = HashSet::new();
    for (_entity, tile) in &map_tiles {
        if tile.0.zoom() != preview_state.zoom {
            zoom_changed = true;
        }
        existing_tiles.insert(tile.0.clone());        
    }

    if let Ok((camera, transform)) = cameras.get(cam_entity) {
        if let Some((viewport_min, viewport_max)) = camera.logical_viewport_rect() {
            let viewport_size = viewport_max - viewport_min;

            let top_left_ray = Ray3d::from_screenspace(Vec2::new(0.0, 0.0), camera, transform);
            let top_right_ray = Ray3d::from_screenspace(Vec2::new(viewport_size.x, 0.0), camera, transform);
            let bottom_left_ray = Ray3d::from_screenspace(Vec2::new(0.0, viewport_size.y), camera, transform);
            let bottom_right_ray = Ray3d::from_screenspace(viewport_size, camera, transform);

            let top_left = ray_groundplane_intersection(&top_left_ray);
            let top_right = ray_groundplane_intersection(&top_right_ray);
            let bottom_left = ray_groundplane_intersection(&bottom_left_ray);
            let bottom_right = ray_groundplane_intersection(&bottom_right_ray);

            let viewport_corners = [top_left, top_right, bottom_left, bottom_right];
            // Calculate AABB
            let min_x = viewport_corners.iter().map(|x| {x.x}).fold(f32::INFINITY, |x, val| if x < val {x} else {val});
            let max_x = viewport_corners.iter().map(|x| {x.x}).fold(-f32::INFINITY, |x, val| if x > val {x} else {val});

            let min_y = viewport_corners.iter().map(|x| {x.y}).fold(f32::INFINITY, |x, val| if x < val {x} else {val});
            let max_y = viewport_corners.iter().map(|x| {x.y}).fold(-f32::INFINITY, |x, val| if x > val {x} else {val});
        
            // TODO(arjo): Gracefully handle unwrap
            let latlon_start = world_to_latlon(Vec3::new(min_x, min_y, 0.0), preview_state.anchor).unwrap();
            let latlon_end = world_to_latlon(Vec3::new(max_x, max_y, 0.0), preview_state.anchor).unwrap();

            for tile in generate_map_tiles(latlon_start.0 as f32, latlon_start.1 as f32, latlon_end.0 as f32, latlon_end.1 as f32, preview_state.zoom) {
                if existing_tiles.contains(&tile) && !zoom_changed {
                   continue; 
                }

                spawn_tile(&mut meshes, &mut materials, &asset_server, &mut commands, tile.get_center(), preview_state.anchor, preview_state.zoom);
            }
        }

        if zoom_changed {
            for (entity, _tile) in &map_tiles{
                commands.entity(entity).despawn();
            }
        }

    }
}

fn ray_groundplane_intersection(ray: &Option<Ray3d>) -> Vec3 {
    if let Some(ray) = ray {
        let t =  - ray.origin().z / ray.direction().z;
        Vec3::new(ray.origin().x + t * ray.direction().x,
        ray.origin().y + t * ray.direction().y
        ,0.0)
    }
    else {
        Vec3::new(0.0, 0.0, 0.0)
    }
}

#[test]
fn test_groundplane() {
    let ray  = Ray3d::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 1.0));
        
    assert!(ray_groundplane_intersection(&Some(ray)).length() < 1e-5);
}