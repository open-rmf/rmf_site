use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_gltf_export::{export_meshes, CompressGltfOptions, GltfPose, MeshData};

use std::collections::BTreeMap;
use std::path::Path;

use crate::site::{CollisionMeshMarker, FloorSegments, VisualMeshMarker};
use rmf_site_format::{FloorMarker, LevelElevation, ModelMarker, NameInSite, Pose, WallMarker};

pub fn collect_site_meshes(world: &mut World, site: Entity, folder: &Path) {
    let mut state: SystemState<(
        Query<&Children>,
        Query<(&NameInSite, &LevelElevation, &Children)>,
        Query<Entity, With<WallMarker>>,
        Query<&FloorSegments, With<FloorMarker>>,
        Query<Entity, With<ModelMarker>>,
        Query<(Entity, &GlobalTransform), With<CollisionMeshMarker>>,
        Query<(Entity, &GlobalTransform), With<VisualMeshMarker>>,
        Query<(&Handle<Mesh>, &Handle<StandardMaterial>)>,
        Query<&GlobalTransform>,
    )> = SystemState::new(world);
    let (q_children, q_levels, q_walls, q_floors, q_models, q_collisions, q_visuals, q_pbr, q_tfs) =
        state.get(world);

    let image_assets = world.resource::<Assets<Image>>();
    let mesh_assets = world.resource::<Assets<Mesh>>();
    let material_assets = world.resource::<Assets<StandardMaterial>>();
    let image_getter = |id: &Handle<Image>| image_assets.get(id).cloned();

    let get_mesh_and_material = |entity: Entity| -> Option<(&Mesh, &StandardMaterial)> {
        let Ok((mesh, material)) = q_pbr.get(entity) else {
            return None;
        };
        let Some(mesh) = mesh_assets.get(mesh) else {
            warn!("Mesh asset not found");
            return None;
        };
        let Some(material) = material_assets.get(material) else {
            warn!("Material asset not found");
            return None;
        };
        Some((mesh, material))
    };

    let Ok(site_children) = q_children.get(site) else {
        return;
    };
    for site_child in site_children.iter() {
        let mut collision_data = Vec::new();
        let mut visual_data = Vec::new();
        if let Ok((level_name, elevation, children)) = q_levels.get(*site_child) {
            let level_pose = GltfPose {
                translation: [0.0, 0.0, **elevation],
                ..Default::default()
            };
            for child in children.iter() {
                if let Ok(res) = q_walls.get(*child) {
                    let Some((mesh, material)) = get_mesh_and_material(res) else {
                        continue;
                    };
                    collision_data.push(MeshData {
                        mesh,
                        material,
                        pose: Some(level_pose.clone()),
                    });
                    visual_data.push(MeshData {
                        mesh,
                        material,
                        pose: Some(level_pose.clone()),
                    });
                } else if let Ok(res) = q_floors.get(*child) {
                    let Some((mesh, material)) = get_mesh_and_material(res.mesh) else {
                        continue;
                    };
                    collision_data.push(MeshData {
                        mesh,
                        material,
                        pose: Some(level_pose.clone()),
                    });
                    visual_data.push(MeshData {
                        mesh,
                        material,
                        pose: Some(level_pose.clone()),
                    });
                } else if let Ok(model) = q_models.get(*child) {
                    // TODO(luca) don't do full descendant iter here or we might add twice?
                    // Iterate through children and select all meshes
                    for model_child in DescendantIter::new(&q_children, model) {
                        if let Ok((entity, tf)) = q_collisions.get(model_child) {
                            // Now iterate through the children of the collision and add them
                            for entity in DescendantIter::new(&q_children, model_child) {
                                let Some((mesh, material)) = get_mesh_and_material(entity) else {
                                    continue;
                                };
                                let Ok(tf) = q_tfs.get(entity) else {
                                    continue;
                                };
                                let tf = tf.compute_transform();
                                let pose = GltfPose {
                                    translation: [
                                        tf.translation.x,
                                        tf.translation.y,
                                        tf.translation.z + **elevation,
                                    ],
                                    rotation: tf.rotation.to_array(),
                                    scale: Some(tf.scale.to_array()),
                                };
                                collision_data.push(MeshData {
                                    mesh,
                                    material,
                                    pose: Some(pose),
                                });
                            }
                        } else if let Ok((entity, tf)) = q_visuals.get(model_child) {
                            // Now iterate through the children of the visuals and add them
                            for entity in DescendantIter::new(&q_children, model_child) {
                                let Some((mesh, material)) = get_mesh_and_material(entity) else {
                                    continue;
                                };
                                let Ok(tf) = q_tfs.get(entity) else {
                                    continue;
                                };
                                let tf = tf.compute_transform();
                                let pose = GltfPose {
                                    translation: [
                                        tf.translation.x,
                                        tf.translation.y,
                                        tf.translation.z + **elevation,
                                    ],
                                    rotation: tf.rotation.to_array(),
                                    scale: Some(tf.scale.to_array()),
                                };
                                visual_data.push(MeshData {
                                    mesh,
                                    material,
                                    pose: Some(pose),
                                });
                            }
                        }
                    }
                } else {
                    continue;
                };
            }
            let Ok(collisions) = export_meshes(
                collision_data,
                None,
                image_getter,
                CompressGltfOptions::skip_materials(),
            ) else {
                error!("Failed exporting collision data");
                continue;
            };
            let Ok(bytes) = collisions.to_bytes() else {
                error!("Error converting glb to bytes");
                continue;
            };
            let filename = format!("{}/level_{}_collision.glb", folder.display(), **level_name);
            let Ok(e) = std::fs::write(filename, bytes) else {
                error!("Error writing mesh to file");
                continue;
            };
            let Ok(visuals) = export_meshes(
                visual_data,
                Some(format!("level_{}_visuals", **level_name)),
                image_getter,
                CompressGltfOptions::default(),
            ) else {
                error!("Failed exporting visual data");
                continue;
            };
            let Ok(bytes) = visuals.to_bytes() else {
                error!("Error converting glb to bytes");
                continue;
            };
            let filename = format!("{}/level_{}_visual.glb", folder.display(), **level_name);
            let Ok(e) = std::fs::write(filename, bytes) else {
                error!("Error writing mesh to file");
                continue;
            };
        }
    }
}
