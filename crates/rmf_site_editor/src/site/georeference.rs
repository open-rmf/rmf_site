use bevy::{asset::AssetPath, math::Ray3d, prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContexts, egui};
use rmf_site_camera::{ActiveCameraQuery, active_camera_maybe};
use rmf_site_egui::*;
use rmf_site_format::{GeographicComponent, GeographicOffset};
use std::collections::HashSet;
use utm::*;

use crate::{OSMTile, generate_map_tiles, workspace::CurrentWorkspace};

const MAX_ZOOM: i32 = 19;
const MIN_ZOOM: i32 = 12;
const MAX_TILES: usize = 50;

#[derive(Component, Clone, Eq, PartialEq, Hash)]
pub struct MapTile(OSMTile);

#[derive(Default)]
struct ReferenceWindow {
    lat: f32,
    lon: f32,
    visible: bool,
}

fn set_reference(
    mut geo_events: EventReader<MenuEvent>,
    osm_menu: Res<OSMMenu>,
    current_ws: Res<CurrentWorkspace>,
    mut egui_context: EguiContexts,
    mut site_properties: Query<(Entity, &mut GeographicComponent)>,
    mut window: Local<ReferenceWindow>,
) {
    for event in geo_events.read() {
        if event.clicked() && event.source() == osm_menu.set_reference {
            window.visible = true;
        }
    }
    if !window.visible {
        return;
    }

    if !window.visible {
        window.visible = true;
    }

    if let Some((_, mut properties)) = site_properties
        .iter_mut()
        .filter(|(entity, _)| Some(*entity) == current_ws.root)
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

                if window.lat <= 90.0 && window.lat >= -90.0 &&
                    window.lon <= 180.0 && window.lon >= -180.0 {
                    properties.0 = Some(GeographicOffset {
                        anchor: (window.lat, window.lon),
                        zoom: 15,
                        visible: true,
                    });
                }
                else {
                    error!("Longitude must be in [-90, 90] range and Latitude must be in [-180, 180] range.");
                }
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

/// Synchronizes menu when opening new files and changing items in
/// with GeographicComponents.
pub fn detect_new_geographic_component(
    geographic_comp: Query<
        &GeographicComponent,
        Or<(Added<GeographicComponent>, Changed<GeographicComponent>)>,
    >,
    mut checkbox_state: Query<&mut MenuItem>,
    current_ws: Res<CurrentWorkspace>,
    osm_menu: Res<OSMMenu>,
    mut command: Commands,
) {
    let Some(ws_entity) = current_ws.root else {
        return;
    };

    let Ok(comp) = geographic_comp.get(ws_entity) else {
        return;
    };

    if let Some(comp) = comp.0 {
        command
            .entity(osm_menu.view_reference)
            .remove::<MenuDisabled>();
        command
            .entity(osm_menu.satellite_map_check_button)
            .remove::<MenuDisabled>();
        command
            .entity(osm_menu.settings_panel)
            .remove::<MenuDisabled>();

        let Ok(mut checkbox) = checkbox_state.get_mut(osm_menu.satellite_map_check_button) else {
            return;
        };

        let MenuItem::CheckBox(_, value) = checkbox.as_mut() else {
            return;
        };

        *value = comp.visible;
    } else {
        command.entity(osm_menu.view_reference).insert(MenuDisabled);
        command
            .entity(osm_menu.satellite_map_check_button)
            .insert(MenuDisabled);
        command.entity(osm_menu.settings_panel).insert(MenuDisabled);

        let Ok(mut checkbox) = checkbox_state.get_mut(osm_menu.satellite_map_check_button) else {
            return;
        };

        let MenuItem::CheckBox(_, value) = checkbox.as_mut() else {
            return;
        };

        *value = false;
    }
}

/// Keeps visibility in check
pub fn handle_visibility_change(
    mut geo_events: EventReader<MenuEvent>,
    osm_menu: Res<OSMMenu>,
    current_ws: Res<CurrentWorkspace>,
    mut geographic_comp: Query<&mut GeographicComponent>,
    checkbox_state: Query<&MenuItem>,
) {
    let Some(current_ws) = current_ws.root else {
        return;
    };

    let Ok(mut comp) = geographic_comp.get_mut(current_ws) else {
        return;
    };

    let Some(comp) = &mut comp.0 else {
        return;
    };

    for event in geo_events.read() {
        if event.clicked() && event.source() == osm_menu.satellite_map_check_button {
            let Ok(item) = checkbox_state.get(osm_menu.satellite_map_check_button) else {
                continue;
            };
            let MenuItem::CheckBox(_, _) = item else {
                continue;
            };
            comp.visible = !comp.visible;
        }
    }
}

pub fn view_reference(
    mut geo_events: EventReader<MenuEvent>,
    osm_menu: Res<OSMMenu>,
    mut egui_context: EguiContexts,
    current_ws: Res<CurrentWorkspace>,
    site_properties: Query<(Entity, &GeographicComponent)>,
    mut window: Local<UTMReferenceWindow>,
) {
    for event in geo_events.read() {
        if event.clicked() && event.source() == osm_menu.view_reference {
            window.visible = true;
        }
    }

    if !window.visible {
        return;
    }

    if let Some((_, properties)) = site_properties
        .iter()
        .filter(|(entity, _)| Some(*entity) == current_ws.root)
        .nth(0)
    {
        if let Some(offset) = properties.0 {
            egui::Window::new("View Geographic Reference").show(egui_context.ctx_mut(), |ui| {
                ui.label(format!(
                    "Offset is at {}°, {}°",
                    offset.anchor.0, offset.anchor.1
                ));
                let zone = lat_lon_to_zone_number(offset.anchor.0.into(), offset.anchor.1.into());
                let zone_letter =
                    if let Some(zone_letter) = lat_to_zone_letter(offset.anchor.0.into()) {
                        zone_letter.to_string()
                    } else {
                        String::with_capacity(0)
                    };
                let utm_offset = to_utm_wgs84(offset.anchor.0.into(), offset.anchor.1.into(), zone);
                ui.label(format!(
                    "Equivalent UTM offset is Zone {}{} with eastings and northings {}, {}",
                    zone, zone_letter, utm_offset.1, utm_offset.0
                ));
                if ui.button("Close").clicked() {
                    window.visible = false;
                }
            });
        }
    }
}

#[derive(Default)]
struct SettingsWindow {
    visible: bool,
}

fn settings(
    mut geo_events: EventReader<MenuEvent>,
    current_ws: Res<CurrentWorkspace>,
    osm_menu: Res<OSMMenu>,
    mut site_properties: Query<&mut GeographicComponent>,
    mut egui_context: EguiContexts,
    mut settings_window: Local<SettingsWindow>,
) {
    for event in geo_events.read() {
        if event.clicked() && event.source() == osm_menu.settings_panel {
            settings_window.visible = true;
        }
    }

    if !settings_window.visible {
        return;
    }

    let Some(current_ws) = current_ws.root else {
        return;
    };

    if let Ok(mut properties) = site_properties.get_mut(current_ws) {
        if let Some(offset) = properties.0.as_mut() {
            if !offset.visible {
                return;
            }

            egui::Window::new("Settings").show(egui_context.ctx_mut(), |ui| {
                ui.label("Tile Resolution");
                ui.add(egui::Slider::new(&mut offset.zoom, MIN_ZOOM..=MAX_ZOOM));

                if ui.button("Ok").clicked() {
                    settings_window.visible = false;
                }
            });
        }
    }
}

fn spawn_tile(
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    commands: &mut Commands,
    coordinates: (f32, f32),
    reference: (f32, f32),
    zoom: i32,
) {
    let tile = OSMTile::from_latlon(zoom, coordinates.0, coordinates.1);

    let Some(mesh) = tile.get_quad_mesh() else {
        error!("Could not retrieve meshshape");
        return;
    };
    let quad_handle = meshes.add(mesh);

    let texture_handle: Handle<Image> = asset_server.load_override(AssetPath::from(&tile));
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let tile_offset = latlon_to_world(coordinates.0, coordinates.1, reference);
    commands
        .spawn((
            Mesh3d(quad_handle),
            MeshMaterial3d(material_handle),
            Transform::from_xyz(tile_offset.x, tile_offset.y, -0.005),
            Visibility::default(),
        ))
        .insert(MapTile(tile));
}

pub fn world_to_latlon(
    world_coordinates: Vec3,
    anchor: (f32, f32),
) -> Result<(f64, f64), WSG84ToLatLonError> {
    let mut zone = lat_lon_to_zone_number(anchor.0.into(), anchor.1.into());
    let Some(mut zone_letter) = lat_to_zone_letter(anchor.0.into()) else {
        return Err(WSG84ToLatLonError::ZoneLetterOutOfRange);
    };
    let utm_offset = to_utm_wgs84(anchor.0.into(), anchor.1.into(), zone);
    let mut easting = world_coordinates.x as f64 + utm_offset.1;
    let mut northing = world_coordinates.y as f64 + utm_offset.0;

    // A really wrong way of measuring stuff. TODO: Handle case where easting
    // and northing are out of bounds. TBH I have no idea how to correctly
    // handle such cases. Ideally we should use proj, but proj is not supported on
    // WASM.
    while northing < 0. {
        northing = 10000000. + northing;
        zone_letter = (zone_letter as u8 - 1) as char;
    }
    while northing > 10000000. {
        northing = northing - 10000000.;
        zone_letter = (zone_letter as u8 + 1) as char;
    }

    while easting < 100000. {
        easting = 1000000. + (100000. - easting);
        zone += 1;
    }

    while easting > 1000000. {
        easting -= 1000000.;
        zone -= 1;
    }
    return wsg84_utm_to_lat_lon(easting, northing, zone, zone_letter);
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
    map_tiles: Query<(Entity, &MapTile)>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    current_ws: Res<CurrentWorkspace>,
    active_cam: ActiveCameraQuery,
    mut commands: Commands,
    site_properties: Query<(Entity, &GeographicComponent)>,
    mut render_settings: Local<RenderSettings>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    if let Some((_, site_properties)) = site_properties
        .iter()
        .filter(|(entity, _)| Some(*entity) == current_ws.root)
        .nth(0)
    {
        if let Some(geo_offset) = site_properties.0 {
            let offset = geo_offset.anchor;

            // if theres a change in offset rerender all tiles
            if offset != render_settings.prev_anchor {
                render_settings.prev_anchor = offset;
                // Clear all exisitng tiles
                for (entity, _tile) in &map_tiles {
                    commands.entity(entity).remove::<Children>().despawn();
                }
            }

            if !geo_offset.visible {
                for (entity, _tile) in &map_tiles {
                    commands.entity(entity).remove::<Children>().despawn();
                }
                return;
            }

            let Ok(cam_entity) = active_camera_maybe(&active_cam) else {
                return;
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
                if let Some(Rect { min, max }) = camera.logical_viewport_rect() {
                    let viewport_size = max - min;

                    let Ok(primary_window) = primary_window.single() else {
                        return;
                    };
                    let top_left_ray = ray_from_screenspace(
                        Vec2::new(0.0, 0.0),
                        camera,
                        transform,
                        primary_window,
                    );
                    let top_right_ray = ray_from_screenspace(
                        Vec2::new(viewport_size.x, 0.0),
                        camera,
                        transform,
                        primary_window,
                    );
                    let bottom_left_ray = ray_from_screenspace(
                        Vec2::new(0.0, viewport_size.y),
                        camera,
                        transform,
                        primary_window,
                    );
                    let bottom_right_ray =
                        ray_from_screenspace(viewport_size, camera, transform, primary_window);

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

                    let Ok(latlon_start) = world_to_latlon(Vec3::new(min_x, min_y, 0.0), offset)
                    else {
                        return;
                    };
                    let Ok(latlon_end) = world_to_latlon(Vec3::new(max_x, max_y, 0.0), offset)
                    else {
                        return;
                    };

                    let mut num_tiles = existing_tiles.len();
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

                        // Limit number of tiles fetched.
                        if num_tiles > MAX_TILES {
                            break;
                        }
                        num_tiles += 1;

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
                        commands.entity(entity).remove::<Children>().despawn();
                    }
                }
            }
        }
    }
}

