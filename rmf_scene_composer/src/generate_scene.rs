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

use crate::protos::gz::msgs::{Geometry, Pose, Scene, geometry::Type, light::LightType};

use thiserror::Error;

use bevy::prelude::*;
use librmf_site_editor::interaction::DragPlaneBundle;
use librmf_site_editor::site::{
    AssetSource, Category, CollisionMeshMarker, DirectionalLight, IsStatic, Light, LightKind,
    Model, ModelLoader, ModelMarker, NameInSite, PointLight, PrimitiveShape, Rotation, Scale,
    SpotLight, VisualMeshMarker,
};

use librmf_site_editor::site::Pose as SitePose;

#[derive(Error, Debug)]
pub enum SceneLoadingError {
    #[error("Unable to retrieve AssetSource from Mesh filename: {0}")]
    MeshFilenameNotFound(String),
    #[error("No geometry found: {0}")]
    GeometryNotFound(String),
}

fn generate_scene(
    root: Entity,
    scene: Scene,
    commands: &mut Commands,
    model_loader: &mut ModelLoader,
) {
    for model in &scene.model {
        let model_entity = commands
            .spawn(SpatialBundle::INHERITED_IDENTITY)
            .set_parent(root)
            .id();

        for link in &model.link {
            let link_pose = parse_pose(&link.pose);
            let link_id = commands
                .spawn(SpatialBundle::from_transform(link_pose.transform()))
                .set_parent(model_entity)
                .id();

            for visual in &link.visual {
                if let Ok(id) = spawn_geometry(
                    commands,
                    &visual.geometry,
                    &visual.pose,
                    &visual.name,
                    model.is_static,
                    root,
                    model_loader,
                ) {
                    match id {
                        Some(id) => {
                            commands
                                .entity(id)
                                .insert(VisualMeshMarker)
                                .insert(Category::Visual)
                                .set_parent(link_id);
                        }
                        None => warn!("Found unhandled geometry type {:?}", &visual.geometry),
                    }
                }
            }

            for collision in &link.collision {
                if let Ok(id) = spawn_geometry(
                    commands,
                    &collision.geometry,
                    &collision.pose,
                    &collision.name,
                    model.is_static,
                    root,
                    model_loader,
                ) {
                    match id {
                        Some(id) => {
                            commands
                                .entity(id)
                                .insert(CollisionMeshMarker)
                                .insert(Category::Visual)
                                .set_parent(link_id);
                        }
                        None => warn!("Found unhandled geometry type {:?}", &collision.geometry),
                    }
                }
            }
        }
    }

    for light in &scene.light {
        if light.is_light_off {
            continue;
        }
        let pose = parse_pose(&light.pose);
        let light_type = light.r#type;
        if light_type == LightType::Point as i32 {
            let _ = commands
                .spawn(Light {
                    pose,
                    kind: LightKind::Point(PointLight {
                        // NOTE(@xiyuoh) assume specular
                        color: match &light.specular {
                            Some(color) => [color.r, color.g, color.b, color.a],
                            None => [0.0; 4],
                        },
                        intensity: light.intensity.clone(),
                        range: light.range.clone(),
                        radius: 0.0, // TODO(@xiyuoh)
                        enable_shadows: light.cast_shadows,
                    }),
                })
                .set_parent(root)
                .id();
        } else if light_type == LightType::Spot as i32 {
            let _ = commands
                .spawn(Light {
                    pose,
                    kind: LightKind::Spot(SpotLight {
                        color: match &light.specular {
                            Some(color) => [color.r, color.g, color.b, color.a],
                            None => [0.0; 4],
                        },
                        intensity: light.intensity.clone(),
                        range: light.range.clone(),
                        radius: 0.0, // TODO(@xiyuoh)
                        enable_shadows: light.cast_shadows,
                    }),
                })
                .set_parent(root)
                .id();
        } else if light_type == LightType::Directional as i32 {
            let _ = commands
                .spawn(Light {
                    pose,
                    kind: LightKind::Directional(DirectionalLight {
                        color: match &light.specular {
                            Some(color) => [color.r, color.g, color.b, color.a],
                            None => [0.0; 4],
                        },
                        illuminance: light.intensity.clone(), // TODO(@xiyuoh) check this
                        enable_shadows: light.cast_shadows,
                    }),
                })
                .set_parent(root)
                .id();
        }
    }

    // TODO(@xiyuoh) check if we need this; if scene is not for editing then maybe not
    for joint in scene.joint {}
}

