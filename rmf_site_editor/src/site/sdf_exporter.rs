use bevy::ecs::{
    relationship::DescendantIter,
    system::{SystemId, SystemState},
};
use bevy::prelude::*;
use bevy_gltf_export::{export_meshes, CompressGltfOptions, MeshData};

use std::{collections::HashMap, path::Path};

use crate::WorkspaceSaver;

use crate::{
    site::{
        ChildLiftCabinGroup, CollisionMeshMarker, DoorSegments, DrawingMarker, FloorSegments,
        LiftDoormat, ModelLoadingState, VisualMeshMarker,
    },
    Autoload, WorkspaceLoader,
};
use rmf_site_format::{
    IsStatic, LevelElevation, LiftCabin, ModelMarker, NameInSite, NameOfSite, SiteID, WallMarker,
};

/// Manages a simple state machine where we:
///   * Wait for a few iterations,
///   * Make sure the world is loaded.
///   * Send a save event.
///   * Wait for a few iterations.
///   * Exit.
#[derive(Debug, Resource)]
pub struct HeadlessSdfExportState {
    iterations: u32,
    world_loaded: bool,
    save_requested: bool,
    target_path: String,
}

impl HeadlessSdfExportState {
    pub fn new(path: &str) -> Self {
        Self {
            iterations: 0,
            world_loaded: false,
            save_requested: false,
            target_path: path.into(),
        }
    }
}

#[derive(Deref, DerefMut)]
pub struct ExportHandler(pub SystemId<In<(Entity, serde_json::Value)>, sdformat_rs::XmlElement>);

impl ExportHandler {
    pub fn new<M, S: IntoSystem<In<(Entity, serde_json::Value)>, sdformat_rs::XmlElement, M>>(
        system: S,
        world: &mut World,
    ) -> Self {
        let mut system = Box::new(IntoSystem::into_system(system));
        system.initialize(world);
        let system_id: SystemId<In<(Entity, serde_json::Value)>, sdformat_rs::XmlElement> =
            world.register_boxed_system(system);

        Self(system_id)
    }

    pub fn export(
        &mut self,
        entity: Entity,
        value: serde_json::Value,
        world: &mut World,
    ) -> Option<sdformat_rs::XmlElement> {
        world.run_system_with(self.0, (entity, value)).ok()
    }
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct ExportHandlers(pub HashMap<String, ExportHandler>);

impl ExportHandlers {
    pub fn insert(&mut self, label: String, handler: ExportHandler) {
        self.0.insert(label, handler);
    }
}

pub fn headless_sdf_export(
    mut commands: Commands,
    mut workspace_saver: WorkspaceSaver,
    mut exit: EventWriter<bevy::app::AppExit>,
    missing_models: Query<(), With<ModelLoadingState>>,
    mut export_state: ResMut<HeadlessSdfExportState>,
    sites: Query<(Entity, &NameOfSite)>,
    drawings: Query<Entity, With<DrawingMarker>>,
    autoload: Option<ResMut<Autoload>>,
    mut workspace_loader: WorkspaceLoader,
) {
    if let Some(mut autoload) = autoload {
        if let Some(filename) = autoload.filename.take() {
            workspace_loader.load_from_path(filename);
        }
    } else {
        error!("Cannot perform a headless export since no site file was specified for loading");
    }

    export_state.iterations += 1;
    if export_state.iterations < 5 {
        return;
    }
    if sites.is_empty() {
        warn!(
            "No site is loaded so we cannot export an SDF file into [{}]",
            export_state.target_path,
        );
        exit.write(bevy::app::AppExit::Error(1.try_into().unwrap()));
    }
    if !missing_models.is_empty() {
        // Despawn all drawings, otherwise floors will become transparent.
        for e in drawings.iter() {
            commands.entity(e).despawn();
        }
        // TODO(luca) implement a timeout logic?
    } else {
        if !export_state.world_loaded {
            export_state.iterations = 0;
            export_state.world_loaded = true;
        } else {
            if !export_state.save_requested && export_state.iterations > 5 {
                let path = std::path::PathBuf::from(export_state.target_path.clone());
                workspace_saver.export_sdf_to_path(path);
                export_state.save_requested = true;
                export_state.iterations = 0;
            } else if export_state.save_requested && export_state.iterations > 5 {
                exit.write(bevy::app::AppExit::Success);
            }
        }
    }
}

pub fn collect_site_meshes(world: &mut World, site: Entity, folder: &Path) -> Result<(), String> {
    let mut state: SystemState<(
        Query<&Children>,
        Query<(&NameInSite, &LevelElevation, &Children)>,
        Query<Entity, With<WallMarker>>,
        Query<&FloorSegments>,
        Query<(Option<&NameInSite>, &DoorSegments)>,
        Query<(Entity, &IsStatic, &NameInSite), With<ModelMarker>>,
        Query<(), With<CollisionMeshMarker>>,
        Query<(), With<VisualMeshMarker>>,
        Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
        Query<(&NameInSite, &LiftCabin<Entity>, &ChildLiftCabinGroup)>,
        Query<(), With<LiftDoormat>>,
        Query<&GlobalTransform>,
        Query<&Transform>,
        Query<&SiteID>,
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
        q_site_ids,
    ) = state.get(world);

    let image_assets = world.resource::<Assets<Image>>();
    let mesh_assets = world.resource::<Assets<Mesh>>();
    let material_assets = world.resource::<Assets<StandardMaterial>>();
    let write_meshes_to_file = |meshes: Vec<MeshData>,
                                name: Option<String>,
                                options: CompressGltfOptions,
                                filename: String|
     -> Result<(), String> {
        let image_getter = |id: &Handle<Image>| image_assets.get(id).cloned();
        let meshes =
            export_meshes(meshes, name, image_getter, options).map_err(|e| e.to_string())?;
        let bytes = meshes.to_bytes().map_err(|e| e.to_string())?;
        std::fs::write(filename, bytes).map_err(|e| e.to_string())
    };

    let get_site_id = |e: Entity| -> Result<u32, String> {
        q_site_ids.get(e).map(|id| id.0).map_err(|_| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            format!("Site ID was not available for entity {e:?}. Backtrace:\n{backtrace}")
        })
    };

