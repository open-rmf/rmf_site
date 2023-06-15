use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{
    egui::{self, Slider},
    EguiContext,
};
use bevy_mod_raycast::Ray3d;
use camera_controls::{CameraControls, ProjectionMode};
use rmf_site_format::{Anchor, AssetSource, GeographicOffset, SiteProperties};
use std::{collections::HashSet, ops::RangeInclusive};
use utm::*;

use crate::{
    generate_map_tiles,
    interaction::{camera_controls, MoveTo, Selected},
    workspace::CurrentWorkspace,
    OSMTile,
};

const MAX_ZOOM: i32 = 19;
const MIN_ZOOM: i32 = 12;

#[derive(Debug, Clone)]
pub struct GeoReferenceSelectAnchorEvent {}

#[derive(Debug, Clone)]
pub struct GeoReferenceSetReferenceEvent;

#[derive(Debug, Clone)]
pub struct GeoReferenceViewReferenceEvent;

#[derive(Debug, Clone)]
pub struct GeoReferenceMoveEvent;

#[derive(SystemParam)]
pub struct GeoreferenceEventWriter<'w, 's> {
    pub select_anchor: EventWriter<'w, 's, GeoReferenceSelectAnchorEvent>,
    pub set_reference: EventWriter<'w, 's, GeoReferenceSetReferenceEvent>,
    pub view_reference: EventWriter<'w, 's, GeoReferenceViewReferenceEvent>,
    pub move_anchor: EventWriter<'w, 's, GeoReferenceMoveEvent>,
}

enum SelectionMode {
    AnchorSelected(Entity),
    AnchorSelect,
    NoSelection,
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
        }
        SelectionMode::AnchorSelect => "Click the anchor you want to use".to_owned(),
        SelectionMode::NoSelection => "Select Anchor".to_owned(),
    }
}

#[derive(Default, Resource)]
pub struct GeoReferencePanelState {
    enabled: bool,
    latitude: f32,
    longitude: f32,
    selection_mode: SelectionMode,
}

#[derive(Default)]
struct MoveAnchor {
    anchor: SelectionMode,
    lat: f32,
    lon: f32,
    visible: bool,
}

fn move_anchor(
    selected_anchors: Query<(&Anchor, &Selected, &GlobalTransform, Entity)>,
    geo_events: EventReader<GeoReferenceMoveEvent>,
    current_ws: Res<CurrentWorkspace>,
    site_properties: Query<(Entity, &SiteProperties)>,
    mut window: Local<MoveAnchor>,
    mut egui_context: ResMut<EguiContext>,
    mut move_commands: EventWriter<MoveTo>,
) {
    if geo_events.is_empty() && !window.visible {
        return;
    }

    if !window.visible {
        window.visible = true;
    }

    if let Some((_, properties)) = site_properties
        .iter()
        .filter(|(entity, _)| *entity == current_ws.root.unwrap())
        .nth(0)
    {
        if let Some(offset) = properties.geographic_offset {
            let offset = offset.anchor;
            let selected: Vec<_> = selected_anchors
                .iter()
                .filter(|(_anchor, selected, _transform, _entity)| selected.is_selected)
                .collect();

            egui::Window::new("Set Geographic Reference").show(egui_context.ctx_mut(), |ui| {
                if ui.button(selection_mode_labels(&window.anchor)).clicked() {
                    if selected.len() == 0 {
                        window.anchor = SelectionMode::AnchorSelect;
                    } else {
                        window.anchor = SelectionMode::AnchorSelected(selected[0].3);
                        let translation = selected[0].2.translation();
                        let (lat, lon) = world_to_latlon(translation, offset).unwrap();
                        window.lat = lat as f32;
                        window.lon = lon as f32;
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Latitude: ");
                    ui.add(egui::DragValue::new(&mut window.lat).speed(1e-16));
                });
                ui.horizontal(|ui| {
                    ui.label("Latitude: ");
                    ui.add(egui::DragValue::new(&mut window.lon).speed(1e-16));
                });
                if ui.button("Move").clicked() {
                    let move_cmd = MoveTo {
                        entity: selected[0].3,
                        transform: Transform::from_translation(latlon_to_world(
                            window.lat, window.lon, offset,
                        )),
                    };
                    move_commands.send(move_cmd);
                }
                if ui.button("Close").clicked() {
                    window.visible = false;
                }
            });

            if selected.len() != 0 && matches!(window.anchor, SelectionMode::AnchorSelect) {
                window.anchor = SelectionMode::AnchorSelected(selected[0].3);
                let translation = selected[0].2.translation();
                let (lat, lon) = world_to_latlon(translation, offset).unwrap();
                window.lat = lat as f32;
                window.lon = lon as f32;
            }
        }
    }
}

