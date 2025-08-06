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

use crate::RosMesh;

use thiserror::Error;

use bevy::prelude::*;
use librmf_site_editor::interaction::DragPlaneBundle;
use librmf_site_editor::site::Model as SiteModel;
use librmf_site_editor::site::Pose as SitePose;
use librmf_site_editor::site::{
    AssetSource, Category, DirectionalLight, IsStatic, Light, LightKind, ModelLoader, ModelMarker,
    NameInSite, PointLight, PrimitiveShape, Rotation, Scale, SpotLight, VisualMeshMarker,
};

use std::collections::VecDeque;

#[derive(Error, Debug)]
pub enum SceneLoadingError {
    #[error("Unable to retrieve AssetSource from Mesh filename: {0}")]
    MeshFilenameNotFound(String),
    #[error("No geometry found: {0}")]
    GeometryNotFound(String),
}

pub(crate) fn generate_scene(
    In(RosMesh { mesh_resource, sum }): In<RosMesh>,
    mut commands: Commands,
    mut model_loader: ModelLoader,
    children: Query<&Children>,
) {
    println!(
        "----- Inside generate scene! getting resource: {:?}, sum: {}",
        mesh_resource, sum
    );
    // // Despawn any old children to clear space for the new scene
    // if let Ok(children) = children.get(scene_root) {
    //     for child in children {
    //         if let Some(e) = commands.get_entity(*child) {
    //             e.despawn_recursive();
    //         }
    //     }
    // }

    // for scene_model in &scene.model {
    //     let mut queue = VecDeque::<(Entity, &Model)>::new();
    //     queue.push_back((scene_root, scene_model));
    //     while let Some((parent, model)) = queue.pop_front() {
    //         let model_pose = parse_pose(&model.pose);
    //         let model_entity = commands
    //             .spawn(SpatialBundle::from_transform(model_pose.transform()))
    //             .set_parent(parent)
    //             .id();

    //         for link in &model.link {
    //             let link_pose = parse_pose(&link.pose);
    //             let link_id = commands
    //                 .spawn(SpatialBundle::from_transform(link_pose.transform()))
    //                 .set_parent(model_entity)
    //                 .id();

    //             for visual in &link.visual {
    //                 if let Ok(id) = spawn_geometry(
    //                     &mut commands,
    //                     &visual.geometry,
    //                     &visual.pose,
    //                     &visual.name,
    //                     model.is_static,
    //                     scene_root, // If any link is selected, the root scene will be selected
    //                     &mut model_loader,
    //                 ) {
    //                     match id {
    //                         Some(id) => {
    //                             commands
    //                                 .entity(id)
    //                                 .insert(VisualMeshMarker)
    //                                 .insert(Category::Visual)
    //                                 .set_parent(link_id);
    //                         }
    //                         None => warn!("Found unhandled geometry type {:?}", &visual.geometry),
    //                     }
    //                 }
    //             }
    //         }

    //         for submodel in &model.model {
    //             queue.push_back((model_entity, submodel));
    //         }
    //     }
    // }

    // for light in &scene.light {
    //     if light.is_light_off {
    //         continue;
    //     }
    //     let pose = parse_pose(&light.pose);
    //     let light_type = light.r#type;
    //     if light_type == LightType::Point as i32 {
    //         let _ = commands
    //             .spawn(Light {
    //                 pose,
    //                 kind: LightKind::Point(PointLight {
    //                     color: light
    //                         .diffuse
    //                         .clone()
    //                         .or(light.specular.clone())
    //                         .map(|color| [color.r, color.g, color.b, color.a])
    //                         .unwrap_or([1.0; 4]),
    //                     intensity: light.intensity.clone(),
    //                     range: light.range.clone(),
    //                     radius: 0.0,
    //                     enable_shadows: light.cast_shadows,
    //                 }),
    //             })
    //             .set_parent(scene_root)
    //             .id();
    //     } else if light_type == LightType::Spot as i32 {
    //         let _ = commands
    //             .spawn(Light {
    //                 pose,
    //                 kind: LightKind::Spot(SpotLight {
    //                     color: light
    //                         .diffuse
    //                         .clone()
    //                         .or(light.specular.clone())
    //                         .map(|color| [color.r, color.g, color.b, color.a])
    //                         .unwrap_or([1.0; 4]),
    //                     intensity: light.intensity.clone(),
    //                     range: light.range.clone(),
    //                     radius: 0.0,
    //                     enable_shadows: light.cast_shadows,
    //                 }),
    //             })
    //             .set_parent(scene_root)
    //             .id();
    //     } else if light_type == LightType::Directional as i32 {
    //         let _ = commands
    //             .spawn(Light {
    //                 pose,
    //                 kind: LightKind::Directional(DirectionalLight {
    //                     color: match &light.specular {
    //                         Some(color) => [color.r, color.g, color.b, color.a],
    //                         None => [0.0; 4],
    //                     },
    //                     illuminance: light.intensity.clone(), // Assume area is small
    //                     enable_shadows: light.cast_shadows,
    //                 }),
    //             })
    //             .set_parent(scene_root)
    //             .id();
    //     }
    // }
}