    let get_mesh_and_material = |entity: Entity| -> Option<(&Mesh, &StandardMaterial)> {
        let Ok((mesh, material)) = q_pbr.get(entity) else {
            return None;
        };
        let Some(mesh) = mesh_assets.get(mesh) else {
            let site_id = q_site_ids.get(entity);
            warn!(
                "Mesh asset not found for entity {:?} with Site ID {:?} while exporting assets",
                entity, site_id
            );
            return None;
        };
        let Some(material) = material_assets.get(material) else {
            let site_id = q_site_ids.get(entity);
            warn!(
                "Material asset not found for entity {:?} with Site ID {:?} while exporting assets",
                entity, site_id
            );
            return None;
        };
        Some((mesh, material))
    };

    let Ok(site_children) = q_children.get(site) else {
        return Ok(());
    };
    for site_child in site_children.iter() {
        let mut collision_data = Vec::new();
        let mut visual_data = Vec::new();
        if let Ok((level_name, elevation, children)) = q_levels.get(site_child) {
            let level_tf = Transform {
                translation: Vec3::new(0.0, 0.0, **elevation),
                ..Default::default()
            };
            for child in children.iter() {
                if let Ok(res) = q_walls.get(child) {
                    let Some((mesh, material)) = get_mesh_and_material(res) else {
                        continue;
                    };
                    collision_data.push(MeshData {
                        mesh,
                        material: None,
                        transform: Some(level_tf.clone()),
                    });
                    visual_data.push(MeshData {
                        mesh,
                        material: Some(material),
                        transform: Some(level_tf.clone()),
                    });
                } else if let Ok(res) = q_floors.get(child) {
                    let Some((mesh, material)) = get_mesh_and_material(res.mesh) else {
                        continue;
                    };
                    collision_data.push(MeshData {
                        mesh,
                        material: None,
                        transform: Some(level_tf.clone()),
                    });
                    visual_data.push(MeshData {
                        mesh,
                        material: Some(material),
                        transform: Some(level_tf.clone()),
                    });
                } else if let Ok((model, is_static, name)) = q_models.get(child) {
                    let mut model_collisions = vec![];
                    let mut model_visuals = vec![];
                    // TODO(luca) don't do full descendant iter here or we might add twice?
                    // Iterate through children and select all meshes
                    for model_child in DescendantIter::new(&q_children, model) {
                        if q_collisions.contains(model_child) {
                            // Now iterate through the children of the collision and add them
                            for entity in DescendantIter::new(&q_children, model_child) {
                                let Some((mesh, _)) = get_mesh_and_material(entity) else {
                                    continue;
                                };
                                let Ok(tf) = q_global_tfs.get(entity) else {
                                    continue;
                                };
                                let mut tf = tf.compute_transform();
                                tf.translation.z = tf.translation.z + **elevation;
                                // Non static meshes have their translation in the SDF element, not in the
                                // gltf node
                                model_collisions.push(MeshData {
                                    mesh,
                                    material: None,
                                    transform: is_static.then_some(tf),
                                });
                            }
                        } else if q_visuals.contains(model_child) {
                            // Now iterate through the children of the visuals and add them
                            for entity in DescendantIter::new(&q_children, model_child) {
                                let Some((mesh, material)) = get_mesh_and_material(entity) else {
                                    continue;
                                };
                                let Ok(tf) = q_global_tfs.get(entity) else {
                                    continue;
                                };
                                let mut tf = tf.compute_transform();
                                tf.translation.z = tf.translation.z + **elevation;
                                model_visuals.push(MeshData {
                                    mesh,
                                    material: Some(material),
                                    transform: is_static.then_some(tf),
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
                        let filename = format!(
                            "{}/model_{}_collision.glb",
                            folder.display(),
                            get_site_id(child)?,
                        );
                        write_meshes_to_file(
                            model_collisions,
                            None,
                            CompressGltfOptions::skip_materials(),
                            filename,
                        )?;
                        let filename = format!(
                            "{}/model_{}_visual.glb",
                            folder.display(),
                            get_site_id(child)?,
                        );
                        write_meshes_to_file(
                            model_visuals,
                            Some(format!("{}_visual", **name)),
                            CompressGltfOptions::default(),
                            filename,
                        )?;
                    }
                } else if let Ok((door_name, segments)) = q_doors.get(child) {
                    for (entity, segment_name) in segments
                        .body
                        .entities()
                        .iter()
                        .zip(segments.body.links().into_iter())
                    {
                        // Generate the visual and collisions here
                        let Some((mesh, material)) = get_mesh_and_material(*entity) else {
                            continue;
                        };
                        let Ok(tf) = q_tfs.get(*entity) else {
                            continue;
                        };

                        let data = MeshData {
                            mesh,
                            material: Some(material),
                            transform: Some(tf.clone()),
                        };
                        let filename = format!(
                            "{}/door_{}_{}.glb",
                            folder.display(),
                            get_site_id(child)?,
                            segment_name,
                        );
                        let door_name = door_name.map(|n| n.0.as_str()).unwrap_or("");
                        write_meshes_to_file(
                            vec![data],
                            Some(format!("door_{}_{}", door_name, segment_name)),
                            CompressGltfOptions::default(),
                            filename,
                        )?;
                    }
                } else {
                    continue;
                };
            }
            let filename = format!(
                "{}/level_{}_collision.glb",
                folder.display(),
                get_site_id(site_child)?,
            );
            write_meshes_to_file(
                collision_data,
                Some(format!("level_{}_collision", **level_name)),
                CompressGltfOptions::skip_materials(),
                filename,
            )?;
            let filename = format!(
                "{}/level_{}_visual.glb",
                folder.display(),
                get_site_id(site_child)?,
            );
            write_meshes_to_file(
                visual_data,
                Some(format!("level_{}_visual", **level_name)),
                CompressGltfOptions::default(),
                filename,
            )?;
        }
        // Lifts
        if let Ok((lift_name, cabin, cabin_children)) = q_lift_cabins.get(site_child) {
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
                    transform: None,
                });
            }
            let filename = format!("{}/lift_{}.glb", folder.display(), get_site_id(site_child)?);
            write_meshes_to_file(
                lift_data,
                Some(format!("lift_{}", **lift_name)),
                CompressGltfOptions::default(),
                filename,
            )?;
            // Now generate the lift doors
            let LiftCabin::Rect(cabin) = cabin;
            for (face, door) in cabin.doors().iter() {
                let Some(door) = door else {
                    continue;
                };
                if let Ok((_, segments)) = q_doors.get(door.door) {
                    // TODO(luca) this is duplicated with door generation, refactor
                    for (entity, segment_name) in segments
                        .body
                        .entities()
                        .iter()
                        .zip(segments.body.links().into_iter())
                    {
                        // Generate the visual and collisions here
                        let Some((mesh, material)) = get_mesh_and_material(*entity) else {
                            continue;
                        };
                        let Ok(tf) = q_tfs.get(*entity) else {
                            continue;
                        };

                        let data = MeshData {
                            mesh,
                            material: Some(material),
                            transform: Some(tf.clone()),
                        };
                        let filename = format!(
                            "{}/lift_{}_{}_{}.glb",
                            folder.display(),
                            get_site_id(site_child)?,
                            face.label(),
                            segment_name,
                        );
                        write_meshes_to_file(
                            vec![data],
                            Some(format!(
                                "lift_{}_{}_{}",
                                **lift_name,
                                face.label(),
                                segment_name
                            )),
                            CompressGltfOptions::default(),
                            filename,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
