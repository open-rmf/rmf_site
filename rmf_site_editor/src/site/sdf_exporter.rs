use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_gltf_export::{export_mesh, CompressGltfOptions, GltfPose};

use std::collections::BTreeMap;

use crate::site::FloorSegments;
use rmf_site_format::{FloorMarker, LevelElevation, NameInSite, Pose, WallMarker};

pub fn collect_site_meshes(world: &mut World, site: Entity, folder: &str) {
    let mut state: SystemState<(
        Query<&Children>,
        Query<(&NameInSite, &LevelElevation, &Children)>,
        Query<Entity, With<WallMarker>>,
        Query<&FloorSegments, With<FloorMarker>>,
        Query<(&Handle<Mesh>, &Handle<StandardMaterial>)>,
    )> = SystemState::new(world);
    let (q_children, q_levels, q_walls, q_floors, q_pbr) = state.get(world);

    let image_assets = world.resource::<Assets<Image>>();
    let mesh_assets = world.resource::<Assets<Mesh>>();
    let material_assets = world.resource::<Assets<StandardMaterial>>();
    let image_getter = |id: &Handle<Image>| image_assets.get(id).cloned();

    let Ok(site_children) = q_children.get(site) else {
        return;
    };
    for site_child in site_children.iter() {
        let mut mesh_data = Vec::new();
        if let Ok((level_name, elevation, children)) = q_levels.get(*site_child) {
            for child in children.iter() {
                let entity = if let Ok(res) = q_walls.get(*child) {
                    res
                } else if let Ok(res) = q_floors.get(*child) {
                    res.mesh
                } else {
                    continue;
                };
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
                match bevy_gltf_export::export_mesh(
                    mesh.clone(),
                    material.clone(),
                    Some(pose),
                    image_getter,
                ) {
                    Ok(exported) => mesh_data.push(exported),
                    Err(e) => error!("Error exporting mesh to glb"),
                }
            }
            let mut meshes = mesh_data.into_iter();
            let Some(mut mesh) = meshes.next() else {
                continue;
            };
            let Ok(bytes) = mesh
                .combine_with(meshes, CompressGltfOptions::maximum())
                .to_bytes()
            else {
                error!("Error converting glb to bytes");
                continue;
            };
            let filename = format!("{}/level_{}.glb", folder, **level_name);
            let Ok(e) = std::fs::write(filename, bytes) else {
                error!("Error writing mesh to file");
                continue;
            };
        }
    }
}
