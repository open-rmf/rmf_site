use std::{any::{type_name, TypeId}, collections::HashMap};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_asset::prelude::*;
use bevy_math::primitives;
use bevy_pbr::StandardMaterial;
use bevy_reflect::Reflect;
use bevy_render::mesh::Mesh;
use bevy_color::palettes::css as Colors;
use bevy_utils::default;
use bytemuck::TransparentWrapper;
use bevy_reflect::prelude::*;

#[derive(Resource, Reflect)]
pub struct CameraOrbitMat(pub Handle<StandardMaterial>);

impl FromWorld for CameraOrbitMat {
    fn from_world(world: &mut World) -> Self {
        let mut mats = world.get_resource_mut::<Assets<StandardMaterial>>().unwrap();

        Self(
            mats.add(
        StandardMaterial {
                base_color: Colors::GREEN.into(),
                emissive: Colors::LIME.into(),
                depth_bias: f32::MAX,
                unlit: true,
                ..default()
                }
            )
        )
    }
}

#[derive(Resource, Reflect)]
pub struct CameraControlMesh(pub Handle<Mesh>);

impl FromWorld for CameraControlMesh {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();

        Self( meshes.add(Mesh::from(primitives::Sphere::new(0.02))))
    }
}

#[derive(Resource, Reflect)]
pub struct CameraControlPanMaterial(pub Handle<StandardMaterial>);

impl FromWorld for CameraControlPanMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut mats = world.get_resource_mut::<Assets<StandardMaterial>>().unwrap();
        Self(
            mats.add(
                StandardMaterial {
                    base_color: Colors::WHITE.into(),
                    emissive: Colors::WHITE.into(),
                    unlit: true,
                    ..default()
                } 
            )
        )
    }
}

/// currently enabled camera.
#[derive(PartialEq, Debug, Copy, Clone, Reflect, Resource, Default)]
#[reflect(Resource)]
pub enum ProjectionMode {
    #[default]
    Perspective,
    Orthographic,
}

/// weather camera is controlable or not. enabled/disabled by [`CameraBlockerRegistry`]
#[derive(Resource)]
pub struct CameraControlBlocked(pub(crate) bool);


impl Default for CameraControlBlocked {
    fn default() -> Self {
        Self(false)
    }
}

/// convenience struct for associating type info and type name.
#[derive(Reflect, Hash, PartialEq, Eq, Debug)]
pub struct TypeInfo {
    type_id: TypeId,
    type_name: String,
}

impl TypeInfo {
    pub fn new<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>().to_string(),
            
        }
    }
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
    pub fn type_name(&self) -> &String {
        &self.type_name
    }
}

/// registry of things that can block camera controls
#[derive(Resource, Reflect, TransparentWrapper, Default)]
#[reflect(Resource)]
#[repr(transparent)]
pub struct CameraBlockerRegistry(pub HashMap<TypeInfo, bool>);