#[derive(Default)]
struct ReferenceWindow {
    lat: f32,
    lon: f32,
    visible: bool,
}

fn set_reference(
    geo_events: EventReader<GeoReferenceSetReferenceEvent>,
    current_ws: Res<CurrentWorkspace>,
    mut egui_context: ResMut<EguiContext>,
    mut site_properties: Query<(Entity, &mut SiteProperties)>,
    mut window: Local<ReferenceWindow>,
) {
    if geo_events.is_empty() && !window.visible {
        return;
    }

    if !window.visible {
        window.visible = true;
    }

    if let Some((_, mut properties)) = site_properties
        .iter_mut()
        .filter(|(entity, _)| *entity == current_ws.root.unwrap())
        .nth(0)
    {
        if !window.visible {
            window.visible = true;
        }

        egui::Window::new("Set Geographic Reference").show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Latitude: ");
                ui.add(egui::DragValue::new(&mut window.lat).speed(1e-16));
            });
            ui.horizontal(|ui| {
                ui.label("Longitude: ");
                ui.add(egui::DragValue::new(&mut window.lon).speed(1e-16));
            });
            if ui.button("Set reference").clicked() {
                properties.geographic_offset = Some(GeographicOffset {
                    anchor: (window.lat, window.lon),
                    zoom: 15,
                    visible: true,
                });
            }
            if ui.button("Close").clicked() {
                window.visible = false;
            }
        });
    }
}

#[derive(Default)]
pub struct UTMReferenceWindow {
    visible: bool,
}

pub fn view_reference(
    geo_events: EventReader<GeoReferenceViewReferenceEvent>,
    mut egui_context: ResMut<EguiContext>,
    current_ws: Res<CurrentWorkspace>,
    site_properties: Query<(Entity, &SiteProperties)>,
    mut window: Local<UTMReferenceWindow>,
) {
    if geo_events.is_empty() && !window.visible {
        return;
    }

    window.visible = true;

    if let Some((_, properties)) = site_properties
        .iter()
        .filter(|(entity, _)| *entity == current_ws.root.unwrap())
        .nth(0)
    {
        if let Some(offset) = properties.geographic_offset {
            egui::Window::new("View Geographic Reference").show(egui_context.ctx_mut(), |ui| {
                ui.label(format!(
                    "Offset is at {}°, {}°",
                    offset.anchor.0, offset.anchor.1
                ));
                let zone = lat_lon_to_zone_number(offset.anchor.0.into(), offset.anchor.1.into());
                let zone_letter = lat_to_zone_letter(offset.anchor.0.into());
                let utm_offset = to_utm_wgs84(offset.anchor.0.into(), offset.anchor.1.into(), zone);
                ui.label(format!(
                    "Equivalent UTM offset is Zone {}{} with eastings and northings {}, {}",
                    zone,
                    zone_letter.unwrap(),
                    utm_offset.1,
                    utm_offset.0
                ));
            });
        }
    }
}

pub fn add_georeference(
    selected_anchors: Query<(&Anchor, &Selected, &GlobalTransform, Entity)>,
    mut panel_state: Local<GeoReferencePanelState>,
    mut egui_context: ResMut<EguiContext>,
    mut geo_events: EventReader<GeoReferenceSelectAnchorEvent>,
    current_ws: Res<CurrentWorkspace>,
    mut site_properties: Query<(Entity, &mut SiteProperties)>,
) {
    if let Some((_, mut properties)) = site_properties
        .iter_mut()
        .filter(|(entity, _)| *entity == current_ws.root.unwrap())
        .nth(0)
    {
        if let Some(offset) = properties.geographic_offset {
            for _event in geo_events.iter() {
                panel_state.enabled = true;
            }

            let selected: Vec<_> = selected_anchors
                .iter()
                .filter(|(_anchor, selected, _transform, _entity)| selected.is_selected)
                .collect();

            if panel_state.enabled {
                // Draw UI
                egui::Window::new("Geographic Reference").show(egui_context.ctx_mut(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Reference Anchor: ");
                        if ui
                            .button(selection_mode_labels(&panel_state.selection_mode))
                            .clicked()
                        {
                            if selected.len() == 0 {
                                panel_state.selection_mode = SelectionMode::AnchorSelect;
                            } else {
                                panel_state.selection_mode =
                                    SelectionMode::AnchorSelected(selected[0].3);
                                let translation = selected[0].2.translation();
                                let (lat, lon) =
                                    world_to_latlon(translation, offset.anchor).unwrap();
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
                                let zone = lat_lon_to_zone_number(
                                    panel_state.latitude as f64,
                                    panel_state.longitude as f64,
                                );
                                let (northing, easting, _) = to_utm_wgs84(
                                    panel_state.latitude as f64,
                                    panel_state.longitude as f64,
                                    zone,
                                );
                                let utm_origin = (
                                    easting - translation.x as f64,
                                    northing - translation.x as f64,
                                );
                                let (lat, lon) = wsg84_utm_to_lat_lon(
                                    utm_origin.0,
                                    utm_origin.1,
                                    zone,
                                    lat_to_zone_letter(panel_state.latitude.into()).unwrap(),
                                )
                                .unwrap();

                                properties.geographic_offset =
                                    Some(GeographicOffset::from_latlon((lat as f32, lon as f32)));
                            }
                        }
                    });

                    if selected.len() != 0
                        && matches!(panel_state.selection_mode, SelectionMode::AnchorSelect)
                    {
                        panel_state.selection_mode = SelectionMode::AnchorSelected(selected[0].3);
                        let translation = selected[0].2.translation();
                        let (lat, lon) = world_to_latlon(translation, offset.anchor).unwrap();
                        panel_state.latitude = lat as f32;
                        panel_state.longitude = lon as f32;
                    }
                });
            }
        }
    }
}

