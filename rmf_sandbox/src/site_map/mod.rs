pub mod ui;
pub use ui::*;

use crate::building_map::BuildingMap;
use crate::level::Level;
use crate::AppState;
use bevy::prelude::*;

#[derive(Default)]
pub struct Handles {
    pub default_floor_material: Handle<StandardMaterial>,
    pub lane_material: Handle<StandardMaterial>,
    pub measurement_material: Handle<StandardMaterial>,
    pub vertex_mesh: Handle<Mesh>,
    pub vertex_material: Handle<StandardMaterial>,
    pub wall_material: Handle<StandardMaterial>,
}

#[derive(Default)]
pub struct SiteMap {
    site_name: String,
    levels: Vec<Level>,
}

impl SiteMap {
    pub fn from_building_map(building_map: BuildingMap) -> SiteMap {
        let sm = SiteMap {
            site_name: building_map.name,
            levels: building_map.levels.into_values().collect(),
            ..Default::default()
        };

        // todo: global alignment via fiducials

        return sm;
    }
}

fn spawn_site_map(
    sm: Res<SiteMap>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<Handles>,
    asset_server: Res<AssetServer>,
) {
    println!("Spawning site map: {}", sm.site_name);

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.001,
    });

    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            illuminance: 20000.,
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0., 0., 50.),
            rotation: Quat::from_rotation_x(0.4),
            ..Default::default()
        },
        ..Default::default()
    });

    for level in &sm.levels {
        level.spawn(&mut commands, &mut meshes, &handles, &asset_server);
    }

    // todo: use real floor polygons
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
        material: handles.default_floor_material.clone(),
        transform: Transform {
            rotation: Quat::from_rotation_x(1.57),
            ..Default::default()
        },
        ..Default::default()
    });
}

fn despawn_site_map(
    mut commands: Commands,
    mesh_query: Query<(Entity, &Handle<Mesh>)>,
) {
    println!("Despawing all meshes...");
    for entity_mesh in mesh_query.iter() {
        let (entity, _mesh) = entity_mesh;
        commands.entity(entity).despawn_recursive();
    }
}

fn init_handles(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut handles: ResMut<Handles>,
) {
    handles.vertex_mesh = meshes.add(Mesh::from(shape::Capsule {
        radius: 0.25,
        rings: 2,
        depth: 0.05,
        latitudes: 8,
        longitudes: 16,
        uv_profile: shape::CapsuleUvProfile::Fixed,
    }));

    handles.default_floor_material = materials.add(Color::rgb(0.3, 0.3, 0.3).into());
    handles.lane_material = materials.add(Color::rgb(1.0, 0.5, 0.3).into());
    handles.measurement_material = materials.add(Color::rgb(1.0, 0.5, 1.0).into());
    handles.vertex_material = materials.add(Color::rgb(0.4, 0.7, 0.6).into());
    handles.wall_material = materials.add(Color::rgb(0.5, 0.5, 1.0).into());
}

#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ui::SiteMapUIPlugin);

        app.init_resource::<Handles>()
            .add_startup_system(init_handles);

        app.add_system_set(SystemSet::on_enter(AppState::SiteMap).with_system(spawn_site_map));

        app.add_system_set(SystemSet::on_exit(AppState::SiteMap).with_system(despawn_site_map));
    }
}
