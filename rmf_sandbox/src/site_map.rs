use std::collections::{HashMap, HashSet};

use crate::despawn::{DespawnBlocker, PendingDespawn};
use crate::door::Door;
use crate::floor::Floor;
use crate::interaction::{
    Bobbing, DefaultVisualCue, FloorVisualCue, Hovering, InteractionAssets, LaneVisualCue,
    Selected, Spinning, AnchorVisualCue, WallVisualCue,
};
use crate::lane::{Lane, LANE_WIDTH, PASSIVE_LANE_HEIGHT};
use crate::lift::Lift;
use crate::light::Light;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::physical_camera::*;
use crate::settings::*;
use crate::spawner::{SiteMapRoot, VerticesManagers};
use crate::traffic_editor::EditableTag;
use crate::vertex::Vertex;
use crate::{building_map::BuildingMap, wall::Wall};

use bevy::{asset::LoadState, prelude::*};

/// Used to keep track of the entity that represents the current level being rendered by the plugin.
pub struct SiteMapCurrentLevel(pub String);

pub fn init_site_map(sm: Res<BuildingMap>, mut commands: Commands, settings: Res<Settings>) {
    println!("Initializing site map: {}", sm.name);
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.001,
        // brightness: 1.0,
    });

    for level in sm.levels.values() {
        // spawn lights
        let bb = level.calc_bb();
        if settings.graphics_quality == GraphicsQuality::Ultra {
            // spawn a grid of lights for this level
            // todo: make UI controls for light spacing, intensity, range, shadows
            let light_spacing = 5.;
            let num_x_lights = ((bb.max_x - bb.min_x) / light_spacing).ceil() as i32;
            let num_y_lights = ((bb.max_y - bb.min_y) / light_spacing).ceil() as i32;
            for x_idx in 0..num_x_lights {
                for y_idx in 0..num_y_lights {
                    let x = bb.min_x + (x_idx as f64) * light_spacing;
                    let y = bb.min_y + (y_idx as f64) * light_spacing;
                    println!("Inserting light at {x}, {y}");
                    commands
                        .spawn_bundle(PointLightBundle {
                            transform: Transform::from_xyz(x as f32, y as f32, 3.0),
                            point_light: PointLight {
                                intensity: 300.,
                                range: 7.,
                                //shadows_enabled: true,
                                ..default()
                            },
                            ..default()
                        })
                        .insert(SiteMapTag);
                }
            }
        }
    }
    let current_level = sm.levels.keys().next().unwrap();
    commands.insert_resource(Some(SiteMapCurrentLevel(current_level.clone())));
    commands.insert_resource(LoadingModels::default());
    commands.insert_resource(SpawnedModels::default());
}

fn despawn_site_map(
    mut commands: Commands,
    site_map_entities: Query<Entity, With<SiteMapTag>>,
    map_root: Query<Entity, With<SiteMapRoot>>,
    mut level: ResMut<Option<SiteMapCurrentLevel>>,
) {
    // removing this causes bevy to panic, instead just replace it with the default.
    commands.init_resource::<AmbientLight>();

    println!("Despawn all entites");
    for entity in site_map_entities.iter() {
        commands.entity(entity).despawn_recursive();
    }

    *level = None;
    for e in map_root.iter() {
        commands.entity(e).insert(PendingDespawn);
    }
}


#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(SiteMapState::Disabled)
            .init_resource::<Vec<Vertex>>()
            .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
            .init_resource::<SiteAssets>()
            .init_resource::<Option<SiteMapCurrentLevel>>()
            .init_resource::<MaterialMap>()
            .add_system_set(
                SystemSet::on_enter(SiteMapState::Enabled)
                    .label(SiteMapLabel)
                    .with_system(init_site_map),
            )
            .add_system_set(
                SystemSet::on_exit(SiteMapState::Enabled)
                    .label(SiteMapLabel)
                    .with_system(despawn_site_map),
            )
            .add_system_set(
                SystemSet::on_update(SiteMapState::Enabled)
                    .label(SiteMapLabel)
                    .with_system(update_floor)
                    .with_system(update_vertices.after(init_site_map))
                    .with_system(update_lanes.after(update_vertices))
                    .with_system(update_walls.after(update_vertices))
                    .with_system(update_measurements.after(update_vertices))
                    .with_system(update_lights.after(init_site_map))
                    .with_system(update_models.after(init_site_map))
                    .with_system(update_doors.after(update_vertices))
                    .with_system(update_lifts.after(init_site_map))
                    .with_system(update_cameras.after(init_site_map)),
            );
    }
}
