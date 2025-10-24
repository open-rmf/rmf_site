/*
 * Copyright (C) 2025 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use bevy::ecs::{
    relationship::DescendantIter,
    system::{SystemId, SystemState},
};
use bevy::prelude::*;
use bevy_gltf_export::{CompressGltfOptions, MeshData, export_meshes};

use std::{collections::HashMap, path::Path};

use crate::site::{
    ChildLiftCabinGroup, CollisionMeshMarker, DoorSegments, FloorSegments, Group, LiftDoormat,
    VisualMeshMarker,
};
use rmf_site_format::{
    Affiliation, IsStatic, LevelElevation, LiftCabin, ModelMarker, NameInSite, SiteID, WallMarker,
};

#[derive(Deref, DerefMut)]
pub struct ExportHandler(pub SystemId<In<(Entity, serde_json::Value)>, sdformat::XmlElement>);

impl ExportHandler {
    pub fn new<M, S: IntoSystem<In<(Entity, serde_json::Value)>, sdformat::XmlElement, M>>(
        system: S,
        world: &mut World,
    ) -> Self {
        let mut system = Box::new(IntoSystem::into_system(system));
        system.initialize(world);
        let system_id: SystemId<In<(Entity, serde_json::Value)>, sdformat::XmlElement> =
            world.register_boxed_system(system);

        Self(system_id)
    }

    pub fn export(
        &mut self,
        entity: Entity,
        value: serde_json::Value,
        world: &mut World,
    ) -> Option<sdformat::XmlElement> {
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

pub fn collect_site_meshes(world: &mut World, site: Entity, folder: &Path) -> Result<(), String> {
    let mut state: SystemState<(
        Query<&Children>,
        Query<(&NameInSite, &LevelElevation, &Children)>,
        Query<Entity, With<WallMarker>>,
        Query<&FloorSegments>,
        Query<(Option<&NameInSite>, &DoorSegments)>,
        Query<(&NameInSite, &IsStatic, &Affiliation<Entity>), (With<ModelMarker>, Without<Group>)>,
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

    let mut description_meshes = HashMap::new();

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
                } else if let Ok((name, is_static, affiliation)) = q_models.get(child) {
                    if **is_static {
                        // static meshes are incorporated directly into the world.
                        append_collisions_and_visuals(
                            child,
                            **elevation,
                            true,
                            &q_children,
                            &q_collisions,
                            &q_visuals,
                            &q_global_tfs,
                            get_mesh_and_material,
                            &mut collision_data,
                            &mut visual_data,
                        );
                    } else if let Some(description) = affiliation.0 {
                        // non-static (robot) meshes are exported once per
                        // description and shared across all instances.
                        let mut collisions = Vec::new();
                        let mut visuals = Vec::new();

                        append_collisions_and_visuals(
                            child,
                            **elevation,
                            false,
                            &q_children,
                            &q_collisions,
                            &q_visuals,
                            &q_global_tfs,
                            get_mesh_and_material,
                            &mut collisions,
                            &mut visuals,
                        );

                        description_meshes.insert(
                            description,
                            ModelDescriptionMeshes {
                                name: name.0.clone(),
                                collisions,
                                visuals,
                            },
                        );
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

    for (description, meshes) in description_meshes {
        // Create a new mesh for it
        let filename = format!(
            "{}/model_{}_collision.glb",
            folder.display(),
            get_site_id(description)?,
        );
        write_meshes_to_file(
            meshes.collisions,
            None,
            CompressGltfOptions::skip_materials(),
            filename,
        )?;
        let filename = format!(
            "{}/model_{}_visual.glb",
            folder.display(),
            get_site_id(description)?,
        );
        write_meshes_to_file(
            meshes.visuals,
            Some(format!("{}_visual", meshes.name)),
            CompressGltfOptions::default(),
            filename,
        )?;
    }

    Ok(())
}

fn append_collisions_and_visuals<'a>(
    model: Entity,
    elevation: f32,
    is_static: bool,
    q_children: &Query<&Children>,
    q_collisions: &Query<(), With<CollisionMeshMarker>>,
    q_visuals: &Query<(), With<VisualMeshMarker>>,
    q_global_tfs: &Query<&GlobalTransform>,
    get_mesh_and_material: impl Fn(Entity) -> Option<(&'a Mesh, &'a StandardMaterial)> + 'a,
    collision_data: &mut Vec<MeshData<'a>>,
    visual_data: &mut Vec<MeshData<'a>>,
) {
    // For non static models, we want every submesh to be translated to the origin by applying the
    // inverse of the model root's global transform.
    // This is done because the pose of non static models is set through SDF, not the GLB itself
    // but we want to preserve relative poses of the model's submeshes
    let root_inverse_pose = if !is_static {
        q_global_tfs.get(model).ok().map(|tf| tf.affine().inverse())
    } else {
        None
    };

    let add_children_data =
        |model_child: Entity, data_vec: &mut Vec<MeshData<'a>>, add_material: bool| {
            for entity in DescendantIter::new(&q_children, model_child) {
                let Some((mesh, material)) = get_mesh_and_material(entity) else {
                    continue;
                };
                let Ok(tf) = q_global_tfs.get(entity) else {
                    continue;
                };
                let mut tf = tf.affine();
                if let Some(root_inverse_pose) = root_inverse_pose {
                    tf = root_inverse_pose * tf;
                }
                let mut tf = Transform::from_matrix(tf.into());
                tf.translation.z = tf.translation.z + elevation;
                data_vec.push(MeshData {
                    mesh,
                    material: add_material.then_some(material),
                    transform: Some(tf),
                });
            }
        };

    // Iterate through children and select all meshes
    for model_child in DescendantIter::new(&q_children, model) {
        if q_collisions.contains(model_child) {
            add_children_data(model_child, collision_data, false);
        } else if q_visuals.contains(model_child) {
            add_children_data(model_child, visual_data, true);
        }
    }
}

struct ModelDescriptionMeshes<'a> {
    name: String,
    collisions: Vec<MeshData<'a>>,
    visuals: Vec<MeshData<'a>>,
}
