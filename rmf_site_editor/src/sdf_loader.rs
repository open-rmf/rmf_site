/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;

use bevy::utils::BoxedFuture;
use futures_lite::AsyncReadExt;

use thiserror::Error;

use sdformat_rs::{SdfGeometry, SdfPose, Vector3d};

use crate::site::{CollisionMeshMarker, VisualMeshMarker};
use rmf_site_format::{
    Angle, AssetSource, Category, IsStatic, Model, ModelMarker, NameInSite, Pose, PrimitiveShape,
    Rotation, Scale,
};

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<SdfLoader>();
    }
}

#[derive(Default)]
struct SdfLoader;

impl AssetLoader for SdfLoader {
    type Asset = bevy::scene::Scene;
    type Settings = ();
    type Error = SdfError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            // TODO(luca) remove unwrap
            reader.read_to_end(&mut bytes).await.unwrap();
            Ok(load_model(bytes, load_context).await?)
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["sdf"];
        EXTENSIONS
    }
}

#[derive(Error, Debug)]
pub enum SdfError {
    #[error("Couldn't read SDF file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Yaserde loading error: {0}")]
    YaserdeError(String),
    #[error("No <model> tag found in model.sdf file")]
    MissingModelTag,
    #[error("Failed parsing asset source: {0}")]
    UnsupportedAssetSource(String),
    #[error("Failed reading mesh at source: {0}")]
    MissingMesh(String),
}

/// Combines the path from the SDF that is currently being processed with the path of a mesh
/// referenced in the SDF to generate an AssetSource that can be loaded by the AssetServer.
fn compute_model_source<'a, 'b>(
    load_context: &'a mut LoadContext<'b>,
    subasset_uri: &'a str,
) -> Result<AssetSource, SdfError> {
    // SDF can reference models with the model:// syntax, which specifies a path relative to a certain
    // model, in the shape of model://ModelName/path_to_file.ext
    let asset_source = if let Some(stripped) = subasset_uri.strip_prefix("model://") {
        let mut asset_source =
            AssetSource::try_from(load_context.asset_path().to_string().as_str())
                .map_err(SdfError::UnsupportedAssetSource)?;
        match asset_source {
            AssetSource::Remote(ref mut p) | AssetSource::Search(ref mut p) => {
                // When working with AssetSource::Remote and AssetSource::Search types, the form is
                // `Organization/ModelName/path_to_file.ext`, hence we substitute the part after `Organization/`
                // with the content of the model:// path.
                // For example: A OpenRobotics/Model1/model.sdf that references to a
                // model://Model2/mesh.obj would be parsed to OpenRobotics/Model2/mesh.obj
                // TODO(luca) Should remote and search `AssetSource` objects have a clear split for
                // organization name and asset name instead of relying on an implicit
                // Organization/Model syntax?
                *p = if let Some(org_name) = p.split("/").next() {
                    org_name.to_owned() + "/" + stripped
                } else {
                    return Err(SdfError::UnsupportedAssetSource(format!(
                        "Unable to extract organization name from asset source [{}]",
                        subasset_uri
                    )));
                }
            }
            AssetSource::Local(ref mut p) | AssetSource::Package(ref mut p) => {
                // Search for a model with the requested name in the same folder as the sdf file by
                // navigating the path up by two levels (removing file name and model folder).
                // For example, if an SDF in /home/user/Model1/model.sdf refers to
                // model://Model2/meshes/mesh.obj, this function will try to load
                // /home/user/Model2/meshes/mesh.obj.
                // Note that this will not play well if the requested model shares files with other
                // models that are placed in different folders or are in fuel, but should work for
                // most local, self contained, models.
                *p = if let Some(model_folder) = p.rsplitn(3, "/").skip(2).next() {
                    model_folder.to_owned() + "/" + stripped
                } else {
                    return Err(SdfError::UnsupportedAssetSource(format!(
                        "Unable to extract model folder from asset source [{}]",
                        subasset_uri
                    )));
                }
            }
        }
        Ok(asset_source)
    } else {
        // It's a path relative to this model, concatenate it to the current context path.
        let path = load_context
            .asset_path()
            .resolve(subasset_uri)
            .or_else(|e| Err(SdfError::UnsupportedAssetSource(e.to_string())))?;
        AssetSource::try_from(path.to_string().as_str()).map_err(SdfError::UnsupportedAssetSource)
    }?;
    // Fire a loading here so we can make sure all the sub assets are retrievable and prefetched
    // Unwrap is safe since this was constructed above and the try_from calls would have failed
    async move {
        let asset_source_string = String::try_from(&asset_source).unwrap();
        load_context
            .load_direct(asset_source_string)
            .await
            .map_err(|_| SdfError::MissingMesh(asset_source_string))
    }?;
    Ok(asset_source)
}

fn parse_scale(scale: &Option<Vector3d>) -> Scale {
    match scale {
        Some(v) => Scale(Vec3::new(v.0.x as f32, v.0.y as f32, v.0.z as f32)),
        None => Scale::default(),
    }
}

