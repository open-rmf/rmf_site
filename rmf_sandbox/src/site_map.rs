use crate::building_map::BuildingMap;
use crate::level::Level;
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub struct SiteMapLabel;

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
    mut handles: ResMut<Handles>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Loading assets");
    commands.init_resource::<Handles>();
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

    let handles: Res<Handles> = Res::from(handles);
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

    println!("Finished spawning site map");
}

fn despawn_site_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_query: Query<(Entity, &Handle<Mesh>)>,
    handles: Res<Handles>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Unloading assets");
    meshes.remove(&handles.vertex_mesh);
    materials.remove(&handles.default_floor_material);
    materials.remove(&handles.lane_material);
    materials.remove(&handles.measurement_material);
    materials.remove(&handles.vertex_material);
    materials.remove(&handles.wall_material);

    println!("Despawing all meshes...");
    for entity_mesh in mesh_query.iter() {
        let (entity, _mesh) = entity_mesh;
        commands.entity(entity).despawn_recursive();
    }
}

fn should_spawn_site_map(sm: Option<Res<SiteMap>>) -> ShouldRun {
    if let Some(sm) = sm {
        if sm.is_added() {
            return ShouldRun::Yes;
        }
    }
    ShouldRun::No
}

fn should_despawn_site_map(sm: Option<Res<SiteMap>>, mut sm_existed: Local<bool>) -> ShouldRun {
    if sm.is_none() && *sm_existed {
        *sm_existed = false;
        return ShouldRun::Yes;
    }
    *sm_existed = sm.is_some();
    return ShouldRun::No;
}

#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Handles>().add_system_set(
            SystemSet::new()
                .label(SiteMapLabel)
                .with_system(spawn_site_map.with_run_criteria(should_spawn_site_map))
                .with_system(despawn_site_map.with_run_criteria(should_despawn_site_map)),
        );
    }
}
