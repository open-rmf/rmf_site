use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_gltf_export::{export_mesh, gltf_to_bytes, GltfPose};

use std::collections::BTreeMap;

use crate::site::FloorSegments;
use rmf_site_format::{FloorMarker, LevelElevation, Pose, WallMarker};

// Returns a map from level site id to
//fn collect_wall_meshes(world: &World, site: Entity) -> HashMap<u32,

pub fn collect_site_meshes(world: &mut World, site: Entity, folder: &str) {
    let mut state: SystemState<(
        Query<&Children>,
        Query<(Entity, &LevelElevation)>,
        Query<Entity, With<WallMarker>>,
        Query<&FloorSegments, With<FloorMarker>>,
        Query<(&Handle<Mesh>, &Handle<StandardMaterial>)>,
    )> = SystemState::new(world);
    // TODO(luca) level elevation into meshes transform
    let (q_children, q_levels, q_walls, q_floors, q_pbr) = state.get(world);

    let image_assets = world.resource::<Assets<Image>>();
    let mesh_assets = world.resource::<Assets<Mesh>>();
    let material_assets = world.resource::<Assets<StandardMaterial>>();
    let image_getter = |id: &Handle<Image>| image_assets.get(id).cloned();

    info!("Looking for children");
    for site_child in q_children.get(site).unwrap().iter() {
        if let Ok((level, elevation)) = q_levels.get(*site_child) {
            info!("Found level {:?}", level);
            for (i, child) in q_children.get(level).unwrap().iter().enumerate() {
                let entity = if let Ok(res) = q_walls.get(*child) {
                    res
                } else if let Ok(res) = q_floors.get(*child) {
                    res.mesh
                } else {
                    continue;
                };
                info!("Found a mesh");
                // Get mesh and material
                let Ok((mesh, material)) = q_pbr.get(entity) else {
                    // This shouldn't happen
                    warn!("Wall or floor {:?} without a mesh", entity);
                    continue;
                };
                let Some(mesh) = mesh_assets.get(mesh) else {
                    warn!("Mesh asset not found");
                    continue;
                };
                let Some(material) = material_assets.get(material) else {
                    warn!("Material asset not found");
                    continue;
                };

                let pose = GltfPose {
                    translation: [0.0, 0.0, **elevation],
                    ..Default::default()
                };
                let (root, bytes) = bevy_gltf_export::export_mesh(
                    mesh.clone(),
                    material.clone(),
                    Some(pose),
                    image_getter,
                )
                .unwrap();
                // TODO(luca) merge the roots
                let bytes = gltf_to_bytes(&root, bytes).unwrap();
                let filename = format!("{}/level_{}_mesh_{}.glb", folder, level.index(), i);
                info!("Writing to {}", filename);
                std::fs::write(filename, bytes).unwrap();
                info!("Wrote mesh");
            }
        }
    }
}