pub fn set_resolution(
    current_ws: Res<CurrentWorkspace>,
    mut site_properties: Query<(Entity, &mut SiteProperties)>,
    mut egui_context: ResMut<EguiContext>,
)
{
    if let Some((_, mut properties)) = site_properties
        .iter_mut()
        .filter(|(entity, _)| *entity == current_ws.root.unwrap())
        .nth(0)
    {
        if let Some(mut offset) = properties.geographic_offset.as_mut() {
            if !offset.visible  {
                return;
            }
            
            egui::Window::new("Tile Resolution")
                .resizable(false)
                .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(30.0, -30.0))
                .show(egui_context.ctx_mut(), |ui| {
                    ui.add(egui::Slider::new(&mut offset.zoom, MIN_ZOOM..=MAX_ZOOM));
                });
        }
    }
}

fn spawn_tile(
    mut meshes: &mut ResMut<Assets<Mesh>>,
    mut materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    mut commands: &mut Commands,
    coordinates: (f32, f32),
    reference: (f32, f32),
    zoom: i32,
) {
    let tile = OSMTile::from_latlon(zoom, coordinates.0, coordinates.1);
    let tile_size = tile.tile_size();

    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        tile_size.0,
        tile_size.1,
    ))));

    let texture_handle: Handle<Image> = asset_server.load(String::from(
        &AssetSource::OSMSlippyMap(tile.zoom(), coordinates.0, coordinates.1),
    ));
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let tile_offset = latlon_to_world(coordinates.0, coordinates.1, reference);
    commands
        .spawn(PbrBundle {
            mesh: quad_handle,
            material: material_handle,
            transform: Transform::from_xyz(tile_offset.x, tile_offset.y, -0.005),
            ..default()
        })
        .insert(MapTile(tile));
}

pub fn world_to_latlon(
    world_coordinates: Vec3,
    anchor: (f32, f32),
) -> Result<(f64, f64), WSG84ToLatLonError> {
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
        easting = 1000000. + (100000. - easting);
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
    Vec3::new(
        (utm_candidate.1 - utm_offset.1) as f32,
        (utm_candidate.0 - utm_offset.0) as f32,
        0.0,
    )
}


#[derive(Default)]
pub struct RenderSettings {
    prev_anchor: (f32, f32),
}

