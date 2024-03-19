use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_gltf_export::{export_meshes, CompressGltfOptions, GltfPose, MeshData, MeshExportError};

use std::collections::BTreeMap;
use std::path::Path;

use crate::site::{
    ChildLiftCabinGroup, CollisionMeshMarker, DoorSegments, FloorSegments, LiftDoormat,
    VisualMeshMarker,
};
use rmf_site_format::{
    IsStatic, LevelElevation, LiftCabin, ModelMarker, NameInSite, Pose, SiteID, WallMarker,
};

pub fn collect_site_meshes(world: &mut World, site: Entity, folder: &Path) {
    let mut state: SystemState<(
        Query<&Children>,
        Query<(&NameInSite, &LevelElevation, &Children)>,
        Query<Entity, With<WallMarker>>,
        Query<&FloorSegments>,
        Query<(Option<&NameInSite>, &DoorSegments)>,
        Query<(Entity, &IsStatic, &NameInSite), With<ModelMarker>>,
        Query<(Entity, &GlobalTransform), With<CollisionMeshMarker>>,
        Query<(Entity, &GlobalTransform), With<VisualMeshMarker>>,
        Query<(&Handle<Mesh>, &Handle<StandardMaterial>)>,
        Query<(&NameInSite, &LiftCabin<Entity>, &ChildLiftCabinGroup)>,
        Query<((), With<LiftDoormat>)>,
        Query<&GlobalTransform>,
        Query<&Transform>,
    )> = SystemState::new(world);
    let (
        q_children,
        q_levels,
        q_walls,
        q_floors,
        q_doors,
        q_models,
        q_collisions,
        q_visuals,
        q_pbr,
        q_lift_cabins,
        q_lift_door_mats,
        q_global_tfs,
        q_tfs,
    ) = state.get(world);

    let image_assets = world.resource::<Assets<Image>>();
    let mesh_assets = world.resource::<Assets<Mesh>>();
    let material_assets = world.resource::<Assets<StandardMaterial>>();
    let write_meshes_to_file = |meshes: Vec<MeshData>,
                                name: Option<String>,
                                options: CompressGltfOptions,
                                filename: String|
     -> Result<(), MeshExportError> {
        let image_getter = |id: &Handle<Image>| image_assets.get(id).cloned();
        let meshes = export_meshes(meshes, name, image_getter, options)?;
        let bytes = meshes.to_bytes()?;
        let Ok(_) = std::fs::write(filename, bytes) else {
            // TODO(luca) make this an error
            error!("Error writing mesh to file");
            return Ok(());
        };
        Ok(())
    };

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
                        material: None,
                        pose: Some(level_pose.clone()),
                    });
                    visual_data.push(MeshData {
                        mesh,
                        material: Some(material),
                        pose: Some(level_pose.clone()),
                    });
                } else if let Ok(res) = q_floors.get(*child) {
                    let Some((mesh, material)) = get_mesh_and_material(res.mesh) else {
                        continue;
                    };
                    collision_data.push(MeshData {
                        mesh,
                        material: None,
                        pose: Some(level_pose.clone()),
                    });
                    visual_data.push(MeshData {
                        mesh,
                        material: Some(material),
                        pose: Some(level_pose.clone()),
                    });
                } else if let Ok((model, is_static, name)) = q_models.get(*child) {
                    let mut model_collisions = vec![];
                    let mut model_visuals = vec![];
                    // TODO(luca) don't do full descendant iter here or we might add twice?
                    // Iterate through children and select all meshes
                    for model_child in DescendantIter::new(&q_children, model) {
                        if let Ok((entity, tf)) = q_collisions.get(model_child) {
                            // Now iterate through the children of the collision and add them
                            for entity in DescendantIter::new(&q_children, model_child) {
                                let Some((mesh, material)) = get_mesh_and_material(entity) else {
                                    continue;
                                };
                                let Ok(tf) = q_global_tfs.get(entity) else {
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
                                model_collisions.push(MeshData {
                                    mesh,
                                    material: None,
                                    pose: Some(pose),
                                });
                            }
                        } else if let Ok((entity, tf)) = q_visuals.get(model_child) {
                            // Now iterate through the children of the visuals and add them
                            for entity in DescendantIter::new(&q_children, model_child) {
                                let Some((mesh, material)) = get_mesh_and_material(entity) else {
                                    continue;
                                };
                                let Ok(tf) = q_global_tfs.get(entity) else {
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
                                model_visuals.push(MeshData {
                                    mesh,
                                    material: Some(material),
                                    pose: Some(pose),
                                });
                            }
                        }
                    }
                    if **is_static {
                        // This is part of the static world, add it to the static mesh
                        collision_data.extend(model_collisions);
                        visual_data.extend(model_visuals);
                    } else {
                        // Create a new mesh for it
                        let filename = format!("{}/{}_collision.glb", folder.display(), **name);
                        write_meshes_to_file(
                            model_collisions,
                            None,
                            CompressGltfOptions::skip_materials(),
                            filename,
                        );
                        let filename = format!("{}/{}_visual.glb", folder.display(), **name);
                        write_meshes_to_file(
                            model_visuals,
                            Some(format!("{}_visual", **name)),
                            CompressGltfOptions::default(),
                            filename,
                        );
                    }
                } else if let Ok((door_name, segments)) = q_doors.get(*child) {
                    for (entity, segment_name) in segments
                        .body
                        .entities()
                        .iter()
                        .zip(segments.body.labels().into_iter())
                    {
                        // Generate the visual and collisions here
                        let Some((mesh, material)) = get_mesh_and_material(*entity) else {
                            continue;
                        };
                        let Ok(tf) = q_tfs.get(*entity) else {
                            continue;
                        };
                        let Some(door_name) = door_name else {
                            continue;
                        };
                        let pose = GltfPose {
                            translation: tf.translation.to_array(),
                            rotation: tf.rotation.to_array(),
                            scale: Some(tf.scale.to_array()),
                        };

                        let data = MeshData {
                            mesh,
                            material: Some(material),
                            pose: Some(pose.clone()),
                        };
                        let filename =
                            format!("{}/{}_{}.glb", folder.display(), **door_name, segment_name);
                        write_meshes_to_file(
                            vec![data],
                            None,
                            CompressGltfOptions::default(),
                            filename,
                        );
                    }
                } else {
                    continue;
                };
            }
            let filename = format!("{}/level_{}_collision.glb", folder.display(), **level_name);
            write_meshes_to_file(
                collision_data,
                None,
                CompressGltfOptions::skip_materials(),
                filename,
            );
            let filename = format!("{}/level_{}_visual.glb", folder.display(), **level_name);
            write_meshes_to_file(
                visual_data,
                Some(format!("level_{}_visuals", **level_name)),
                CompressGltfOptions::default(),
                filename,
            );
        }
        // Lifts
        if let Ok((lift_name, cabin, cabin_children)) = q_lift_cabins.get(*site_child) {
            // The children of this entity have the mesh for the lift cabin
            let mut lift_data = vec![];
            for entity in DescendantIter::new(&q_children, **cabin_children) {
                if q_lift_door_mats.get(entity).is_ok() {
                    // Just visual cues, not exported
                    continue;
                }
                let Some((mesh, material)) = get_mesh_and_material(entity) else {
                    info!("Cabin Child without mesh!");
                    continue;
                };
                info!("Found mesh for cabin child");
                lift_data.push(MeshData {
                    mesh,
                    material: Some(material),
                    pose: None,
                });
            }
            let filename = format!("{}/{}.glb", folder.display(), **lift_name);
            write_meshes_to_file(lift_data, None, CompressGltfOptions::default(), filename);
            // Now generate the lift doors
            let LiftCabin::Rect(cabin) = cabin;
            for (face, door) in cabin.doors().iter() {
                let Some(door) = door else {
                    continue;
                };
                println!("Found door with label {}", face.label());
                if let Ok((_, segments)) = q_doors.get(door.door) {
                    println!("Segments found");
                    // TODO(luca) this is duplicated with door generation, refactor
                    for (entity, segment_name) in segments
                        .body
                        .entities()
                        .iter()
                        .zip(segments.body.labels().into_iter())
                    {
                        // Generate the visual and collisions here
                        let Some((mesh, material)) = get_mesh_and_material(*entity) else {
                            continue;
                        };
                        let Ok(tf) = q_tfs.get(*entity) else {
                            continue;
                        };
                        let pose = GltfPose {
                            translation: tf.translation.to_array(),
                            rotation: tf.rotation.to_array(),
                            scale: Some(tf.scale.to_array()),
                        };

                        let data = MeshData {
                            mesh,
                            material: Some(material),
                            pose: Some(pose.clone()),
                        };
                        let filename = format!(
                            "{}/{}_{}_{}.glb",
                            folder.display(),
                            **lift_name,
                            face.label(),
                            segment_name
                        );
                        write_meshes_to_file(
                            vec![data],
                            None,
                            CompressGltfOptions::default(),
                            filename,
                        );
                    }
                }
            }
        }
    }
}
