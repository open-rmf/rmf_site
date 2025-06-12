use bevy_ecs::prelude::*;
use bevy_asset::prelude::*;
use bevy_math::primitives;
use bevy_pbr::StandardMaterial;
use bevy_reflect::Reflect;
use bevy_render::mesh::Mesh;
use bevy_color::palettes::css as Colors;
use bevy_utils::default;

#[derive(Resource)]
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

#[derive(Resource)]
pub struct CameraControlMesh(pub Handle<Mesh>);

impl FromWorld for CameraControlMesh {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();

        Self( meshes.add(Mesh::from(primitives::Sphere::new(0.02))))
    }
}

#[derive(Resource)]
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

#[derive(PartialEq, Debug, Copy, Clone, Reflect, Resource, Default)]
pub enum ProjectionMode {
    #[default]
    Perspective,
    Orthographic,
}