fn ray_from_screenspace(
    cursor_position: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<Ray3d> {
    camera
        .viewport
        .as_ref()
        .map(|viewport| {
            cursor_position - &viewport.physical_position.as_vec2() / window.scale_factor()
        })
        .and_then(|viewport_pos| {
            camera
                .viewport_to_world(&camera_transform, viewport_pos)
                .ok()
                .map(Ray3d::from)
        })
}

fn ray_groundplane_intersection(ray: &Option<Ray3d>) -> Vec3 {
    if let Some(ray) = ray {
        let t = -ray.origin.z / ray.direction.z;
        Vec3::new(
            ray.origin.x + t * ray.direction.x,
            ray.origin.y + t * ray.direction.y,
            0.0,
        )
    } else {
        Vec3::new(0.0, 0.0, 0.0)
    }
}

#[test]
fn test_groundplane() {
    let ray = Ray3d::new(
        Vec3::new(1.0, 1.0, 1.0),
        Dir3::from_xyz(1.0, 1.0, 1.0).unwrap(),
    );

    // Ground plane should be at (0,0,0)
    assert!(ray_groundplane_intersection(&Some(ray)).length() < 1e-5);
}

#[derive(Debug, Resource)]
pub struct OSMMenu {
    set_reference: Entity,
    view_reference: Entity,
    settings_panel: Entity,
    satellite_map_check_button: Entity,
}

impl FromWorld for OSMMenu {
    fn from_world(world: &mut World) -> Self {
        // Tools menu
        let set_reference = world.spawn(MenuItem::Text("Set Reference".into())).id();
        let view_reference = world.spawn(MenuItem::Text("View Reference".into())).id();
        let settings_reference = world.spawn(MenuItem::Text("Settings".into())).id();

        let sub_menu = world
            .spawn(Menu::from_title("Geographic Offset".to_string()))
            .id();
        world.entity_mut(sub_menu).add_children(&[
            set_reference,
            view_reference,
            settings_reference,
        ]);

        let tool_header = world.resource::<ToolMenu>().get();
        world.entity_mut(tool_header).add_children(&[sub_menu]);

        // Checkbox
        let view_header = world.resource::<ViewMenu>().get();
        let satellite_map_check_button = world
            .spawn(MenuItem::CheckBox("Satellite Map".to_string(), false))
            .id();
        world
            .entity_mut(view_header)
            .add_children(&[satellite_map_check_button]);

        OSMMenu {
            set_reference,
            view_reference,
            settings_panel: settings_reference,
            satellite_map_check_button,
        }
    }
}

pub struct OSMViewPlugin;

impl Plugin for OSMViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OSMMenu>()
            // TODO(luca) these were in PreUpdate before but putting them there seems to break
            // editing text fields
            .add_systems(Update, (set_reference, view_reference, settings))
            .add_systems(
                Update,
                (
                    render_map_tiles,
                    handle_visibility_change,
                    detect_new_geographic_component,
                ),
            );
    }
}