// fn parse_pose(scene_pose: &Option<Pose>) -> SitePose {
//     let Some(scene_pose) = scene_pose else {
//         return SitePose::default();
//     };

//     SitePose {
//         trans: match &scene_pose.position {
//             Some(position) => [position.x as f32, position.y as f32, position.z as f32],
//             None => [0.0; 3],
//         },
//         rot: match &scene_pose.orientation {
//             Some(orientation) => Rotation::Quat([
//                 orientation.x as f32,
//                 orientation.y as f32,
//                 orientation.z as f32,
//                 orientation.w as f32,
//             ]),
//             None => Rotation::default(),
//         },
//     }
// }

// fn spawn_geometry(
//     commands: &mut Commands,
//     geometry: &Option<Geometry>,
//     pose: &Option<Pose>,
//     name: &String,
//     is_static: bool,
//     root: Entity,
//     model_loader: &mut ModelLoader,
// ) -> Result<Option<Entity>, SceneLoadingError> {
//     let pose = parse_pose(pose);
//     match geometry {
//         Some(geom) => {
//             let geom_type = geom.r#type;
//             if geom_type == Type::Mesh as i32 {
//                 if let Some(mesh) = &geom.mesh {
//                     let uri = if let Some(stripped) = mesh.filename.strip_prefix("model://") {
//                         stripped
//                     } else {
//                         mesh.filename.as_str()
//                     };
//                     let asset_source = AssetSource::Local(uri.to_string());
//                     let mesh_entity = commands
//                         .spawn(SiteModel {
//                             name: NameInSite(name.to_owned()),
//                             source: asset_source.clone(),
//                             pose,
//                             is_static: IsStatic(is_static),
//                             scale: match &mesh.scale {
//                                 Some(scale) => {
//                                     Scale(Vec3::new(scale.x as f32, scale.y as f32, scale.z as f32))
//                                 }
//                                 None => Scale::default(),
//                             },
//                             marker: ModelMarker,
//                         })
//                         .id();
//                     let interaction = DragPlaneBundle::new(root, Vec3::Z).globally();
//                     model_loader
//                         .update_asset_source_impulse(
//                             mesh_entity,
//                             asset_source,
//                             Some(interaction.clone()),
//                         )
//                         .detach();
//                     return Ok(Some(mesh_entity));
//                 }
//             } else if geom_type == Type::Box as i32 {
//                 if let Some(box_size) = geom.r#box.clone().and_then(|b| b.size) {
//                     return Ok(Some(
//                         commands
//                             .spawn(PrimitiveShape::Box {
//                                 size: [box_size.x as f32, box_size.y as f32, box_size.z as f32],
//                             })
//                             .insert(NameInSite(name.to_owned()))
//                             .insert(SpatialBundle::from_transform(pose.transform()))
//                             .id(),
//                     ));
//                 }
//             } else if geom_type == Type::Cylinder as i32 {
//                 if let Some((radius, length)) = geom.cylinder.clone().map(|c| (c.radius, c.length))
//                 {
//                     return Ok(Some(
//                         commands
//                             .spawn(PrimitiveShape::Cylinder {
//                                 radius: radius as f32,
//                                 length: length as f32,
//                             })
//                             .insert(pose)
//                             .insert(NameInSite(name.to_owned()))
//                             .insert(SpatialBundle::from_transform(pose.transform()))
//                             .id(),
//                     ));
//                 }
//             } else if geom_type == Type::Capsule as i32 {
//                 if let Some((radius, length)) = geom.capsule.clone().map(|c| (c.radius, c.length)) {
//                     return Ok(Some(
//                         commands
//                             .spawn(PrimitiveShape::Capsule {
//                                 radius: radius as f32,
//                                 length: length as f32,
//                             })
//                             .insert(pose)
//                             .insert(NameInSite(name.to_owned()))
//                             .insert(SpatialBundle::from_transform(pose.transform()))
//                             .id(),
//                     ));
//                 }
//             } else if geom_type == Type::Sphere as i32 {
//                 if let Some(radius) = geom.sphere.clone().map(|s| s.radius) {
//                     return Ok(Some(
//                         commands
//                             .spawn(PrimitiveShape::Sphere {
//                                 radius: radius as f32,
//                             })
//                             .insert(pose)
//                             .insert(NameInSite(name.to_owned()))
//                             .insert(SpatialBundle::from_transform(pose.transform()))
//                             .id(),
//                     ));
//                 }
//             }
//         }
//         None => {}
//     }

//     Err(SceneLoadingError::GeometryNotFound(name.clone()))
// }