fn parse_pose(pose: &Option<SdfPose>) -> Pose {
    if let Some(pose) = pose.clone().and_then(|p| p.get_pose().ok()) {
        let rot = pose.rotation.euler_angles();
        Pose {
            trans: [
                pose.translation.x as f32,
                pose.translation.y as f32,
                pose.translation.z as f32,
            ],
            rot: Rotation::EulerExtrinsicXYZ([
                Angle::Rad(rot.0 as f32),
                Angle::Rad(rot.1 as f32),
                Angle::Rad(rot.2 as f32),
            ]),
        }
    } else {
        Pose::default()
    }
}

fn spawn_geometry<'a, 'b>(
    world: &'a mut World,
    geometry: &'a SdfGeometry,
    geometry_name: &'a str,
    pose: &'a Option<SdfPose>,
    load_context: &'a mut LoadContext<'b>,
    is_static: bool,
) -> Result<Option<Entity>, SdfError> {
    let pose = parse_pose(pose);
    let geometry = match geometry {
        SdfGeometry::Mesh(mesh) => Some(
            world
                .spawn(Model {
                    name: NameInSite(geometry_name.to_owned()),
                    source: compute_model_source(load_context, &mesh.uri)?,
                    pose,
                    is_static: IsStatic(is_static),
                    scale: parse_scale(&mesh.scale),
                    marker: ModelMarker,
                })
                .id(),
        ),
        SdfGeometry::Box(b) => {
            let s = &b.size.0;
            Some(
                world
                    .spawn(PrimitiveShape::Box {
                        size: [s.x as f32, s.y as f32, s.z as f32],
                    })
                    .insert(pose)
                    .insert(NameInSite(geometry_name.to_owned()))
                    .insert(SpatialBundle::INHERITED_IDENTITY)
                    .id(),
            )
        }
        SdfGeometry::Capsule(c) => Some(
            world
                .spawn(PrimitiveShape::Capsule {
                    radius: c.radius as f32,
                    length: c.length as f32,
                })
                .insert(pose)
                .insert(NameInSite(geometry_name.to_owned()))
                .insert(SpatialBundle::INHERITED_IDENTITY)
                .id(),
        ),
        SdfGeometry::Cylinder(c) => Some(
            world
                .spawn(PrimitiveShape::Cylinder {
                    radius: c.radius as f32,
                    length: c.length as f32,
                })
                .insert(pose)
                .insert(NameInSite(geometry_name.to_owned()))
                .insert(SpatialBundle::INHERITED_IDENTITY)
                .id(),
        ),
        SdfGeometry::Sphere(s) => Some(
            world
                .spawn(PrimitiveShape::Sphere {
                    radius: s.radius as f32,
                })
                .insert(pose)
                .insert(NameInSite(geometry_name.to_owned()))
                .insert(SpatialBundle::INHERITED_IDENTITY)
                .id(),
        ),
        _ => None,
    };
    Ok(geometry)
}

async fn load_model<'a, 'b>(
    bytes: Vec<u8>,
    load_context: &'a mut LoadContext<'b>,
) -> Result<bevy::scene::Scene, SdfError> {
    let sdf_str = std::str::from_utf8(&bytes).unwrap();
    let root = sdformat_rs::from_str::<sdformat_rs::SdfRoot>(sdf_str);
    match root {
        Ok(root) => {
            if let Some(model) = root.model {
                let mut world = World::default();
                let e = world.spawn(SpatialBundle::INHERITED_IDENTITY).id();
                // TODO(luca) hierarchies and joints, rather than flat link importing
                // All Open-RMF assets have no hierarchy, for now.
                for link in &model.link {
                    let link_pose = parse_pose(&link.pose);
                    let link_id = world
                        .spawn(SpatialBundle::from_transform(link_pose.transform()))
                        .id();
                    world.entity_mut(e).add_child(link_id);
                    for visual in &link.visual {
                        let id = spawn_geometry(
                            &mut world,
                            &visual.geometry,
                            &visual.name,
                            &visual.pose,
                            load_context,
                            model.r#static.unwrap_or(false),
                        )?;
                        match id {
                            Some(id) => {
                                world
                                    .entity_mut(id)
                                    .insert(VisualMeshMarker)
                                    .insert(Category::Visual)
                                    .set_parent(link_id);
                            }
                            None => warn!("Found unhandled geometry type {:?}", &visual.geometry),
                        }
                    }
                    for collision in &link.collision {
                        let id = spawn_geometry(
                            &mut world,
                            &collision.geometry,
                            &collision.name,
                            &collision.pose,
                            load_context,
                            model.r#static.unwrap_or(false),
                        )?;
                        match id {
                            Some(id) => {
                                world
                                    .entity_mut(id)
                                    .insert(CollisionMeshMarker)
                                    .insert(Category::Collision)
                                    .set_parent(link_id);
                            }
                            None => {
                                warn!("Found unhandled geometry type {:?}", &collision.geometry)
                            }
                        }
                    }
                }
                Ok(bevy::scene::Scene::new(world))
            } else {
                Err(SdfError::MissingModelTag)
            }
        }
        Err(err) => Err(SdfError::YaserdeError(err)),
    }
}