fn parse_pose(scene_pose: &Option<Pose>) -> SitePose {
    let Some(scene_pose) = scene_pose else {
        return SitePose::default();
    };

    SitePose {
        trans: match &scene_pose.position {
            Some(position) => [position.x as f32, position.y as f32, position.z as f32],
            None => [0.0; 3],
        },
        rot: match &scene_pose.orientation {
            Some(orientation) => Rotation::Quat([
                orientation.x as f32,
                orientation.y as f32,
                orientation.z as f32,
                orientation.w as f32,
            ]),
            None => Rotation::default(),
        },
    }
}

fn spawn_geometry(
    commands: &mut Commands,
    geometry: &Option<Geometry>,
    pose: &Option<Pose>,
    name: &String,
    is_static: bool,
    root: Entity,
    model_loader: &mut ModelLoader,
) -> Result<Option<Entity>, SceneLoadingError> {
    let pose = parse_pose(pose);
    match geometry {
        Some(geom) => {
            let geom_type = geom.r#type;
            if geom_type == Type::Mesh as i32 {
                if let Some(mesh) = &geom.mesh {
                    let asset_source =
                        AssetSource::try_from(mesh.filename.as_str()).map_err(|_| {
                            SceneLoadingError::MeshFilenameNotFound(mesh.filename.clone())
                        })?;
                    let mesh_entity = commands
                        .spawn(Model {
                            name: NameInSite(name.to_owned()),
                            source: asset_source.clone(),
                            pose,
                            is_static: IsStatic(is_static),
                            scale: match &mesh.scale {
                                Some(scale) => {
                                    Scale(Vec3::new(scale.x as f32, scale.y as f32, scale.z as f32))
                                }
                                None => Scale::default(),
                            },
                            marker: ModelMarker,
                        })
                        .id();
                    let interaction = DragPlaneBundle::new(root, Vec3::Z);
                    model_loader.update_asset_source(mesh_entity, asset_source, Some(interaction));
                    return Ok(Some(mesh_entity));
                }
            } else if geom_type == Type::Box as i32 {
                if let Some(box_size) = geom.r#box.clone().and_then(|b| b.size) {
                    return Ok(Some(
                        commands
                            .spawn(PrimitiveShape::Box {
                                size: [box_size.x as f32, box_size.y as f32, box_size.z as f32],
                            })
                            .insert(pose)
                            .insert(NameInSite(name.to_owned()))
                            .insert(SpatialBundle::INHERITED_IDENTITY)
                            .id(),
                    ));
                }
            } else if geom_type == Type::Cylinder as i32 {
                if let Some((radius, length)) = geom.cylinder.clone().map(|c| (c.radius, c.length))
                {
                    return Ok(Some(
                        commands
                            .spawn(PrimitiveShape::Cylinder {
                                radius: radius as f32,
                                length: length as f32,
                            })
                            .insert(pose)
                            .insert(NameInSite(name.to_owned()))
                            .insert(SpatialBundle::INHERITED_IDENTITY)
                            .id(),
                    ));
                }
            } else if geom_type == Type::Capsule as i32 {
                if let Some((radius, length)) = geom.capsule.clone().map(|c| (c.radius, c.length)) {
                    return Ok(Some(
                        commands
                            .spawn(PrimitiveShape::Capsule {
                                radius: radius as f32,
                                length: length as f32,
                            })
                            .insert(pose)
                            .insert(NameInSite(name.to_owned()))
                            .insert(SpatialBundle::INHERITED_IDENTITY)
                            .id(),
                    ));
                }
            } else if geom_type == Type::Sphere as i32 {
                if let Some(radius) = geom.sphere.clone().map(|s| s.radius) {
                    return Ok(Some(
                        commands
                            .spawn(PrimitiveShape::Sphere {
                                radius: radius as f32,
                            })
                            .insert(pose)
                            .insert(NameInSite(name.to_owned()))
                            .insert(SpatialBundle::INHERITED_IDENTITY)
                            .id(),
                    ));
                }
            }
        }
        None => {}
    }

    Err(SceneLoadingError::GeometryNotFound(name.clone()))
}