pub fn render_map_tiles(
    mut map_tiles: Query<(Entity, &MapTile)>,
    mut cameras: Query<(&Camera, &GlobalTransform)>,
    camera_controls: Res<CameraControls>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    current_ws: Res<CurrentWorkspace>,
    mut commands: Commands,
    site_properties: Query<(Entity, &SiteProperties)>,
    mut render_settings: Local<RenderSettings>,
) {
    if let Some((_, site_properties)) = site_properties
        .iter()
        .filter(|(entity, _)| *entity == current_ws.root.unwrap())
        .nth(0)
    {
        if let Some(geo_offset) = site_properties.geographic_offset {
            let offset = geo_offset.anchor;

            // if theres a change in offset rerender all tiles
            if offset != render_settings.prev_anchor {
                render_settings.prev_anchor = offset;
                // Clear all exisitng tiles
                for (entity, _tile) in &map_tiles {
                    commands.entity(entity).despawn();
                }
            }

            if !geo_offset.visible {
                for (entity, _tile) in &map_tiles {
                    commands.entity(entity).despawn();
                }
                return;
            }

            let cam_entity = match camera_controls.mode() {
                ProjectionMode::Perspective => camera_controls.perspective_camera_entities[0],
                ProjectionMode::Orthographic => camera_controls.orthographic_camera_entities[0],
            };

            let mut zoom_changed = false;
            let mut existing_tiles = HashSet::new();
            for (_entity, tile) in &map_tiles {
                if tile.0.zoom() != geo_offset.zoom {
                    zoom_changed = true;
                }
                existing_tiles.insert(tile.0.clone());
            }

            if let Ok((camera, transform)) = cameras.get(cam_entity) {
                if let Some((viewport_min, viewport_max)) = camera.logical_viewport_rect() {
                    let viewport_size = viewport_max - viewport_min;

                    let top_left_ray =
                        Ray3d::from_screenspace(Vec2::new(0.0, 0.0), camera, transform);
                    let top_right_ray =
                        Ray3d::from_screenspace(Vec2::new(viewport_size.x, 0.0), camera, transform);
                    let bottom_left_ray =
                        Ray3d::from_screenspace(Vec2::new(0.0, viewport_size.y), camera, transform);
                    let bottom_right_ray =
                        Ray3d::from_screenspace(viewport_size, camera, transform);

                    let top_left = ray_groundplane_intersection(&top_left_ray);
                    let top_right = ray_groundplane_intersection(&top_right_ray);
                    let bottom_left = ray_groundplane_intersection(&bottom_left_ray);
                    let bottom_right = ray_groundplane_intersection(&bottom_right_ray);

                    let viewport_corners = [top_left, top_right, bottom_left, bottom_right];
                    // Calculate AABB
                    let min_x = viewport_corners
                        .iter()
                        .map(|x| x.x)
                        .fold(f32::INFINITY, |x, val| if x < val { x } else { val });
                    let max_x = viewport_corners
                        .iter()
                        .map(|x| x.x)
                        .fold(-f32::INFINITY, |x, val| if x > val { x } else { val });

                    let min_y = viewport_corners
                        .iter()
                        .map(|x| x.y)
                        .fold(f32::INFINITY, |x, val| if x < val { x } else { val });
                    let max_y = viewport_corners
                        .iter()
                        .map(|x| x.y)
                        .fold(-f32::INFINITY, |x, val| if x > val { x } else { val });

                    // TODO(arjo): Gracefully handle unwrap
                    let latlon_start =
                        world_to_latlon(Vec3::new(min_x, min_y, 0.0), offset).unwrap();
                    let latlon_end = world_to_latlon(Vec3::new(max_x, max_y, 0.0), offset).unwrap();

                    for tile in generate_map_tiles(
                        latlon_start.0 as f32,
                        latlon_start.1 as f32,
                        latlon_end.0 as f32,
                        latlon_end.1 as f32,
                        geo_offset.zoom,
                    ) {
                        if existing_tiles.contains(&tile) && !zoom_changed {
                            continue;
                        }

                        spawn_tile(
                            &mut meshes,
                            &mut materials,
                            &asset_server,
                            &mut commands,
                            tile.get_center(),
                            offset,
                            geo_offset.zoom,
                        );
                    }
                }

                if zoom_changed {
                    for (entity, _tile) in &map_tiles {
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

fn ray_groundplane_intersection(ray: &Option<Ray3d>) -> Vec3 {
    if let Some(ray) = ray {
        let t = -ray.origin().z / ray.direction().z;
        Vec3::new(
            ray.origin().x + t * ray.direction().x,
            ray.origin().y + t * ray.direction().y,
            0.0,
        )
    } else {
        Vec3::new(0.0, 0.0, 0.0)
    }
}

#[test]
fn test_groundplane() {
    let ray = Ray3d::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 1.0));

    // Ground plane should be at (0,0,0)
    assert!(ray_groundplane_intersection(&Some(ray)).length() < 1e-5);
}

pub struct OSMViewPlugin;

impl Plugin for OSMViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GeoReferenceViewReferenceEvent>()
            .add_event::<GeoReferenceSelectAnchorEvent>()
            .add_event::<GeoReferenceSetReferenceEvent>()
            .add_event::<GeoReferenceMoveEvent>()
            .add_stage_after(CoreStage::PreUpdate, "WindowUI", SystemStage::parallel())
            .add_system_to_stage("WindowUI", add_georeference)
            .add_system_to_stage("WindowUI", set_reference)
            .add_system_to_stage("WindowUI", view_reference)
            .add_system_to_stage("WindowUI", move_anchor)
            .add_system_to_stage("WindowUI", set_resolution)
            .add_system(render_map_tiles);
    }
}
