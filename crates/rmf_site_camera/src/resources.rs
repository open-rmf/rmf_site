use std::{collections::HashMap, marker::PhantomData};

use bevy_asset::prelude::*;
use bevy_color::palettes::css as Colors;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyCode;
use bevy_math::{Vec3, primitives};
use bevy_pbr::StandardMaterial;
use bevy_reflect::Reflect;
use bevy_render::mesh::Mesh;
use bevy_utils::default;
use bytemuck::TransparentWrapper;

use crate::TypeInfo;

/// Material for camera's orbit marker
#[derive(Resource, Reflect)]
pub struct OrbitMarkerMaterial(pub Handle<StandardMaterial>);

impl FromWorld for OrbitMarkerMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut mats = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();

        Self(mats.add(StandardMaterial {
            base_color: Colors::GREEN.into(),
            emissive: Colors::LIME.into(),
            depth_bias: f32::MAX,
            unlit: true,
            ..default()
        }))
    }
}

/// Mesh for camera's pick marker
#[derive(Resource, Reflect)]
pub struct PickMarkerMesh(pub Handle<Mesh>);

impl FromWorld for PickMarkerMesh {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();

        Self(meshes.add(Mesh::from(primitives::Sphere::new(0.02))))
    }
}

/// Material for camera's pan marker mesh
#[derive(Resource, Reflect)]
pub struct PanMarkerMaterial(pub Handle<StandardMaterial>);

impl FromWorld for PanMarkerMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut mats = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        Self(mats.add(StandardMaterial {
            base_color: Colors::WHITE.into(),
            emissive: Colors::WHITE.into(),
            unlit: true,
            ..default()
        }))
    }
}

/// Camera(s)'s controls.
#[derive(Debug, Clone, Reflect, Resource)]
pub struct CameraControls {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub zoom_in: KeyCode,
    pub zoom_out: KeyCode,
}

impl Default for CameraControls {
    fn default() -> Self {
        Self {
            up: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            zoom_in: KeyCode::PageUp,
            zoom_out: KeyCode::PageDown,
        }
    }
}

/// Config settings for cameras.
#[derive(Debug, Clone, Reflect, Resource, Default)]
pub struct CameraConfig {
    pub orbit_center: Option<Vec3>,
}

/// Currently enabled camera.
#[derive(PartialEq, Debug, Copy, Clone, Reflect, Resource, Default)]
#[reflect(Resource)]
pub enum ProjectionMode {
    #[default]
    Perspective,
    Orthographic,
}

/// Block status for a given [`Registry`]. Managed by [`set_block_status`]
#[derive(Resource, TransparentWrapper, Default)]
#[transparent(bool)]
#[repr(transparent)]
pub struct BlockStatus<Registry>(bool, PhantomData<Registry>);

impl<Registry> BlockStatus<Registry> {
    /// Weather there was at least one block or not at the time of this resource's fetch.
    pub fn blocked(&self) -> bool {
        self.0
    }
}

/// Weather camera is controlable or not. enabled/disabled by [`CameraControlBlockers`]
pub type CameraControlBlocked = BlockStatus<CameraControlBlockers>;

/// Registry of things that can block camera controls
#[derive(Resource, Reflect, TransparentWrapper, Default)]
#[reflect(Resource)]
#[repr(transparent)]
pub struct CameraControlBlockers(pub HashMap<TypeInfo, bool>);
